$ErrorActionPreference = "Stop"
$cli = if ($env:CHEMSEMA_CLI) { $env:CHEMSEMA_CLI } else { "chemsema-cli" }
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $here
try {
  & $cli targets ..\..\..\figure1.cdxml --out targets.json --pretty
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
