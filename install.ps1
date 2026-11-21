# Axon v1.0.0 Self-Hosted Multi-OS Native Toolchain Installer (Windows x64)
param([switch]$Uninstall)
$ErrorActionPreference = "Stop"

$AxonVersion = "1.0.0"
$AxonRoot = Join-Path $env:USERPROFILE ".axon"
$InstallBin = Join-Path $AxonRoot "bin"
$InstallStd = Join-Path $AxonRoot "std"
$AxcExe = Join-Path $InstallBin "axc.exe"

if ($Uninstall) {
    if (Test-Path -LiteralPath $AxonRoot) { Remove-Item -LiteralPath $AxonRoot -Recurse -Force }
    $UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    if ($UserPath) {
        $Norm = $InstallBin.TrimEnd('\').ToLowerInvariant()
        $Kept = @($UserPath -split ';' | Where-Object { $_.Trim().Length -gt 0 -and $_.Trim().TrimEnd('\').ToLowerInvariant() -ne $Norm })
        [Environment]::SetEnvironmentVariable("Path", ($Kept -join ';'), [EnvironmentVariableTarget]::User)
    }
    Write-Host "[+] Uninstalled Axon v$AxonVersion from $AxonRoot" -ForegroundColor Green
    exit 0
}

Write-Host "[*] Installing Axon v$AxonVersion Self-Hosted Native Toolchain..." -ForegroundColor Cyan

New-Item -ItemType Directory -Path $InstallBin -Force | Out-Null
New-Item -ItemType Directory -Path $InstallStd -Force | Out-Null

$CurrentAxc = Join-Path $PSScriptRoot "bin\axc.exe"
$ZipPkg = Join-Path $PSScriptRoot "dist\axon-v1.0.0-win-x64.zip"

if (Test-Path -LiteralPath $CurrentAxc) {
    Copy-Item -LiteralPath $CurrentAxc -Destination $AxcExe -Force
    Remove-Item -Path (Join-Path $InstallStd "*.ax") -Force -ErrorAction SilentlyContinue
    Copy-Item -Path (Join-Path $PSScriptRoot "std\*.ax") -Destination $InstallStd -Force
    foreach ($extra in @("AXON_LLM_SPEC.md", "axon_mcp.json")) {
        $p = Join-Path $PSScriptRoot $extra
        if (Test-Path -LiteralPath $p) { Copy-Item -LiteralPath $p -Destination $AxonRoot -Force }
    }
} elseif (Test-Path -LiteralPath $ZipPkg) {
    Write-Host "[*] Extracting toolchain package ($ZipPkg)..." -ForegroundColor Cyan
    Expand-Archive -LiteralPath $ZipPkg -DestinationPath $AxonRoot -Force
} else {
    throw "Neither bin\axc.exe nor distribution package found."
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($null -eq $UserPath) { $UserPath = "" }
$NormalizedBin = $InstallBin.TrimEnd('\').ToLowerInvariant()
$PathEntries = @($UserPath -split ';' | Where-Object { $_.Trim().Length -gt 0 })
$AlreadyInPath = ($PathEntries | Where-Object { $_.Trim().TrimEnd('\').ToLowerInvariant() -eq $NormalizedBin }).Count -gt 0
if (-not $AlreadyInPath) {
    $NewPath = if ($PathEntries.Count -gt 0) { "$InstallBin;$UserPath" } else { $InstallBin }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$InstallBin;$env:Path"
    Write-Host "[+] Added $InstallBin to User PATH." -ForegroundColor Green
}

$TotalBytes = (Get-ChildItem -Path $AxonRoot -Recurse -File | Measure-Object -Property Length -Sum).Sum
$TotalKB = [math]::Round($TotalBytes / 1KB, 1)
Write-Host "[+] Installed Axon v$AxonVersion ($TotalKB KB: 11-target compiler + linker + pkg + MCP + stdlib) to $AxonRoot" -ForegroundColor Green
& $AxcExe version
