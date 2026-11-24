# Packaging script for Axon Ultra-Lightweight Toolchain Distribution
param([ValidateSet("win-x64", "linux-x64", "macos-arm64")][string]$Target = "win-x64")
$ErrorActionPreference = "Stop"

$ProjectRoot = (Get-Item -LiteralPath $PSScriptRoot).Parent.FullName
$DistDir = Join-Path $ProjectRoot "dist"
$TempStage = Join-Path $DistDir "stage_axon_$Target"
$ZipOut = Join-Path $DistDir "axon-v1.2.0-$Target.zip"

New-Item -ItemType Directory -Path $DistDir -Force | Out-Null
Remove-Item -LiteralPath $TempStage -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $ZipOut -Force -ErrorAction SilentlyContinue

try {
    New-Item -ItemType Directory -Path (Join-Path $TempStage "bin") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $TempStage "std") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $TempStage "tools") -Force | Out-Null

    $SrcBin = switch ($Target) {
        "win-x64"     { Join-Path $ProjectRoot "bin\axc.exe" }
        "linux-x64"   { Join-Path $ProjectRoot "bin\axc-linux-x64" }
        "macos-arm64" { Join-Path $ProjectRoot "bin\axc-macos-arm64" }
    }
    $DstBinName = if ($Target -eq "win-x64") { "axc.exe" } else { "axc" }
    Copy-Item -LiteralPath $SrcBin -Destination (Join-Path $TempStage "bin\$DstBinName") -Force
    Copy-Item -Path (Join-Path $ProjectRoot "std\*.ax") -Destination (Join-Path $TempStage "std") -Force

    foreach ($extra in @("LICENSE", "README.md", "AXON_LLM_SPEC.md", "axon_mcp.json", "install.ps1", "install.sh")) {
        $p = Join-Path $ProjectRoot $extra
        if (Test-Path -LiteralPath $p) { Copy-Item -LiteralPath $p -Destination $TempStage -Force }
    }
    $InstallMcp = Join-Path $ProjectRoot "tools\install_mcp.ps1"
    if (Test-Path -LiteralPath $InstallMcp) { Copy-Item -LiteralPath $InstallMcp -Destination (Join-Path $TempStage "tools\install_mcp.ps1") -Force }

    Compress-Archive -Path "$TempStage\*" -DestinationPath $ZipOut -CompressionLevel Optimal
    $ZipKB = [math]::Round((Get-Item -LiteralPath $ZipOut).Length / 1KB, 1)
    $Hash = (Get-FileHash -LiteralPath $ZipOut -Algorithm SHA256).Hash
    Write-Host "[+] Packaged $ZipOut ($ZipKB KB, SHA256=$Hash)" -ForegroundColor Green
} finally {
    Remove-Item -LiteralPath $TempStage -Recurse -Force -ErrorAction SilentlyContinue
}
