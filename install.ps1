# Axon v6.0 Ultra-Lightweight Self-Hosted Toolchain Installer (Windows x64)
$ErrorActionPreference = "Stop"

$AxonVersion = "6.0.0"
$AxonRoot = Join-Path $env:USERPROFILE ".axon"
$InstallBin = Join-Path $AxonRoot "bin"
$InstallStd = Join-Path $AxonRoot "std"
$AxcExe = Join-Path $InstallBin "axc.exe"

Write-Host "[*] Installing Axon v$AxonVersion Self-Hosted Native Toolchain..." -ForegroundColor Cyan

New-Item -ItemType Directory -Path $InstallBin -Force | Out-Null
New-Item -ItemType Directory -Path $InstallStd -Force | Out-Null

$CurrentAxc = Join-Path $PSScriptRoot "bin\axc.exe"
if (-not (Test-Path -Path $CurrentAxc)) {
    throw "bin\axc.exe not found. Bootstrap Stage-0 first via bootstrap_archive/axc_native."
}

Copy-Item -Path $CurrentAxc -Destination $AxcExe -Force
Copy-Item -Path (Join-Path $PSScriptRoot "std\*.ax") -Destination $InstallStd -Force

$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallBin*") {
    $NewPath = "$InstallBin;$UserPath"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$InstallBin;$env:Path"
    Write-Host "[+] Added $InstallBin to User PATH." -ForegroundColor Green
}

$TotalBytes = (Get-ChildItem -Path $AxonRoot -Recurse -File | Measure-Object -Property Length -Sum).Sum
$TotalKB = [math]::Round($TotalBytes / 1KB, 2)
Write-Host "[+] Installed Axon v$AxonVersion ($TotalKB KB total: compiler + linker + stdlib) to $AxonRoot" -ForegroundColor Green
& $AxcExe version
