// Pure tests: lists of strings only, no disk, no git.
import test from "node:test";
import assert from "node:assert/strict";
import { arquivosProibidos } from "../proibidos.mjs";

const nomes = (lista) => arquivosProibidos(lista).map((a) => a.arquivo);

test("flags the lab secret, tools, output and evidence folders", () => {
  const lista = [
    "sandbox-lab/shared/secrets/mcp-auth.key",
    "sandbox-lab/shared/secrets/outro.txt",
    "sandbox-lab/shared/tools/uv/uv.exe",
    "sandbox-lab/shared/out/status.json",
    "sandbox-lab/evidence/O1-x.png",
    ".sandbox-work/deploy/app/x.exe",
  ];
  assert.deepEqual(nomes(lista), lista);
});

test("flags key files anywhere, including .key.pub", () => {
  assert.deepEqual(nomes(["src-tauri/assinatura.key", "x/y/z.key.pub", "docs/chave.KEY"]),
    ["src-tauri/assinatura.key", "x/y/z.key.pub", "docs/chave.KEY"]);
});

test("flags generated lab files and the lab .mcp.json", () => {
  const lista = [
    "sandbox-lab/.mcp.json",
    "sandbox-lab/sandbox/sandbox.generated.wsb",
    "sandbox-lab/sandbox/guest.config.json",
  ];
  assert.deepEqual(nomes(lista), lista);
});

test("accepts the files the lab versions (tools.sha256 is not tools/)", () => {
  assert.deepEqual(arquivosProibidos([
    "sandbox-lab/scripts/test-up.ps1",
    "sandbox-lab/shared/mcp.json",
    "sandbox-lab/shared/tools.sha256",
    "sandbox-lab/shared/db/backup.sqlite",
    "sandbox-lab/sandbox/sandbox.template.wsb",
    ".claude/skills/claudia-teste-sandbox/SKILL.md",
    "src/main.tsx",
  ]), []);
});

test("is case-insensitive and accepts backslashes and ./", () => {
  assert.deepEqual(nomes(["Sandbox-Lab\\Shared\\Secrets\\x.txt", "./sandbox-lab/shared/TOOLS/a"]),
    ["Sandbox-Lab\\Shared\\Secrets\\x.txt", "./sandbox-lab/shared/TOOLS/a"]);
});

test("every finding has a reason", () => {
  for (const a of arquivosProibidos(["sandbox-lab/shared/secrets/k", "a.key"])) assert.ok(a.motivo.length > 0);
});

test("ignores empty and non-string entries", () => {
  assert.deepEqual(arquivosProibidos(["", "   ", null, undefined, 3]), []);
  assert.deepEqual(arquivosProibidos(undefined), []);
});
