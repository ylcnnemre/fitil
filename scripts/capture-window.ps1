# Captures a screenshot of the running Fitil window (Windows only).
# Lets us check UI changes without looking at the screen:
#   scripts/capture-window.ps1 -OutFile shot.png

param([string]$ProcessName = "fitil", [string]$OutFile)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;
public static class Win {
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attr, out RECT rect, int size);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public static List<IntPtr> WindowsOf(uint pid) {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) found.Add(h); return true; }, IntPtr.Zero);
        return found;
    }
}
"@

[Win]::SetProcessDPIAware() | Out-Null
$proc = Get-Process $ProcessName -ErrorAction Stop | Select-Object -First 1
$best = $null; $bestArea = 0; $bestRect = $null
foreach ($h in [Win]::WindowsOf([uint32]$proc.Id)) {
    $r = New-Object Win+RECT
    [Win]::DwmGetWindowAttribute($h, 9, [ref]$r, [System.Runtime.InteropServices.Marshal]::SizeOf($r)) | Out-Null
    $area = ($r.Right - $r.Left) * ($r.Bottom - $r.Top)
    if ($area -gt $bestArea) { $best = $h; $bestArea = $area; $bestRect = $r }
}
if (-not $best) { throw "No visible window for $ProcessName" }
[Win]::SetForegroundWindow($best) | Out-Null
Start-Sleep -Milliseconds 600
$w = $bestRect.Right - $bestRect.Left; $hgt = $bestRect.Bottom - $bestRect.Top
$bmp = New-Object System.Drawing.Bitmap $w, $hgt
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($bestRect.Left, $bestRect.Top, 0, 0, $bmp.Size)
$bmp.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"Captured ${w}x${hgt} -> $OutFile"
