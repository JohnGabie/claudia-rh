@echo off
REM ============================================================================
REM  Emulacao de busca de vagas no LinkedIn — SOMENTE MONITORIZACAO
REM
REM  Replica a invocacao que a app faz em commands/sessao.rs, com duas
REM  diferencas deliberadas:
REM    1. o system prompt e o de monitorizacao, nao o de producao
REM    2. NAO passa --dangerously-skip-permissions: cada acao pede confirmacao
REM
REM  Usa a extensao Claude in Chrome (--chrome), que e o caminho actual da app.
REM  Para o caminho Playwright, ver monitor-playwright.cmd.
REM ============================================================================

setlocal
cd /d "%~dp0"

echo.
echo   Sessao de MONITORIZACAO — o agente NAO se candidata a nada.
echo   Max 5 vagas, max 15 minutos, so LinkedIn.
echo.
echo   As permissoes NAO estao desligadas: cada accao pede confirmacao.
echo   Se preferires deixar correr, acrescenta --dangerously-skip-permissions
echo   a linha abaixo — mas le o prompt-monitor.md antes.
echo.
pause

claude ^
  --chrome ^
  --mcp-config "%~dp0mcp-monitor.json" ^
  --strict-mcp-config ^
  --system-prompt-file "%~dp0prompt-monitor.md" ^
  "Inicie a sessao de observacao. Escolha uma variante de busca, procure vagas no LinkedIn, e avalie-as em voz alta. Nao se candidate a nada."

endlocal
