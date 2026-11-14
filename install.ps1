# Axon v5.0 Standalone Toolchain Installer (Windows x64)
$ErrorActionPreference = "Stop"

$AxonVersion = "5.0.0"
$InstallDir = Join-Path $env:USERPROFILE ".axon\bin"
$AxcExe = Join-Path $InstallDir "axc.exe"

Write-Host "[*] Installing Axon v$AxonVersion Self-Hosted Native Compiler..." -ForegroundColor Cyan

if (-not (Test-Path -Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$CurrentAxc = Join-Path $PSScriptRoot "bin\axc.exe"
if (-not (Test-Path -Path $CurrentAxc)) {
    throw "bin\axc.exe not found. Build Stage-0 first via cargo build --release --manifest-path bootstrap_archive/axc_native/Cargo.toml"
}

Copy-Item -Path $CurrentAxc -Destination $AxcExe -Force

$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallDir*") {
    $NewPath = "$InstallDir;$UserPath"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$InstallDir;$env:Path"
    Write-Host "[+] Added $InstallDir to User PATH." -ForegroundColor Green
}

$AxcSizeKB = [math]::Round((Get-Item $AxcExe).Length / 1KB, 2)
Write-Host "[+] Installed axc.exe ($AxcSizeKB KB) to $AxcExe" -ForegroundColor Green
& $AxcExe version
