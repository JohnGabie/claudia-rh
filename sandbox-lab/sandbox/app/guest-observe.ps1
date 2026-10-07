# guest-observe.ps1 - roda DENTRO do Sandbox na sessao do usuario (wsb exec -r ExistingLogin).
# Alternativa ao winapp quando o Sandbox esta SEM rede (o winapp nao alcanca o guest sem rede):
#   - screenshot da tela inteira (System.Drawing) -> C:\labwork\run\<Name>.png
#   - arvore UIA (nomes/tipos) das janelas do processo alvo -> C:\labwork\run\<Name>-uia.txt
#   - processos cujo nome casa com -Match
param([string]$Name = 'shot', [string]$Process = '', [string]$Match = 'msedgewebview2|chrome', [int]$MaxDepth = 12)
$ErrorActionPreference = 'Continue'
$Out = 'C:\labwork\run'; New-Item -ItemType Directory -Force $Out | Out-Null
Add-Type -AssemblyName System.Windows.Forms, System.Drawing, UIAutomationClient, UIAutomationTypes
# DPI-aware para capturar em pixels fisicos
Add-Type -Namespace W -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][W.Dpi]::SetProcessDPIAware()
$b = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Left, $b.Top, 0, 0, $bmp.Size)
$bmp.Save((Join-Path $Out "$Name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()

$lines = New-Object System.Collections.Generic.List[string]
$procs = @(if ($Process) { Get-Process -Name $Process -ErrorAction SilentlyContinue })
$lines.Add("processos $Process : $($procs.Id -join ',')")
$pattern = $(if ($Process) { "$([regex]::Escape($Process))|$Match" } else { $Match })
Get-Process | Where-Object { $_.Name -match $pattern } | ForEach-Object {
  $p = $_; $path = try { $p.Path } catch { '?' }; $lines.Add("proc $($p.Name)($($p.Id)) $path")
} | Out-Null
$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
function Dump($el, [int]$d) {
  if ($d -gt $MaxDepth -or -not $el) { return }
  try {
    $c = $el.Current
    $lines.Add(('  ' * $d) + "[$($c.ControlType.ProgrammaticName -replace 'ControlType\.','')] '$($c.Name)' id=$($c.AutomationId)")
  } catch { return }
  $ch = $walker.GetFirstChild($el)
  while ($ch) { Dump $ch ($d + 1); $ch = $walker.GetNextSibling($ch) }
}
$root = [System.Windows.Automation.AutomationElement]::RootElement
foreach ($p in $procs) {
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $p.Id)
  foreach ($w in $root.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)) { Dump $w 0 }
}
[IO.File]::WriteAllLines((Join-Path $Out "$Name-uia.txt"), $lines)
"ok $Name : $($lines.Count) linhas"
