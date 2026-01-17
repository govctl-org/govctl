# Sync Claude commands from assets/ with cargo run substitution
# Usage: .\scripts\sync-commands.ps1

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

$AssetsDir = Join-Path $ProjectRoot "assets"
$OutputDir = Join-Path $ProjectRoot ".claude\commands"

# Placeholder and replacement
$Placeholder = "{{GOVCTL}}"
$Replacement = "cargo run --quiet --"

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# Process each template
Get-ChildItem -Path $AssetsDir -Filter "*.md" | ForEach-Object {
    $template = $_.FullName
    $filename = $_.Name
    $output = Join-Path $OutputDir $filename
    
    $content = Get-Content -Path $template -Raw
    $content = $content -replace [regex]::Escape($Placeholder), $Replacement
    Set-Content -Path $output -Value $content -NoNewline
    
    Write-Host "Synced: $output"
}

Write-Host "Done. Claude commands synced to $OutputDir"
