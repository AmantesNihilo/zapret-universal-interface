param([string]$OutputDir = "", [switch]$AnalyzeOnly)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "crates\tg-ws-proxy-rs\Cargo.toml"
$output = if ($OutputDir) { [IO.Path]::GetFullPath($OutputDir) } else { Join-Path $root ".local-build-artifacts\tg-ws-h2" }
New-Item -ItemType Directory -Path $output -Force | Out-Null
if (-not $AnalyzeOnly) {
$messages = & cargo test --manifest-path $manifest --lib --no-run --offline --message-format=json
if ($LASTEXITCODE -ne 0) { throw "Failed to build the test executable" }
$executable = $messages | ForEach-Object { $_ | ConvertFrom-Json } |
    Where-Object { $_.reason -eq "compiler-artifact" -and $_.executable -and $_.profile.test } |
    Select-Object -Last 1 -ExpandProperty executable
if (-not $executable) { throw "Test executable was not found" }
$stdout = Join-Path $output "memory-test.stdout.txt"
$stderr = Join-Path $output "memory-test.stderr.txt"
$process = Start-Process -FilePath $executable -ArgumentList @(
    "h2_transport::tests::sustained_memory_sampling", "--exact", "--ignored", "--nocapture"
) -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$null = $process.Handle
$samples = [Collections.Generic.List[object]]::new()
$watch = [Diagnostics.Stopwatch]::StartNew()
while (-not $process.HasExited) {
    $process.Refresh()
    if ($process.HasExited) { break }
    $phase = Get-Content -LiteralPath $stdout -ErrorAction SilentlyContinue |
        Where-Object { $_ -like "phase=*" } | Select-Object -Last 1
    if (-not $phase) { $phase = "startup" }
    $samples.Add([pscustomobject]@{
        elapsedMs = $watch.ElapsedMilliseconds
        phase = $phase
        workingSetBytes = $process.WorkingSet64
        privateBytes = $process.PrivateMemorySize64
    })
    if ($watch.Elapsed.TotalSeconds -gt 60) { $process.Kill(); throw "Memory test exceeded 60 seconds" }
    Start-Sleep -Milliseconds 100
}
$process.WaitForExit()
$samples | Export-Csv -LiteralPath (Join-Path $output "memory-samples.csv") -NoTypeInformation -Encoding UTF8
Get-Content -LiteralPath $stdout
if (($null -ne $process.ExitCode -and $process.ExitCode -ne 0) -or
    -not (Select-String -LiteralPath $stdout -Pattern '^test result: ok\. 1 passed;' -Quiet)) {
    Get-Content -LiteralPath $stderr
    throw "Memory test failed: $($process.ExitCode)"
}
} else {
    $samples = Import-Csv -LiteralPath (Join-Path $output "memory-samples.csv")
}
$groups = @{}
foreach ($sample in $samples) {
    if ([double]$sample.privateBytes -le 0) { continue }
    $key = if ($sample.phase) { [string]$sample.phase } else { "startup" }
    if (-not $groups.ContainsKey($key)) { $groups[$key] = [Collections.Generic.List[object]]::new() }
    $groups[$key].Add($sample)
}
$summary = foreach ($key in ($groups.Keys | Sort-Object)) {
    $values = $groups[$key]
    [pscustomobject]@{
        phase = $key
        peakPrivateMiB = [Math]::Round(($values | Measure-Object -Property privateBytes -Maximum).Maximum / 1MB, 2)
        lastPrivateMiB = [Math]::Round([double]$values[-1].privateBytes / 1MB, 2)
        peakWorkingSetMiB = [Math]::Round(($values | Measure-Object -Property workingSetBytes -Maximum).Maximum / 1MB, 2)
    }
}
$summary | Format-Table -AutoSize
$summary | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output "memory-summary.json") -Encoding UTF8
