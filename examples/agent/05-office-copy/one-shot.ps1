$ErrorActionPreference = "Stop"
$cli = if ($env:CHEMSEMA_CLI) { $env:CHEMSEMA_CLI } else { "chemsema-cli" }
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $here
try {
  $copyJson = & $cli copy ..\..\..\figure1.cdxml `
    --target object:obj_bracket_001 `
    --payload payload.json `
    --no-copy `
    --pretty
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $copyJson | Set-Content -Path copy-result.json -Encoding UTF8
} finally {
  Pop-Location
}
