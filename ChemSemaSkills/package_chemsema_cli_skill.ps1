param(
    [string]$OutDir = "dist\chemsema-skills",
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $root
$skillSource = Join-Path $root "skills\chemsema-cli"
$manifestPath = Join-Path $skillSource "assets\runtime-manifest.json"

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Missing runtime manifest: $manifestPath"
}

$out = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutDir)
$stagingRoot = Join-Path $out "_staging"
$packageSkillDir = Join-Path $stagingRoot "chemsema-cli"

function Assert-PathInside {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Parent
    )
    $resolvedParent = [System.IO.Path]::GetFullPath($Parent)
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    $prefix = $resolvedParent.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolvedPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to operate outside output directory: $resolvedPath"
    }
}

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

function Test-RuntimeAssets {
    param([Parameter(Mandatory = $true)][string]$SkillDir)

    $manifest = Get-Content -LiteralPath (Join-Path $SkillDir "assets\runtime-manifest.json") -Raw | ConvertFrom-Json
    foreach ($platform in $manifest.platforms.PSObject.Properties) {
        $entry = $platform.Value
        $asset = Join-Path (Join-Path $SkillDir "assets") $entry.path
        if (-not (Test-Path -LiteralPath $asset -PathType Leaf)) {
            throw "Missing runtime asset for $($platform.Name): $asset"
        }

        $file = Get-Item -LiteralPath $asset
        if ($entry.size -and $file.Length -ne [int64]$entry.size) {
            throw "Size mismatch for $($platform.Name): expected $($entry.size), got $($file.Length)"
        }

        if ($entry.sha256) {
            $hash = (Get-FileHash -LiteralPath $asset -Algorithm SHA256).Hash
            if ($hash -ne $entry.sha256) {
                throw "SHA256 mismatch for $($platform.Name): expected $($entry.sha256), got $hash"
            }
        }
    }
    return $manifest
}

New-Item -ItemType Directory -Force -Path $out | Out-Null
Assert-PathInside -Path $stagingRoot -Parent $out
if ($Clean -and (Test-Path -LiteralPath $stagingRoot)) {
    Remove-Item -LiteralPath $stagingRoot -Recurse -Force
}
if (Test-Path -LiteralPath $stagingRoot) {
    Remove-Item -LiteralPath $stagingRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null

Copy-Item -LiteralPath $skillSource -Destination $packageSkillDir -Recurse
Remove-GeneratedSkillFiles -Path $packageSkillDir

$manifest = Test-RuntimeAssets -SkillDir $packageSkillDir
$platforms = ($manifest.platforms.PSObject.Properties | ForEach-Object { $_.Name }) -join "+"
$version = $manifest.version
$zipName = "chemsema-cli-skill-$version-$platforms-unsigned.zip"
$zipPath = Join-Path $out $zipName
$checksumsPath = Join-Path $out "SHA256SUMS.txt"
$noticePath = Join-Path $out "UNSIGNED-RUNTIME.txt"

if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath $packageSkillDir -DestinationPath $zipPath -CompressionLevel Optimal

$zipHash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
"$zipHash  $zipName" | Set-Content -LiteralPath $checksumsPath -Encoding UTF8

@"
ChemSema CLI Skill Runtime Notice

This package includes an unsigned prebuilt ChemSema CLI runtime. Verify the
package with SHA256SUMS.txt before installing it. The runtime assets are also
listed in chemsema-cli/assets/runtime-manifest.json with size and SHA256 values.

If you do not want to run the bundled runtime, install a trusted ChemSema CLI
separately and set CHEMSEMA_CLI to its executable path.
"@ | Set-Content -LiteralPath $noticePath -Encoding UTF8

[pscustomobject]@{
    ok = $true
    package = $zipPath
    sha256 = $zipHash
    checksums = $checksumsPath
    notice = $noticePath
    platforms = $platforms
    version = $version
    repoRoot = $repoRoot
} | ConvertTo-Json -Depth 4
