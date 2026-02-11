# Sync assets (commands, skills, agents) from assets/ to .claude/ with {{GOVCTL}} substitution
# Usage: .\scripts\sync-assets.ps1

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# Placeholder and replacement
$Placeholder = "{{GOVCTL}}"
$Replacement = "cargo run --quiet --"

# Categories to sync: assets\<category>\ -> .claude\<category>\
$Categories = @("commands", "skills", "agents")

foreach ($category in $Categories) {
    $SrcDir = Join-Path $ProjectRoot "assets\$category"
    $OutDir = Join-Path $ProjectRoot ".claude\$category"

    # Skip if source directory has no .md files
    $templates = Get-ChildItem -Path $SrcDir -Filter "*.md" -ErrorAction SilentlyContinue
    if (-not $templates) {
        continue
    }

    if (-not (Test-Path $OutDir)) {
        New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
    }

    $templates | ForEach-Object {
        $template = $_.FullName
        $filename = $_.Name
        $output = Join-Path $OutDir $filename

        $content = Get-Content -Path $template -Raw
        $content = $content -replace [regex]::Escape($Placeholder), $Replacement
        Set-Content -Path $output -Value $content -NoNewline

        Write-Host "Synced: $output"
    }
}

Write-Host "Done. Assets synced to .claude/"
