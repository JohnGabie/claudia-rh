<#
.SYNOPSIS
  Read-only screenshot of the app's main window, or of the whole screen, without CDP.
  Captures what is on screen: a window covered by another shows the one on top.
.PARAMETER Processo     Process name (default: claudia-rh).
.PARAMETER TelaInteira  Capture the whole virtual screen (use it for native dialogs or when the window is not found).
.PARAMETER Saida        PNG path (default: debug-logs\prints\<timestamp>.png under the repo root).
#>
param([string]$Processo = 'claudia-rh', [switch]$TelaInteira, [string]$Saida = '')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -Namespace W -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool IsIconic(System.IntPtr h);
public struct RECT { public int Left, Top, Right, Bottom; }
'@
[void][W.U]::SetProcessDPIAware()
try {
  if (-not $Saida) {
    $root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
    $Saida = Join-Path $root ('debug-logs\prints\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.png')
  }
  $Saida = [IO.Path]::GetFullPath($Saida, (Get-Location).Path)
  New-Item -ItemType Directory -Force (Split-Path $Saida) | Out-Null
  if ($TelaInteira) {
    $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $x = $b.Left; $y = $b.Top; $w = $b.Width; $h = $b.Height
  } else {
    $p = Get-Process -Name $Processo -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if (-not $p) { throw "No visible window for process '$Processo'. Use -TelaInteira." }
    if ([W.U]::IsIconic($p.MainWindowHandle)) { throw "Window of '$Processo' is minimized: restore it or use -TelaInteira." }
    $r = New-Object W.U+RECT
    [void][W.U]::GetWindowRect($p.MainWindowHandle, [ref]$r)
    $x = $r.Left; $y = $r.Top; $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
  }
  if ($w -le 0 -or $h -le 0) { throw "Window of '$Processo' has no visible area: use -TelaInteira." }
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
  $bmp.Save($Saida, [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output ([ordered]@{ arquivo = $Saida; largura = $w; altura = $h } | ConvertTo-Json -Compress)
  exit 0
} catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
