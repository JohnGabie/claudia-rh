# Servidor HTTP estatico minimo (guest) servindo C:\lab em http://localhost:<Port>/
# Necessario porque content scripts de extensoes unpacked nao rodam em file:// sem toggle manual.
param([int]$Port = 8080, [string]$Root = 'C:\lab')
$ErrorActionPreference = 'Continue'
$mime = @{ '.html'='text/html; charset=utf-8'; '.htm'='text/html; charset=utf-8'; '.js'='application/javascript'; '.css'='text/css'; '.json'='application/json'; '.png'='image/png'; '.jpg'='image/jpeg'; '.svg'='image/svg+xml'; '.txt'='text/plain; charset=utf-8' }
$l = New-Object System.Net.HttpListener
$l.Prefixes.Add("http://localhost:$Port/")
$l.Prefixes.Add("http://127.0.0.1:$Port/")
$l.Start()
while ($l.IsListening) {
  $ctx = $l.GetContext()
  try {
    $rel = [Uri]::UnescapeDataString($ctx.Request.Url.AbsolutePath.TrimStart('/'))
    if ($rel -eq '') { $rel = 'test-page.html' }
    $full = [IO.Path]::GetFullPath((Join-Path $Root $rel))
    if (-not $full.StartsWith($Root, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path $full -PathType Leaf)) {
      $ctx.Response.StatusCode = 404; $bytes = [Text.Encoding]::UTF8.GetBytes('not found')
    } else {
      $ext = [IO.Path]::GetExtension($full).ToLower()
      $ctx.Response.ContentType = $(if ($mime.ContainsKey($ext)) { $mime[$ext] } else { 'application/octet-stream' })
      $bytes = [IO.File]::ReadAllBytes($full)
    }
    $ctx.Response.OutputStream.Write($bytes, 0, $bytes.Length)
  } catch { } finally { $ctx.Response.Close() }
}
