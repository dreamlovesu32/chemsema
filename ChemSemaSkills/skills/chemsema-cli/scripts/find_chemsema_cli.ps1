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

function Get-PlatformTag {
    $arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
        "X64" { "x64" }
        "Arm64" { "arm64" }
        default { $_.ToString().ToLowerInvariant() }
    }
    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        return "win-$arch"
    }
    if ($IsMacOS) {
        return "macos-$arch"
    }
    if ($IsLinux) {
        return "linux-$arch"
    }
    return "unknown-$arch"
}

function Get-ExecutableName {
    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        return "chemsema-cli.exe"
    }
    return "chemsema-cli"
}

$candidates = @()
$sources = @{}
if ($env:CHEMSEMA_CLI) {
    $candidates += $env:CHEMSEMA_CLI
    $sources[$env:CHEMSEMA_CLI] = "CHEMSEMA_CLI"
}

$pathCommand = Get-Command chemsema-cli -ErrorAction SilentlyContinue
if ($pathCommand) {
    $candidates += $pathCommand.Source
    $sources[$pathCommand.Source] = "PATH"
}

$skillRoot = Split-Path -Parent $PSScriptRoot
$platformTag = Get-PlatformTag
$manifestPath = Join-Path $skillRoot "assets\runtime-manifest.json"
if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    $entry = $manifest.platforms.$platformTag
    if ($entry -and $entry.path) {
        $candidate = Join-Path (Join-Path $skillRoot "assets") $entry.path
        $candidates += $candidate
        $sources[$candidate] = "bundled:$platformTag"
    }
}
$bundledDefault = Join-Path $skillRoot (Join-Path "assets\bin\$platformTag" (Get-ExecutableName))
if (-not ($candidates -contains $bundledDefault)) {
    $candidates += $bundledDefault
    $sources[$bundledDefault] = "bundled:$platformTag"
}

foreach ($candidate in $candidates) {
    if (Test-ExecutablePath $candidate) {
        $resolved = (Resolve-Path -LiteralPath $candidate).Path
        if ($Json) {
            [pscustomobject]@{
                ok = $true
                path = $resolved
                source = $sources[$candidate]
                platform = $platformTag
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
        source = $null
        platform = $platformTag
        message = "chemsema-cli was not found. Install the self-contained ChemSema CLI skill, install ChemSema CLI on PATH, or set CHEMSEMA_CLI. Source checkout builds are handled by the chemsema-development skill."
    } | ConvertTo-Json -Depth 4
} else {
    Write-Error "chemsema-cli was not found. Install the self-contained ChemSema CLI skill, install ChemSema CLI on PATH, or set CHEMSEMA_CLI. Source checkout builds are handled by the chemsema-development skill."
}
exit 1
