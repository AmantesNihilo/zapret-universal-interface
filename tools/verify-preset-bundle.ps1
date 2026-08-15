param(
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$zapret2 = Join-Path $ProjectRoot 'resources\zapret2'
$manifest = Get-Content (Join-Path $zapret2 'manifest.json') -Raw | ConvertFrom-Json
$presets = @(Get-ChildItem (Join-Path $zapret2 'presets') -File -Filter '*.txt')
$binFiles = @(Get-ChildItem (Join-Path $zapret2 'bin') -File)

Assert-True ($presets.Count -eq [int]$manifest.presetCount) "Zapret 2 preset count mismatch: disk=$($presets.Count), manifest=$($manifest.presetCount)"
Assert-True ($binFiles.Count -eq [int]$manifest.bundledFakeFileCount) "Zapret 2 bin count mismatch: disk=$($binFiles.Count), manifest=$($manifest.bundledFakeFileCount)"

$references = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($preset in $presets) {
    $text = Get-Content $preset.FullName -Raw
    foreach ($match in [regex]::Matches($text, '(?i)(?:@)?((?:bin|lua|lists|windivert\.filter)/[^\s,"\r\n]+)')) {
        [void]$references.Add($match.Groups[1].Value.TrimEnd(';', ':'))
    }
}

$missing = @($references | Where-Object {
    -not (Test-Path -LiteralPath (Join-Path $zapret2 ($_.Replace('/', '\'))))
})
Assert-True ($missing.Count -eq 0) "Missing Zapret 2 resources: $($missing -join ', ')"

$flowseal = Join-Path $ProjectRoot 'resources\zapret\flowseal\1.10.1'
$flowManifest = Get-Content (Join-Path $flowseal 'zui-manifest.json') -Raw | ConvertFrom-Json
$flowPresets = @(Get-ChildItem $flowseal -File -Filter 'general*.bat')
Assert-True ($flowPresets.Count -eq [int]$flowManifest.presetCount) "Flowseal preset count mismatch: disk=$($flowPresets.Count), manifest=$($flowManifest.presetCount)"

foreach ($userFile in $flowManifest.preservedUserFiles) {
    Assert-True (Test-Path -LiteralPath (Join-Path $flowseal $userFile)) "Missing preserved Flowseal file: $userFile"
}

$flowMissing = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($preset in $flowPresets) {
    $text = Get-Content $preset.FullName -Raw
    foreach ($match in [regex]::Matches($text, '(?i)%BIN%([^"%\s\^]+)')) {
        $relative = 'bin\' + $match.Groups[1].Value.TrimEnd(';', ':')
        if (-not (Test-Path -LiteralPath (Join-Path $flowseal $relative))) { [void]$flowMissing.Add($relative) }
    }
    foreach ($match in [regex]::Matches($text, '(?i)%LISTS%([^"%\s\^]+)')) {
        $relative = 'lists\' + $match.Groups[1].Value.TrimEnd(';', ':')
        if (-not (Test-Path -LiteralPath (Join-Path $flowseal $relative))) { [void]$flowMissing.Add($relative) }
    }
}
Assert-True ($flowMissing.Count -eq 0) "Missing Flowseal resources: $($flowMissing -join ', ')"

Write-Output "PASS Zapret 2: $($presets.Count) presets, $($references.Count) referenced resources, $($binFiles.Count) bin files"
Write-Output "PASS Flowseal: $($flowPresets.Count) presets, preserved user lists present"
