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

    # Find all .md files (flat or in subdirectories)
    $templates = Get-ChildItem -Path $SrcDir -Filter "*.md" -Recurse -ErrorAction SilentlyContinue
    if (-not $templates) {
        continue
    }

    $templates | ForEach-Object {
        $template = $_.FullName
        $rel = $template.Substring($SrcDir.Length + 1)
        $output = Join-Path $OutDir $rel

        $outputDir = Split-Path -Parent $output
        if (-not (Test-Path $outputDir)) {
            New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
        }

        $content = Get-Content -Path $template -Raw
        $content = $content -replace [regex]::Escape($Placeholder), $Replacement
        Set-Content -Path $output -Value $content -NoNewline

        Write-Host "Synced: $output"
    }
}

Write-Host "Done. Assets synced to .claude/"
