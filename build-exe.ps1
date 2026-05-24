$ErrorActionPreference = "Stop"

Set-Location -LiteralPath $PSScriptRoot

Write-Host "Building Zapret2GUI portable package..." -ForegroundColor Cyan

python -m PyInstaller --clean --noconfirm .\Zapret2GUI.spec

$outDir = Join-Path $PSScriptRoot "dist\Zapret2GUI"
if (-not (Test-Path $outDir)) {
    throw "Build output not found: $outDir"
}

$docsSource = Join-Path $PSScriptRoot "docs"
$docsDestination = Join-Path $outDir "docs"
if (Test-Path -LiteralPath $docsSource) {
    if (Test-Path -LiteralPath $docsDestination) {
        Remove-Item -LiteralPath $docsDestination -Recurse -Force
    }
    Copy-Item -LiteralPath $docsSource -Destination $docsDestination -Recurse -Force
}

Write-Host ""
Write-Host "Build complete:" -ForegroundColor Green
Write-Host "  $outDir"
Write-Host ""
Write-Host "Run:" -ForegroundColor Yellow
Write-Host "  $outDir\Zapret2GUI.exe"
