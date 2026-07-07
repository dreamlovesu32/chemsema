param(
    [switch]$Verify,
    [switch]$Cargo,
    [switch]$Cli,
    [switch]$All,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    @"
Usage:
  powershell -ExecutionPolicy Bypass -File chemcore_check.ps1 [-Verify] [-Cargo] [-Cli] [-All]

Options:
  -Verify   Run npm run verify.
  -Cargo    Run cargo test --workspace.
  -Cli      Run chemcore-cli version and doctor through cargo.
  -All      Run all checks.
  -Help     Show this message.

When no check option is provided, the script runs the same checks as -All.
"@
    exit 0
}

if ($All -or (-not $Verify -and -not $Cargo -and -not $Cli)) {
    $Verify = $true
    $Cargo = $true
    $Cli = $true
}

if ($Verify) {
    npm run verify
}

if ($Cargo) {
    cargo test --workspace
}

if ($Cli) {
    cargo run -p chemcore-cli -- version --pretty
    cargo run -p chemcore-cli -- doctor --pretty
}
