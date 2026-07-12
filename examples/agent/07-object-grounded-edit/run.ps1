$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here "..\..\..")
$input = "..\..\..\figure1.cdxml"
$objectSelector = "object:obj_mol_004"
$nodeSelector = "node:1176604361"
$nodeId = "1176604361"
$cli = $env:CHEMCORE_CLI

if (-not $cli) {
  if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Push-Location $root
    try {
      & cargo build -p chemcore-cli
      if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
      }
    } finally {
      Pop-Location
    }
  }

  $candidates = @(
    (Join-Path $root "target\debug\chemcore-cli.exe"),
    (Join-Path $root "target\release\chemcore-cli.exe")
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      $cli = $candidate
      break
    }
  }
}

if (-not $cli) {
  $cli = "chemcore-cli"
}

function Invoke-ChemCore {
  param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
  & $cli @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "chemcore-cli failed: $($Arguments -join ' ')"
  }
}

function Assert-File {
  param([string]$Path)
  if (-not (Test-Path $Path)) {
    throw "Expected output was not written: $Path"
  }
  if ((Get-Item $Path).Length -le 0) {
    throw "Expected output is empty: $Path"
  }
}

function Write-Json {
  param([string]$Path, [object]$Value)
  $Value | ConvertTo-Json -Depth 32 | Set-Content -Path $Path -Encoding UTF8
}

function Read-Json {
  param([string]$Path)
  Get-Content -Raw -Encoding UTF8 $Path | ConvertFrom-Json
}

function New-Transaction {
  param([string]$ExpectedHash, [bool]$DryRun)
  [ordered]@{
    schema = "chemcore.command-transaction.v1"
    preconditions = [ordered]@{
      expectedDocumentHash = $ExpectedHash
      requiredSelectors = @($objectSelector, $nodeSelector)
    }
    scope = [ordered]@{
      editableTargets = @($objectSelector)
      includeDescendants = $true
      includeReferencedResources = $true
      allowCreate = $false
      allowDelete = $false
      forbidChangesOutsideScope = $true
    }
    options = [ordered]@{
      atomic = $true
      dryRun = $DryRun
    }
    commands = @(
      [ordered]@{
        type = "replace-node-label"
        node_id = $nodeId
        label = "OMe"
      }
    )
    postconditions = @(
      [ordered]@{ type = "document-valid" },
      [ordered]@{ type = "no-unexpected-changes" },
      [ordered]@{ type = "selector-exists"; selector = $objectSelector },
      [ordered]@{ type = "selector-exists"; selector = $nodeSelector }
    )
  }
}

Push-Location $here
try {
  foreach ($path in @(
      "bundle",
      "targets.json",
      "before.ccjs",
      "before.png",
      "before-capture.json",
      "bundle-command.json",
      "transaction-dry-run.json",
      "dry-run-report.json",
      "transaction-execute.json",
      "execute-report.json",
      "output.ccjs",
      "output.cdxml",
      "diff.json",
      "after.png",
      "after-capture.json",
      "target-subset.ccjs",
      "target-subset.cdxml",
      "full-inspect.json",
      "subset-inspect.json",
      "output-cdxml-inspect.json",
      "acceptance.json"
    )) {
    if (Test-Path $path) {
      Remove-Item -Recurse -Force $path
    }
  }

  Invoke-ChemCore targets $input --out targets.json --pretty
  Invoke-ChemCore convert $input before.ccjs --format ccjs
  & $cli bundle $input `
    --target $objectSelector `
    --out-dir bundle `
    --context-radius 55 `
    --capture-format png `
    --capture-width 1200 `
    --subset-format ccjs `
    --pretty `
    | Set-Content -Path bundle-command.json -Encoding UTF8
  if ($LASTEXITCODE -ne 0) {
    throw "chemcore-cli failed: bundle $input --target $objectSelector"
  }
  Invoke-ChemCore capture $input `
    --target $objectSelector `
    --expand 55 `
    --out before.png `
    --width 1200 `
    --pretty `
    | Set-Content -Path before-capture.json -Encoding UTF8

  $manifest = Read-Json bundle\manifest.json
  $expectedHash = $manifest.source.documentHash
  Write-Json transaction-dry-run.json (New-Transaction -ExpectedHash $expectedHash -DryRun $true)
  Write-Json transaction-execute.json (New-Transaction -ExpectedHash $expectedHash -DryRun $false)

  Invoke-ChemCore run $input transaction-dry-run.json `
    --results dry-run-report.json `
    --pretty
  Invoke-ChemCore run $input transaction-execute.json `
    --out output.ccjs `
    --results execute-report.json `
    --pretty

  Invoke-ChemCore diff before.ccjs output.ccjs --out diff.json --pretty
  Invoke-ChemCore capture output.ccjs `
    --target $objectSelector `
    --expand 55 `
    --out after.png `
    --width 1200 `
    --pretty `
    | Set-Content -Path after-capture.json -Encoding UTF8
  Invoke-ChemCore convert output.ccjs target-subset.ccjs --target $objectSelector --format ccjs
  Invoke-ChemCore convert output.ccjs target-subset.cdxml --target $objectSelector --format cdxml
  Invoke-ChemCore convert output.ccjs output.cdxml --format cdxml
  Invoke-ChemCore inspect output.ccjs --out full-inspect.json --pretty
  Invoke-ChemCore inspect target-subset.ccjs --out subset-inspect.json --pretty
  Invoke-ChemCore inspect output.cdxml --out output-cdxml-inspect.json --pretty

  foreach ($path in @(
      "targets.json",
      "before.ccjs",
      "bundle\manifest.json",
      "bundle\identity-map.json",
      "bundle\provenance.json",
      "bundle\editable-subset.ccjs",
      "bundle\capture.png",
      "bundle-command.json",
      "before.png",
      "before-capture.json",
      "transaction-dry-run.json",
      "dry-run-report.json",
      "transaction-execute.json",
      "execute-report.json",
      "output.ccjs",
      "output.cdxml",
      "diff.json",
      "after.png",
      "after-capture.json",
      "target-subset.ccjs",
      "target-subset.cdxml",
      "full-inspect.json",
      "subset-inspect.json",
      "output-cdxml-inspect.json"
    )) {
    Assert-File $path
  }

  $targets = Read-Json targets.json
  $dryRun = Read-Json dry-run-report.json
  $execute = Read-Json execute-report.json
  $diff = Read-Json diff.json
  $fullInspect = Read-Json full-inspect.json
  $subsetInspect = Read-Json subset-inspect.json
  $cdxmlInspect = Read-Json output-cdxml-inspect.json

  $targetResolved = [bool](
    ($targets.targets.objects | Where-Object { $_.selector -eq $objectSelector }) -and
    ($targets.targets.nodes | Where-Object { $_.selector -eq $nodeSelector })
  )
  $nodeChanged = [bool]($diff.nodes.updated | Where-Object { $_ -eq $nodeSelector })
  $objectChanged = [bool]($diff.objects.updated | Where-Object { $_ -eq $objectSelector })
  $unexpectedCount = @($execute.scope.unexpectedChanges).Count
  if ($null -eq $execute.scope.unexpectedChanges) {
    $unexpectedCount = 0
  }

  $acceptance = [ordered]@{
    targetResolved = $targetResolved
    bundleGenerated = [bool]$manifest.ok
    captureGenerated = (Test-Path before.png) -and (Test-Path after.png)
    dryRunDidNotApply = ($dryRun.transaction.dryRun -eq $true) -and ($dryRun.transaction.applied -eq $false)
    executeApplied = ($execute.transaction.applied -eq $true)
    expectedSelectorsChanged = @($nodeSelector, $objectSelector)
    nodeChanged = $nodeChanged
    objectChanged = $objectChanged
    unexpectedSelectorCount = $unexpectedCount
    fullDocumentValid = [bool]$fullInspect.summary.counts.objects
    subsetDocumentValid = [bool]$subsetInspect.summary.counts.objects
    cdxmlRoundTripValid = [bool]$cdxmlInspect.summary.counts.objects
  }
  Write-Json acceptance.json $acceptance

  if (-not $acceptance.targetResolved) { throw "Target selectors were not resolved." }
  if (-not $acceptance.bundleGenerated) { throw "Bundle manifest reports ok=false." }
  if (-not $acceptance.captureGenerated) { throw "Before/after captures were not generated." }
  if (-not $acceptance.dryRunDidNotApply) { throw "Dry-run transaction applied unexpectedly." }
  if (-not $acceptance.executeApplied) { throw "Execute transaction did not apply." }
  if (-not $acceptance.nodeChanged) { throw "Diff did not report node change." }
  if (-not $acceptance.objectChanged) { throw "Diff did not report object change." }
  if ($acceptance.unexpectedSelectorCount -ne 0) { throw "Unexpected selectors changed." }
  if (-not $acceptance.fullDocumentValid) { throw "Full output inspection failed." }
  if (-not $acceptance.subsetDocumentValid) { throw "Subset output inspection failed." }
  if (-not $acceptance.cdxmlRoundTripValid) { throw "CDXML output inspection failed." }

  Write-Host "Object-grounded edit completed."
} finally {
  Pop-Location
}
