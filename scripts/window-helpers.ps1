# Shared Win32 helpers for capture-window.ps1 and click-window.ps1.

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;
public static class FitilWindow {
    public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hWnd, ref POINT point);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    public static List<IntPtr> WindowsOf(uint pid) {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) found.Add(h); return true; }, IntPtr.Zero);
        return found;
    }
}
"@ -ErrorAction SilentlyContinue

[FitilWindow]::SetProcessDPIAware() | Out-Null

# The largest visible window of the process, with its screen rectangle.
function Find-FitilWindow([string]$ProcessName) {
    $process = Get-Process $ProcessName -ErrorAction Stop | Select-Object -First 1
    $best = $null
    foreach ($handle in [FitilWindow]::WindowsOf([uint32]$process.Id)) {
        $rect = New-Object FitilWindow+RECT
        [FitilWindow]::GetWindowRect($handle, [ref]$rect) | Out-Null
        $area = ($rect.Right - $rect.Left) * ($rect.Bottom - $rect.Top)
        if (-not $best -or $area -gt $best.Area) {
            $best = @{ Handle = $handle; Rect = $rect; Area = $area }
        }
    }
    if (-not $best) { throw "No visible window for $ProcessName" }
    $best
}
