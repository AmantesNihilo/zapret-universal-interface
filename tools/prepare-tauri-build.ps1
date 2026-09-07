$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$targetRoot = if ($env:CARGO_TARGET_DIR) {
    $configuredTarget = if ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        $env:CARGO_TARGET_DIR
    } else {
        Join-Path $root $env:CARGO_TARGET_DIR
    }
    [IO.Path]::GetFullPath($configuredTarget)
} else {
    [IO.Path]::GetFullPath((Join-Path $root "src-tauri\target"))
}

foreach ($profile in @("debug", "release")) {
    $resourceDir = [IO.Path]::GetFullPath((Join-Path $targetRoot "$profile\resources"))
    $expectedParent = [IO.Path]::GetFullPath((Join-Path $targetRoot $profile))
    if ([IO.Path]::GetDirectoryName($resourceDir) -ne $expectedParent -or
        [IO.Path]::GetFileName($resourceDir) -ne "resources") {
        throw "Unsafe Tauri resource staging path: $resourceDir"
    }
    if (Test-Path -LiteralPath $resourceDir) {
        Remove-Item -LiteralPath $resourceDir -Recurse -Force
    }
}
