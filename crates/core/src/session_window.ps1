$pidToFind = [int]$args[0]
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class AgentPadWindows {
  public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr data);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hWnd, int command);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  public static IntPtr[] WindowsFor(uint process) {
    var found = new List<IntPtr>();
    EnumWindows((window, ignored) => {
      uint owner;
      GetWindowThreadProcessId(window, out owner);
      if (owner == process && IsWindowVisible(window)) found.Add(window);
      return true;
    }, IntPtr.Zero);
    return found.ToArray();
  }
}
'@
for ($depth = 0; $depth -lt 16 -and $pidToFind -gt 1; $depth++) {
  $process = Get-CimInstance Win32_Process -Filter "ProcessId=$pidToFind" -ErrorAction SilentlyContinue
  if ($null -eq $process) { break }
  $windows = [AgentPadWindows]::WindowsFor([uint32]$pidToFind)
  if ($windows.Length -eq 1) {
    [AgentPadWindows]::ShowWindowAsync($windows[0], 9) | Out-Null
    if ([AgentPadWindows]::SetForegroundWindow($windows[0]) -and
        [AgentPadWindows]::GetForegroundWindow() -eq $windows[0]) { 'raised'; exit 0 }
    break
  }
  $pidToFind = [int]$process.ParentProcessId
}
exit 1
