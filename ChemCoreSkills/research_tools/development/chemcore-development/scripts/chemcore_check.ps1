param(
    [switch]$Verify,
    [switch]$Cargo,
    [switch]$Cli,
    [switch]$All
)

$ErrorActionPreference = "Stop"

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
