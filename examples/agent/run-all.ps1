$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here "..\..")
$oldCli = $env:CHEMSEMA_CLI
$setCli = $false

if (-not $env:CHEMSEMA_CLI) {
  if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Push-Location $root
    try {
      & cargo build -p chemsema-cli
      if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
      }
    } finally {
      Pop-Location
    }
  }

  $candidates = @(
    (Join-Path $root "target\debug\chemsema-cli.exe"),
    (Join-Path $root "target\release\chemsema-cli.exe")
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      $env:CHEMSEMA_CLI = $candidate
      $setCli = $true
      break
    }
  }
}

if (-not $env:CHEMSEMA_CLI) {
  $env:CHEMSEMA_CLI = "chemsema-cli"
  $setCli = $true
}

$examples = @(
  "01-discover-targets",
  "02-context-crop",
  "03-edit-reaction-scheme",
  "04-session-workflow",
  "05-office-copy",
  "06-reaction-poc",
  "07-object-grounded-edit"
)

Push-Location $root
try {
  foreach ($example in $examples) {
    $script = Join-Path $here (Join-Path $example "one-shot.ps1")
    if (-not (Test-Path $script)) {
      $script = Join-Path $here (Join-Path $example "run.ps1")
    }
    if (-not (Test-Path $script)) {
      throw "No example entrypoint found for $example"
    }
    Write-Host "==> $example"
    & powershell -ExecutionPolicy Bypass -File $script
    if ($LASTEXITCODE -ne 0) {
      exit $LASTEXITCODE
    }
  }
  Write-Host "Agent examples completed."
} finally {
  Pop-Location
  if ($setCli) {
    if ($null -eq $oldCli) {
      Remove-Item Env:\CHEMSEMA_CLI -ErrorAction SilentlyContinue
    } else {
      $env:CHEMSEMA_CLI = $oldCli
    }
  }
}
