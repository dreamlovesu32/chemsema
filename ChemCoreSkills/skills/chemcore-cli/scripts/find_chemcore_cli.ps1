param(
    [switch]$Json
)

$ErrorActionPreference = "Stop"

function Test-ExecutablePath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $false
    }
    return (Test-Path -LiteralPath $Path -PathType Leaf)
}

function Find-RepoRootFrom {
    param([string]$Start)
    $current = (Resolve-Path -LiteralPath $Start).Path
    while ($true) {
        if ((Test-Path -LiteralPath (Join-Path $current "Cargo.toml")) -and
            (Test-Path -LiteralPath (Join-Path $current "package.json"))) {
            return $current
        }
        $parent = Split-Path -Parent $current
        if ($parent -eq $current -or [string]::IsNullOrEmpty($parent)) {
            return $null
        }
        $current = $parent
    }
}

$candidates = @()
if ($env:CHEMCORE_CLI) {
    $candidates += $env:CHEMCORE_CLI
}

$pathCommand = Get-Command chemcore-cli -ErrorAction SilentlyContinue
if ($pathCommand) {
    $candidates += $pathCommand.Source
}

$repoRoot = Find-RepoRootFrom $PSScriptRoot
if (-not $repoRoot) {
    $repoRoot = Find-RepoRootFrom (Get-Location).Path
}
if ($repoRoot) {
    $candidates += (Join-Path $repoRoot "target\release\chemcore-cli.exe")
    $candidates += (Join-Path $repoRoot "target\debug\chemcore-cli.exe")
}

foreach ($candidate in $candidates) {
    if (Test-ExecutablePath $candidate) {
        $resolved = (Resolve-Path -LiteralPath $candidate).Path
        if ($Json) {
            [pscustomobject]@{
                ok = $true
                path = $resolved
                repoRoot = $repoRoot
            } | ConvertTo-Json -Depth 4
        } else {
            $resolved
        }
        exit 0
    }
}

if ($Json) {
    [pscustomobject]@{
        ok = $false
        path = $null
        repoRoot = $repoRoot
        message = "chemcore-cli.exe was not found. Build it or set CHEMCORE_CLI."
    } | ConvertTo-Json -Depth 4
} else {
    Write-Error "chemcore-cli.exe was not found. Build it or set CHEMCORE_CLI."
}
exit 1
