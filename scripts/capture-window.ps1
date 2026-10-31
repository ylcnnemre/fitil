# Captures a screenshot of the running Fitil window (Windows only).
# Works even when the window is behind others, and never takes focus from the user:
#   scripts/capture-window.ps1 -OutFile shot.png [-ProcessId 1234]

param([string]$ProcessName = "fitil", [string]$OutFile, [int]$ProcessId = 0)

Add-Type -AssemblyName System.Drawing
. "$PSScriptRoot\window-helpers.ps1"

$window = Find-FitilWindow $ProcessName $ProcessId
$rect = $window.Rect
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$hdc = $graphics.GetHdc()
# PW_RENDERFULLCONTENT (2) also captures GPU-rendered content.
[FitilWindow]::PrintWindow($window.Handle, $hdc, 2) | Out-Null
$graphics.ReleaseHdc($hdc)
$bitmap.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose(); $bitmap.Dispose()
"Captured ${width}x${height} -> $OutFile"
