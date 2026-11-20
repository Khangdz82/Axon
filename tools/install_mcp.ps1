# Axon MCP Server Auto-Installer for Claude Desktop & Cursor
# Usage: powershell -ExecutionPolicy Bypass -File tools/install_mcp.ps1 [-Target claude|cursor|all]

param(
    [ValidateSet("all", "claude", "cursor")]
    [string]$Target = "all"
)

$ErrorActionPreference = "Stop"

$ProjectRoot = (Get-Item -LiteralPath $PSScriptRoot).Parent.FullName
$AxcPath = Join-Path $ProjectRoot "bin\axc.exe"
if (-not (Test-Path -LiteralPath $AxcPath)) {
    $AxcPath = Join-Path $env:USERPROFILE ".axon\bin\axc.exe"
}
if (-not (Test-Path -LiteralPath $AxcPath)) {
    throw "axc.exe not found at $AxcPath or ~/.axon/bin/axc.exe"
}

$AxcPath = (Resolve-Path -LiteralPath $AxcPath).Path
Write-Host "[*] Found Axon Compiler at: $AxcPath" -ForegroundColor Cyan

function Register-McpConfig([string]$configPath, [string]$name) {
    $configDir = Split-Path -Parent $configPath
    if (-not (Test-Path -LiteralPath $configDir)) {
        New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    }

    $cfg = [PSCustomObject]@{}
    if (Test-Path -LiteralPath $configPath) {
        $raw = [System.IO.File]::ReadAllText($configPath, [System.Text.Encoding]::UTF8)
        if ($raw.Trim().Length -gt 0) {
            try {
                $parsed = $raw | ConvertFrom-Json
                if ($null -eq $parsed -or $parsed -isnot [PSCustomObject]) {
                    throw "Root JSON value is not an object."
                }
                $cfg = $parsed
            } catch {
                throw "Refusing to overwrite invalid JSON config at '$configPath': $($_.Exception.Message). Fix or back up the file manually first."
            }
        }
        $backupPath = "$configPath.bak"
        Copy-Item -LiteralPath $configPath -Destination $backupPath -Force
        Write-Host "[*] Backed up existing $name config to $backupPath" -ForegroundColor DarkGray
    }

    if (-not ($cfg.PSObject.Properties.Name -contains "mcpServers") -or $null -eq $cfg.mcpServers) {
        $cfg | Add-Member -NotePropertyName "mcpServers" -NotePropertyValue ([PSCustomObject]@{}) -Force
    } elseif ($cfg.mcpServers -isnot [PSCustomObject]) {
        throw "Refusing to overwrite '$configPath': 'mcpServers' is not a JSON object."
    }

    $axonEntry = [PSCustomObject]@{
        command = $AxcPath
        args = @("mcp")
    }
    $cfg.mcpServers | Add-Member -NotePropertyName "axon" -NotePropertyValue $axonEntry -Force

    $jsonStr = ($cfg | ConvertTo-Json -Depth 16) + "`n"
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($configPath, $jsonStr, $utf8NoBom)
    Write-Host "[+] Successfully registered Axon MCP Server in $name ($configPath)" -ForegroundColor Green
}

# 1. Claude Desktop
if ($Target -eq "all" -or $Target -eq "claude") {
    $claudeConfig = Join-Path $env:APPDATA "Claude\claude_desktop_config.json"
    Register-McpConfig $claudeConfig "Claude Desktop"
}

# 2. Cursor
if ($Target -eq "all" -or $Target -eq "cursor") {
    $cursorConfig = Join-Path $env:USERPROFILE ".cursor\mcp.json"
    Register-McpConfig $cursorConfig "Cursor"
}

Write-Host "[+] Setup complete! Axon's 17 MCP tools are now available in your AI environment." -ForegroundColor Green
