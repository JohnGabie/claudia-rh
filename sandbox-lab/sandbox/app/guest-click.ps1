# guest-click.ps1 - clique de mouse REAL dentro do guest (sessao do usuario, via wsb exec -r ExistingLogin).
# Coordenadas em pixels fisicos da tela do guest (as mesmas do PNG do guest-observe.ps1).
# Substitui o "winapp ui click" quando o Sandbox esta sem rede.
param([Parameter(Mandatory)][int]$X, [Parameter(Mandatory)][int]$Y, [int]$WheelDelta = 0)
Add-Type -Namespace W -Name In -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, int d, System.UIntPtr e);
'@
[void][W.In]::SetProcessDPIAware()
[void][W.In]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 150
if ($WheelDelta -ne 0) { [W.In]::mouse_event(0x0800, 0, 0, $WheelDelta, [UIntPtr]::Zero); "wheel $WheelDelta at $X,$Y"; return }
[W.In]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)   # LEFTDOWN
Start-Sleep -Milliseconds 60
[W.In]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)   # LEFTUP
"click $X,$Y"
