// Exposes the app's WebView2 DevTools port (inside the sandbox guest) to the host, optionally starting the app with CDP on.
// Prints {host, porta, browser}: use them as CDP_HOST / CDP_PORT with the claudia-app scripts.
// Usage: node cdp-guest.mjs [--id <sandbox>] [--porta 9333] [--porta-host 19333] [--iniciar] [--exe-path <exe in guest>] [--timeout 90]
//   --iniciar   start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<porta>
//   --exe-path  app exe inside the guest (e.g. the installed copy); without it, the lab's guest-run.ps1 starts the published exe
import { parseArgs } from "node:util";
import { sandboxId, sandboxIp, noGuest, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: {
    id: { type: "string" }, porta: { type: "string", default: "9333" }, "porta-host": { type: "string", default: "19333" },
    iniciar: { type: "boolean" }, "exe-path": { type: "string" }, timeout: { type: "string", default: "90" },
  } });
  const porta = Number(values.porta), portaHost = Number(values["porta-host"]), timeout = Number(values.timeout);
  const id = sandboxId(values.id);
  const ip = sandboxIp(id);
  noGuest(id, "System", [
    `netsh interface portproxy delete v4tov4 listenaddress=0.0.0.0 listenport=${portaHost} | Out-Null`,
    `netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=${portaHost} connectaddress=127.0.0.1 connectport=${porta} | Out-Null`,
    "Remove-NetFirewallRule -DisplayName 'claudia-cdp' -ErrorAction SilentlyContinue",
    `New-NetFirewallRule -DisplayName 'claudia-cdp' -Direction Inbound -Action Allow -Protocol TCP -LocalPort ${portaHost} | Out-Null`,
  ].join("\n"));
  if (values.iniciar) {
    const exe = values["exe-path"];
    const start = exe ? `Start-Process '${exe.replace(/'/g, "''")}'` : "& C:\\labsandbox\\app\\guest-run.ps1";
    noGuest(id, "ExistingLogin", `$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=${porta}'; ${start}`);
  }
  const url = `http://${ip}:${portaHost}/json/version`;
  const fim = Date.now() + timeout * 1000;
  let v = null;
  while (!v && Date.now() < fim) {
    try { v = await (await fetch(url, { signal: AbortSignal.timeout(3000) })).json(); }
    catch { await new Promise((r) => setTimeout(r, 2000)); }
  }
  if (!v) throw new Error(`CDP did not answer at ${url} within ${timeout}s. Check that WebView2 exists in the guest and the app is running (the claudia-app screenshot ladder shows why).`);
  console.log(JSON.stringify({ host: ip, porta: portaHost, browser: v.Browser }));
} catch (e) { falhaDe(e); }
