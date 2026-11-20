# Axon MCP Server Auto-Installer for Claude Desktop & Cursor
# Usage: powershell -ExecutionPolicy Bypass -File tools/install_mcp.ps1 [-Target claude|cursor|all]

param(
    [string]$Target = "all"
)

$ErrorActionPreference = "Stop"

$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName
$AxcPath = Join-Path $ProjectRoot "bin\axc.exe"
if (-not (Test-Path $AxcPath)) {
    $AxcPath = Join-Path $env:USERPROFILE ".axon\bin\axc.exe"
}
if (-not (Test-Path $AxcPath)) {
    throw "axc.exe not found at $AxcPath or ~/.axon/bin/axc.exe"
}

$AxcPath = (Resolve-Path $AxcPath).Path
Write-Host "[*] Found Axon Compiler at: $AxcPath" -ForegroundColor Cyan

function Register-McpConfig($configPath, $name) {
    $configDir = Split-Path -Parent $configPath
    if (-not (Test-Path $configDir)) {
        New-Item -ItemType Directory -Path $configDir -Force | Out-Null
    }

    $cfg = @{}
    if (Test-Path $configPath) {
        try {
            $raw = Get-Content $configPath -Raw
            if ($raw.Trim().Length -gt 0) {
                $cfg = $raw | ConvertFrom-Json -AsHashtable
            }
        } catch {
            $cfg = @{}
        }
    }

    if (-not $cfg.ContainsKey("mcpServers")) {
        $cfg["mcpServers"] = @{}
    }

    $cfg["mcpServers"]["axon"] = @{
        command = $AxcPath
        args = @("mcp")
    }

    $jsonStr = $cfg | ConvertTo-Json -Depth 10
    Set-Content -Path $configPath -Value $jsonStr -Force
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
