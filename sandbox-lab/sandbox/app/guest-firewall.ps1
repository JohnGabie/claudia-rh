# guest-firewall.ps1 - roda DENTRO do Sandbox como SYSTEM (wsb exec -r System).
# Bloqueia a saida para a INTERNET com uma regra Block (Block vence Allow) cobrindo todos os IPv4 publicos
# e IPv6 global; loopback e redes privadas (NAT do Sandbox <-> host) continuam livres para winapp/MCP.
# Saida: C:\labwork\run\firewall.txt
$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force C:\labwork\run | Out-Null
$public = '0.0.0.0/5','8.0.0.0/7','11.0.0.0/8','12.0.0.0/6','16.0.0.0/4','32.0.0.0/3','64.0.0.0/2','128.0.0.0/3',
          '160.0.0.0/5','168.0.0.0/6','172.0.0.0/12','172.32.0.0/11','172.64.0.0/10','172.128.0.0/9','173.0.0.0/8',
          '174.0.0.0/7','176.0.0.0/4','192.0.0.0/9','192.128.0.0/11','192.160.0.0/13','192.169.0.0/16','192.170.0.0/15',
          '192.172.0.0/14','192.176.0.0/12','192.192.0.0/10','193.0.0.0/8','194.0.0.0/7','196.0.0.0/6','200.0.0.0/5',
          '208.0.0.0/4','224.0.0.0/3','2000::/3'
Set-NetFirewallProfile -All -Enabled True
Remove-NetFirewallRule -DisplayName 'lab-block-internet-out' -ErrorAction SilentlyContinue
New-NetFirewallRule -DisplayName 'lab-block-internet-out' -Direction Outbound -Action Block -RemoteAddress $public | Out-Null
$t = try { Invoke-WebRequest https://www.google.com -UseBasicParsing -TimeoutSec 8 | Out-Null; 'INTERNET: ACESSIVEL (FALHA)' } catch { "INTERNET: BLOQUEADA ($($_.Exception.Message))" }
$t2 = try { Invoke-WebRequest https://storage.googleapis.com -UseBasicParsing -TimeoutSec 8 | Out-Null; 'storage.googleapis.com: ACESSIVEL (FALHA)' } catch { "storage.googleapis.com: BLOQUEADO" }
$o = @((Get-NetFirewallProfile | ForEach-Object { "$($_.Name) enabled=$($_.Enabled)" }) + "regra lab-block-internet-out: $(@(Get-NetFirewallRule -DisplayName 'lab-block-internet-out').Count)" + $t + $t2)
[IO.File]::WriteAllLines('C:\labwork\run\firewall.txt', $o); $o
