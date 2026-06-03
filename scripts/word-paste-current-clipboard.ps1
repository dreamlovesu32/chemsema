param(
  [Parameter(Mandatory = $true)]
  [string]$OutputDocx
)

$ErrorActionPreference = "Stop"

$outputPath = [System.IO.Path]::GetFullPath($OutputDocx)
$outputDir = Split-Path -Parent $outputPath
if (-not (Test-Path $outputDir)) {
  New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}

$word = $null
$doc = $null

try {
  $word = New-Object -ComObject Word.Application
  $word.Visible = $false
  $word.DisplayAlerts = 0

  $doc = $word.Documents.Add()
  $word.Selection.Paste() | Out-Null
  Start-Sleep -Milliseconds 800

  Write-Host "inline=$($doc.InlineShapes.Count) shapes=$($doc.Shapes.Count)"
  if ($doc.InlineShapes.Count -gt 0) {
    $oleFormat = $doc.InlineShapes.Item(1).OLEFormat
    if ($null -ne $oleFormat) {
      Write-Host "progId=$($oleFormat.ProgID)"
    }
  }

  $format = 16
  $doc.SaveAs2([ref]$outputPath, [ref]$format) | Out-Null
  Write-Output $outputPath
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
