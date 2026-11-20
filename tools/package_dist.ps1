# Packaging script for Axon Ultra-Lightweight Toolchain Distribution
# Creates dist/axon-v1.0.0-win-x64.zip (< 500 KB total)

$ErrorActionPreference = "Stop"

$ProjectRoot = (Get-Item -LiteralPath $PSScriptRoot).Parent.FullName
$DistDir = Join-Path $ProjectRoot "dist"
$TempStage = Join-Path $DistDir "stage_axon"
$ZipOut = Join-Path $DistDir "axon-v1.0.0-win-x64.zip"

if (-not (Test-Path -LiteralPath $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
}
if (Test-Path -LiteralPath $TempStage) {
    Remove-Item -LiteralPath $TempStage -Recurse -Force
}
if (Test-Path -LiteralPath $ZipOut) {
    Remove-Item -LiteralPath $ZipOut -Force
}

try {
    New-Item -ItemType Directory -Path (Join-Path $TempStage "bin") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $TempStage "std") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $TempStage "tools") -Force | Out-Null

    Write-Host "[*] Staging Axon standalone binary and stdlib..." -ForegroundColor Cyan
    Copy-Item -LiteralPath (Join-Path $ProjectRoot "bin\axc.exe") -Destination (Join-Path $TempStage "bin\axc.exe") -Force
    Get-ChildItem -LiteralPath (Join-Path $ProjectRoot "std") -Filter "*.ax" -File | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $TempStage "std") -Force
    }

    $SpecFile = Join-Path $ProjectRoot "AXON_LLM_SPEC.md"
    if (Test-Path -LiteralPath $SpecFile) {
        Copy-Item -LiteralPath $SpecFile -Destination $TempStage -Force
    }
    $InstallPs1 = Join-Path $ProjectRoot "install.ps1"
    if (Test-Path -LiteralPath $InstallPs1) {
        Copy-Item -LiteralPath $InstallPs1 -Destination $TempStage -Force
    }
    $McpJson = Join-Path $ProjectRoot "axon_mcp.json"
    if (Test-Path -LiteralPath $McpJson) {
        Copy-Item -LiteralPath $McpJson -Destination $TempStage -Force
    }
    $InstallMcp = Join-Path $ProjectRoot "tools\install_mcp.ps1"
    if (Test-Path -LiteralPath $InstallMcp) {
        Copy-Item -LiteralPath $InstallMcp -Destination (Join-Path $TempStage "tools\install_mcp.ps1") -Force
    }

    Write-Host "[*] Compressing ultra-lightweight distribution package..." -ForegroundColor Cyan
    Compress-Archive -Path "$TempStage\*" -DestinationPath $ZipOut -CompressionLevel Optimal

    $ZipBytes = (Get-Item -LiteralPath $ZipOut).Length
    $ZipKB = [math]::Round($ZipBytes / 1KB, 2)

    Write-Host "[+] Created ${ZipOut} (${ZipKB} KB)" -ForegroundColor Green
    if ($ZipBytes -lt 500 * 1024) {
        Write-Host "[+] SUCCESS: Toolchain package is under 500 KB ($ZipKB KB)!" -ForegroundColor Green
    } else {
        Write-Host "[!] Warning: Package size is $ZipKB KB" -ForegroundColor Yellow
    }
} finally {
    if (Test-Path -LiteralPath $TempStage) {
        Remove-Item -LiteralPath $TempStage -Recurse -Force
    }
}
