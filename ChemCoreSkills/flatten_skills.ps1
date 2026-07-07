param(
    [Parameter(Mandatory = $true)]
    [string]$OutDir,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$out = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutDir)

if ($Clean -and (Test-Path -LiteralPath $out)) {
    Remove-Item -LiteralPath $out -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $out | Out-Null

function Remove-GeneratedSkillFiles {
    param([Parameter(Mandatory = $true)][string]$Path)

    Get-ChildItem -LiteralPath $Path -Recurse -Force -Directory | Where-Object {
        $_.Name -in @("__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache")
    } | Remove-Item -Recurse -Force

    Get-ChildItem -LiteralPath $Path -Recurse -Force -File | Where-Object {
        $_.Name -like "*.pyc" -or
        $_.Name -like "*.pyo" -or
        $_.Name -like "*.log" -or
        $_.Name -like "*.tmp" -or
        $_.Name -like "*.bak" -or
        $_.Name -like "*.orig" -or
        $_.Name -like "*.rej" -or
        $_.Name -like "*~"
    } | Remove-Item -Force
}

$skillFiles = Get-ChildItem -LiteralPath $root -Recurse -Filter SKILL.md | Where-Object {
    $fullName = $_.FullName
    -not $fullName.StartsWith($out, [System.StringComparison]::OrdinalIgnoreCase)
}
foreach ($skillFile in $skillFiles) {
    $skillDir = $skillFile.Directory
    $name = $skillDir.Name
    $target = Join-Path $out $name
    if (Test-Path -LiteralPath $target) {
        Remove-Item -LiteralPath $target -Recurse -Force
    }
    Copy-Item -LiteralPath $skillDir.FullName -Destination $target -Recurse
    Remove-GeneratedSkillFiles -Path $target
    Write-Host "flattened $name -> $target"
}
