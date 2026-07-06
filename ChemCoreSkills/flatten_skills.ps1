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
    Write-Host "flattened $name -> $target"
}
