# Packaging script for Axon Ultra-Lightweight Toolchain Distribution
# Creates dist/axon-v1.0.0-win-x64.zip (< 500 KB total)

$ErrorActionPreference = "Stop"

$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName
$DistDir = Join-Path $ProjectRoot "dist"
$TempStage = Join-Path $DistDir "stage_axon"
$ZipOut = Join-Path $DistDir "axon-v1.0.0-win-x64.zip"

if (Test-Path $DistDir) {
    Remove-Item -Recurse -Force $DistDir
}
New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $TempStage "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $TempStage "std") -Force | Out-Null

Write-Host "[*] Staging Axon standalone binary and stdlib..." -ForegroundColor Cyan
Copy-Item (Join-Path $ProjectRoot "bin\axc.exe") (Join-Path $TempStage "bin\axc.exe") -Force
Copy-Item (Join-Path $ProjectRoot "std\*.ax") (Join-Path $TempStage "std") -Force

if (Test-Path (Join-Path $ProjectRoot "AXON_LLM_SPEC.md")) {
    Copy-Item (Join-Path $ProjectRoot "AXON_LLM_SPEC.md") $TempStage -Force
}
if (Test-Path (Join-Path $ProjectRoot "install.ps1")) {
    Copy-Item (Join-Path $ProjectRoot "install.ps1") $TempStage -Force
}

Write-Host "[*] Compressing ultra-lightweight distribution package..." -ForegroundColor Cyan
Compress-Archive -Path "$TempStage\*" -DestinationPath $ZipOut -CompressionLevel Optimal

$ZipBytes = (Get-Item $ZipOut).Length
$ZipKB = [math]::Round($ZipBytes / 1KB, 2)

Write-Host "[+] Created ${ZipOut} (${ZipKB} KB)" -ForegroundColor Green
if ($ZipBytes -lt 500 * 1024) {
    Write-Host "[+] SUCCESS: Toolchain package is under 500 KB ($ZipKB KB)!" -ForegroundColor Green
} else {
    Write-Host "[!] Warning: Package size is $ZipKB KB" -ForegroundColor Yellow
}

Remove-Item -Recurse -Force $TempStage
