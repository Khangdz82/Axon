# Axon v7.0 Self-Hosted Multi-OS Native Toolchain Installer (Windows x64)
$ErrorActionPreference = "Stop"

$AxonVersion = "7.0.0"
$AxonRoot = Join-Path $env:USERPROFILE ".axon"
$InstallBin = Join-Path $AxonRoot "bin"
$InstallStd = Join-Path $AxonRoot "std"
$AxcExe = Join-Path $InstallBin "axc.exe"

Write-Host "[*] Installing Axon v$AxonVersion Self-Hosted Multi-OS Native Toolchain..." -ForegroundColor Cyan

New-Item -ItemType Directory -Path $InstallBin -Force | Out-Null
New-Item -ItemType Directory -Path $InstallStd -Force | Out-Null

$CurrentAxc = Join-Path $PSScriptRoot "bin\axc.exe"
if (-not (Test-Path -Path $CurrentAxc)) {
    throw "bin\axc.exe not found."
}

Copy-Item -Path $CurrentAxc -Destination $AxcExe -Force
Copy-Item -Path (Join-Path $PSScriptRoot "std\*.ax") -Destination $InstallStd -Force
if (Test-Path (Join-Path $PSScriptRoot "AXON_LLM_SPEC.md")) {
    Copy-Item -Path (Join-Path $PSScriptRoot "AXON_LLM_SPEC.md") -Destination $AxonRoot -Force
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
Write-Host "[+] Installed Axon v$AxonVersion ($TotalKB KB total: 11-target compiler + linker + pkg + MCP + stdlib) to $AxonRoot" -ForegroundColor Green
& $AxcExe version
