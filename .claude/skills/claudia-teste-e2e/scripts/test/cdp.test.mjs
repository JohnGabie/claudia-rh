import test from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import path from "node:path";
import { spawnSync } from "node:child_process";

const SCRIPTS = path.resolve(import.meta.dirname, "..");

async function portaLivre() {
  const srv = net.createServer();
  await new Promise((r) => srv.listen(0, "127.0.0.1", r));
  const { port } = srv.address();
  await new Promise((r) => srv.close(r));
  return port;
}

for (const [script, args] of [["ver.mjs", ["arvore"]], ["invocar.mjs", ["listar_vagas"]]]) {
  test(`${script} exits 1 with a clear message when CDP is not reachable`, async () => {
    const port = await portaLivre();
    const r = spawnSync(process.execPath, [path.join(SCRIPTS, script), ...args], {
      encoding: "utf8",
      env: { ...process.env, CDP_HOST: "127.0.0.1", CDP_PORT: String(port) },
    });
    assert.equal(r.status, 1);
    assert.match(r.stderr, new RegExp(`CDP not reachable at 127\.0\.0\.1:${port}`));
    assert.match(r.stderr, /claudia-teste-sandbox\/scripts\/cdp-guest\.mjs --iniciar/);
    assert.doesNotMatch(r.stderr, /at .*\.mjs:\d+/);
  });
}
