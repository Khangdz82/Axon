# Axon v1.0.0 Self-Hosted Multi-OS Native Toolchain Installer (Windows x64)
$ErrorActionPreference = "Stop"

$AxonVersion = "1.0.0"
$AxonRoot = Join-Path $env:USERPROFILE ".axon"
$InstallBin = Join-Path $AxonRoot "bin"
$InstallStd = Join-Path $AxonRoot "std"
$AxcExe = Join-Path $InstallBin "axc.exe"

Write-Host "[*] Installing Axon v$AxonVersion Self-Hosted Multi-OS Native Toolchain..." -ForegroundColor Cyan

New-Item -ItemType Directory -Path $InstallBin -Force | Out-Null
New-Item -ItemType Directory -Path $InstallStd -Force | Out-Null

$CurrentAxc = Join-Path $PSScriptRoot "bin\axc.exe"
$ZipPkg = Join-Path $PSScriptRoot "dist\axon-v1.0.0-win-x64.zip"

if (Test-Path -Path $CurrentAxc) {
    Copy-Item -Path $CurrentAxc -Destination $AxcExe -Force
    Copy-Item -Path (Join-Path $PSScriptRoot "std\*.ax") -Destination $InstallStd -Force
    if (Test-Path (Join-Path $PSScriptRoot "AXON_LLM_SPEC.md")) {
        Copy-Item -Path (Join-Path $PSScriptRoot "AXON_LLM_SPEC.md") -Destination $AxonRoot -Force
    }
} elseif (Test-Path -Path $ZipPkg) {
    Write-Host "[*] Extracting ultra-lightweight toolchain package ($ZipPkg)..." -ForegroundColor Cyan
    Expand-Archive -Path $ZipPkg -DestinationPath $AxonRoot -Force
} else {
    throw "Neither bin\axc.exe nor distribution package found."
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallBin*") {
    $NewPath = "$InstallBin;$UserPath"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$InstallBin;$env:Path"
    Write-Host "[+] Added $InstallBin to User PATH." -ForegroundColor Green
}

$TotalBytes = (Get-ChildItem -Path $AxonRoot -Recurse -File | Measure-Object -Property Length -Sum).Sum
$TotalKB = [math]::Round($TotalBytes / 1KB, 2)
Write-Host "[+] Installed Axon v$AxonVersion ($TotalKB KB installed footprint, ~440 KB compressed zip: 11-target compiler + linker + pkg + MCP + stdlib) to $AxonRoot" -ForegroundColor Green
& $AxcExe version
