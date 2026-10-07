// Pure tests: lists of strings only, no disk, no git.
import test from "node:test";
import assert from "node:assert/strict";
import { arquivosProibidos, achaCaminhoPessoal } from "../proibidos.mjs";

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

const B = "\\";
const naoAcha = (t) => assert.deepEqual(achaCaminhoPessoal(t), [], t);
const acha = (t) => assert.equal(achaCaminhoPessoal(t).length, 1, t);

test("achaCaminhoPessoal: Windows user folder with \\, / and JSON-escaped \\\\", () => {
  acha(["C:", "Users", "fulano", "x"].join(B));
  acha(["c:", "users", "fulano"].join("/"));
  acha(["D:", "Users", "fulano", "a"].join(B + B));
});

test("achaCaminhoPessoal: macOS and Linux home folders", () => {
  acha(`cd ${"/" + ["Users", "fulano", "x"].join("/")}`);
  acha(`"${"/" + ["home", "fulano"].join("/")}"`);
});

test("achaCaminhoPessoal: IDE project folders", () => {
  acha("Pycharm" + "Projects/x");
  acha("Idea" + "Projects");
  acha(["source", "repos", "x"].join(B));
});

test("achaCaminhoPessoal: placeholders, variables, the Sandbox user and guest paths are fine", () => {
  naoAcha(["C:", "Users", "<nome>", "x"].join(B));
  naoAcha(["C:", "Users", "%USERNAME%"].join(B));
  naoAcha(["C:", "Users", "$env:USERNAME"].join(B));
  naoAcha(["C:", "Users", "WDAGUtilityAccount", "AppData"].join(B));
  naoAcha(["C:", "Users", "Public"].join(B));
  naoAcha("%USERPROFILE%\\.rustup\\toolchains");
  naoAcha("C:\\lab\\tools\\sqlite\\sqlite3.exe and C:\\labwork\\db");
  naoAcha("https://example.com/ho" + "me/x");
});

test("achaCaminhoPessoal: Public-like names are still flagged", () => {
  acha(["C:", "Users", "Publicador"].join(B));
});

test("achaCaminhoPessoal: reports the line number and the match", () => {
  const r = achaCaminhoPessoal(`a\nb\nx ${["C:", "Users", "fulano"].join(B)} y`);
  assert.deepEqual(r, [{ linha: 3, trecho: ["C:", "Users", "fulano"].join(B) }]);
});
