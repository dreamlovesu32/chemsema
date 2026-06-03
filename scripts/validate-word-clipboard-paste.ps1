param(
  [Parameter(Mandatory = $true)]
  [string]$InputCdxml,

  [string]$OutputDir = "tmp/word-clipboard-paste",

  [string]$CargoCommand = "cargo",

  [string]$OfficePackage = "chemcore-office",

  [string]$OfficeCommand = ""
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.IO.Compression.FileSystem

$repoRoot = Split-Path -Parent $PSScriptRoot
$inputPath = (Resolve-Path $InputCdxml).Path
$outRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDir))
$stem = [System.IO.Path]::GetFileNameWithoutExtension($inputPath)
$sampleDir = Join-Path $outRoot $stem

New-Item -ItemType Directory -Force -Path $sampleDir | Out-Null

$payloadPath = Join-Path $sampleDir "$stem.payload.json"
$docxPath = Join-Path $sampleDir "$stem.pasted.docx"
$summaryPath = Join-Path $sampleDir "$stem.summary.json"
$defaultOfficeExe = Join-Path $repoRoot "target\debug\chemcore-office.exe"

function Invoke-Checked {
  param(
    [string]$Command,
    [string[]]$Arguments
  )
  Write-Host "[RUN] $Command $($Arguments -join ' ')"
  & $Command @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code ${LASTEXITCODE}: $Command $($Arguments -join ' ')"
  }
}

function Get-DocxOleSummary {
  param([string]$Path)
  $zip = [System.IO.Compression.ZipFile]::OpenRead($Path)
  try {
    $entries = @($zip.Entries | ForEach-Object { $_.FullName })
    $embeddingEntries = @($entries | Where-Object { $_ -like "word/embeddings/*" })
    $mediaEntries = @($entries | Where-Object { $_ -like "word/media/*" })
    [pscustomobject]@{
      path = $Path
      embeddingCount = $embeddingEntries.Count
      mediaCount = $mediaEntries.Count
      embeddings = $embeddingEntries
      media = $mediaEntries
    }
  }
  finally {
    $zip.Dispose()
  }
}

Invoke-Checked $CargoCommand @(
  "run", "-q",
  "-p", "chemcore-engine",
  "--example", "cdxml_to_clipboard_payload",
  "--",
  $inputPath,
  $payloadPath
)

if ($OfficeCommand) {
  Invoke-Checked $OfficeCommand @("--copy-clipboard-payload", $payloadPath)
} elseif (Test-Path $defaultOfficeExe) {
  Invoke-Checked $defaultOfficeExe @("--copy-clipboard-payload", $payloadPath)
} else {
  Invoke-Checked $CargoCommand @(
    "run", "-q",
    "-p", $OfficePackage,
    "--",
    "--copy-clipboard-payload",
    $payloadPath
  )
}

$word = $null
$doc = $null

try {
  $word = New-Object -ComObject Word.Application
  $word.Visible = $false
  $word.DisplayAlerts = 0

  Write-Host "[WORD] create document"
  $doc = $word.Documents.Add()
  Write-Host "[WORD] paste clipboard"
  $word.Selection.Paste() | Out-Null
  Start-Sleep -Milliseconds 800

  $inlineShapeCount = $doc.InlineShapes.Count
  $shapeCount = $doc.Shapes.Count
  if (($inlineShapeCount + $shapeCount) -lt 1) {
    throw "Word paste did not create a shape or inline shape."
  }
  if ($inlineShapeCount -lt 1) {
    throw "Word paste did not create an inline OLE shape."
  }

  $oleFormat = $doc.InlineShapes.Item(1).OLEFormat
  if ($null -eq $oleFormat) {
    throw "Word paste created an inline shape without OLEFormat; it was pasted as a plain image."
  }
  $progId = $oleFormat.ProgID
  if ($progId -ne "Chemcore.Document.1") {
    throw "Word paste created unexpected OLE ProgID '$progId'."
  }

  Write-Host "[WORD] save pasted document"
  $doc.SaveAs([ref][System.IO.Path]::GetFullPath($docxPath)) | Out-Null
  $doc.Close([ref]0) | Out-Null
  $doc = $null

  $docxOle = Get-DocxOleSummary -Path $docxPath
  if ($docxOle.embeddingCount -lt 1) {
    throw "Saved Word document does not contain an OLE embedding."
  }

  $summary = [pscustomobject]@{
    inputCdxml = $inputPath
    payload = $payloadPath
    pastedDocx = [System.IO.Path]::GetFullPath($docxPath)
    inlineShapeCount = $inlineShapeCount
    shapeCount = $shapeCount
    progId = $progId
    docx = $docxOle
  }
  $summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $summaryPath
  Write-Output $summaryPath
}
finally {
  if ($doc -ne $null) {
    $doc.Close([ref]0) | Out-Null
  }
  if ($word -ne $null) {
    $word.Quit() | Out-Null
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($word) | Out-Null
  }
}
