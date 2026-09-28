@echo off
REM ============================================================================
REM  Emulacao de busca de vagas no LinkedIn — SOMENTE MONITORIZACAO
REM  Variante Playwright: o browser vem do @playwright/mcp, nao da extensao.
REM
REM  Perfil DEDICADO e SEPARADO do prototipo da CLA-7:
REM    perfil-monitor/   <- este script
REM    perfil-claudia/   <- Tarefa 2 da CLA-7, NAO TOCAR
REM
REM  Na primeira execucao o perfil esta vazio e o LinkedIn vai pedir login.
REM  O login e feito a mao, pelo utilizador, na janela que abre.
REM ============================================================================

setlocal
cd /d "%~dp0"

echo.
echo   Sessao de MONITORIZACAO via Playwright MCP.
echo   O agente NAO se candidata a nada. Max 5 vagas, max 15 minutos.
echo.
echo   Perfil: %~dp0perfil-monitor
echo   Se for a primeira vez, faz login a mao na janela que abrir.
echo.
pause

claude ^
  --mcp-config "%~dp0mcp-monitor-playwright.json" ^
  --strict-mcp-config ^
  --system-prompt-file "%~dp0prompt-monitor.md" ^
  "Inicie a sessao de observacao. Use as tools do playwright para navegar. Escolha uma variante de busca, procure vagas no LinkedIn, e avalie-as em voz alta. Nao se candidate a nada."

endlocal
