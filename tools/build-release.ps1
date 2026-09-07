param(
    [string]$Version = "2.2.0",
    [string]$TargetDir = "",
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$release = if ($OutputDir) { [IO.Path]::GetFullPath($OutputDir) } else { Join-Path $root "release" }
$target = if ($TargetDir) { [IO.Path]::GetFullPath($TargetDir) } else { Join-Path $root "src-tauri\target\release" }
$stage = Join-Path $release ".portable-stage-$Version"
$portable = Join-Path $release "ZUI_${Version}_portable"
$portableZip = Join-Path $release "ZUI_${Version}_portable.zip"
$tempZip = Join-Path $release ".portable.tmp.zip"

New-Item -ItemType Directory -Path $release -Force | Out-Null

foreach ($path in @($stage, $portable)) {
    $resolvedParent = [IO.Path]::GetFullPath((Split-Path -Parent $path))
    if ($resolvedParent -ne [IO.Path]::GetFullPath($release)) {
        throw "Unsafe release path: $path"
    }
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}

foreach ($path in @($tempZip, $portableZip)) {
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Force
    }
}

New-Item -ItemType Directory -Path $stage | Out-Null
Copy-Item -LiteralPath (Join-Path $target "zui.exe") -Destination (Join-Path $stage "ZUI.exe")
$stageResources = Join-Path $stage "resources"
New-Item -ItemType Directory -Path $stageResources | Out-Null
Copy-Item -LiteralPath (Join-Path $root "resources\zapret") -Destination (Join-Path $stageResources "zapret") -Recurse
Copy-Item -LiteralPath (Join-Path $root "resources\zapret2") -Destination (Join-Path $stageResources "zapret2") -Recurse
New-Item -ItemType File -Path (Join-Path $stage "portable.flag") | Out-Null

$data = Join-Path $stage "data"
New-Item -ItemType Directory -Path $data | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
$secretBytes = New-Object byte[] 16
$secretRng = [Security.Cryptography.RandomNumberGenerator]::Create()
try {
    $secretRng.GetBytes($secretBytes)
} finally {
    $secretRng.Dispose()
}
$defaultTgSecret = ($secretBytes | ForEach-Object { $_.ToString('x2') }) -join ''

$profiles = @{
    activeProfileId = "default"
    profiles = @(
        @{
            id = "default"
            name = "Default"
            zapretEnabled = $false
            zapretEngine = $null
            zapretPresetId = $null
            tgWsEnabled = $false
            tgWsHost = "127.0.0.1"
            tgWsPort = 1443
            tgWsSecret = $defaultTgSecret
            tgWsDcIps = @("2:149.154.167.220", "4:149.154.167.220")
            tgWsCfProxyEnabled = $true
            tgWsCfCustomEnabled = $false
            tgWsDefaultDomains = $true
            tgWsCfDomains = @()
            tgWsCfWorkerEnabled = $false
            tgWsCfWorkerDomain = $null
            tgWsFrontingDomain = "sprinthost.ru"
            tgWsCfPriority = $false
            tgWsCfBalance = $false
            tgWsBufKb = 256
            tgWsPoolSize = 4
            tgWsVerbose = $false
            tgWsLogMaxMb = 5.0
            tgWsForceTestDc = $false
            autostartOnAppLaunch = $false
            notes = $null
        }
    )
}

$settings = @{
    theme = "dark"
    accent = "cyan"
    language = "ru"
    layoutOrientation = "portrait"
    launchMinimized = $false
    closeToTray = $false
    minimizeBehavior = "taskbar"
    minimizeDontAsk = $false
    startWithWindows = $false
    startWithWindowsInTray = $true
    autoStartActiveProfileOnLaunch = $false
    checkUpdatesOnLaunch = $true
    customPresetRoots = @()
    testTargets = @()
}

[IO.File]::WriteAllText(
    (Join-Path $data "profiles.json"),
    ($profiles | ConvertTo-Json -Depth 5),
    $utf8
)
[IO.File]::WriteAllText(
    (Join-Path $data "settings.json"),
    ($settings | ConvertTo-Json -Depth 5),
    $utf8
)
[IO.File]::WriteAllText((Join-Path $data "test-results.json"), "[]", $utf8)
[IO.File]::WriteAllText((Join-Path $data "preset-preferences.json"), "{}", $utf8)

$nsisSource = Join-Path $target "bundle\nsis\ZUI_${Version}_x64-setup.exe"
$msiSource = Join-Path $target "bundle\msi\ZUI_${Version}_x64_en-US.msi"
$nsisRelease = Join-Path $release "ZUI_${Version}_x64-setup.exe"
$msiRelease = Join-Path $release "ZUI_${Version}_x64_en-US.msi"

Copy-Item -LiteralPath $nsisSource -Destination $nsisRelease -Force
Copy-Item -LiteralPath $msiSource -Destination $msiRelease -Force
Move-Item -LiteralPath $stage -Destination $portable
Compress-Archive -Path (Join-Path $portable "*") -DestinationPath $tempZip -CompressionLevel Optimal
Move-Item -LiteralPath $tempZip -Destination $portableZip

$artifacts = @($nsisRelease, $msiRelease, $portableZip)
$hashLines = Get-FileHash $artifacts -Algorithm SHA256 | ForEach-Object {
    "$($_.Hash.ToLower())  $([IO.Path]::GetFileName($_.Path))"
}
[IO.File]::WriteAllLines((Join-Path $release "SHA256SUMS.txt"), $hashLines, [Text.Encoding]::ASCII)

Get-Item $artifacts | Select-Object Name, Length, LastWriteTime
