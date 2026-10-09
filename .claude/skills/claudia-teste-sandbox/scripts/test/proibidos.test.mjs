// Pure tests: lists of strings only, no disk, no git.
import test from "node:test";
import assert from "node:assert/strict";
import { arquivosProibidos, achaCaminhoPessoal, achaToken, textoParaConferir } from "../proibidos.mjs";

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

test("achaCaminhoPessoal: Git Bash, WSL and doubly JSON-escaped Windows user folders", () => {
  acha(`cd ${"/" + ["c", "Users", "fulano", "x"].join("/")}`);
  acha(`"${"/" + ["mnt", "c", "Users", "fulano"].join("/")}"`);
  acha(`${"/" + ["mnt", "d", "users", "fulano"].join("/")}`);
  acha(["C:", "Users", "fulano", "x"].join(B + B + B + B)); // 4 backslashes = JSON inside a JSON string
  acha(["C:", "Users", "fulano"].join(B + B + B));
});

test("achaCaminhoPessoal: Git Bash, WSL and escaped forms keep the same exceptions", () => {
  naoAcha(`cd ${"/" + ["c", "Users", "WDAGUtilityAccount", "x"].join("/")}`);
  naoAcha(`${"/" + ["mnt", "c", "Users", "Public"].join("/")}`);
  naoAcha(["C:", "Users", "<nome>"].join(B + B + B + B));
  naoAcha(["C:", "Users", "WDAGUtilityAccount"].join(B + B + B + B));
  naoAcha("https://example.com/c/Users/fulano");
  naoAcha("see /c/Userspace/x");
});

test("achaCaminhoPessoal: paths wrapped in Markdown backticks are flagged", () => {
  acha("`" + "/" + ["c", "Users", "fulano", "x"].join("/") + "`");
  acha("`" + "/" + ["Users", "fulano", "x"].join("/") + "`");
  acha("`" + ["C:", "Users", "fulano", "x"].join(B) + "`");
  naoAcha("`" + "/" + ["c", "Users", "WDAGUtilityAccount"].join("/") + "`");
});

test("textoParaConferir: sandbox-lab, skills, docs and CLAUDE.md only", () => {
  for (const a of ["sandbox-lab/README.md", ".claude/skills/x/SKILL.md", "docs/a.md", "docs/superpowers/plans/p.md", "CLAUDE.md", "Docs/B.md"]) {
    assert.equal(textoParaConferir(a), true, a);
  }
  for (const a of ["src/App.tsx", "README.md", "docs", "notdocs/a.md", "sub/CLAUDE.md", "CLAUDE.md.bak", ""]) {
    assert.equal(textoParaConferir(a), false, a);
  }
});

test("achaCaminhoPessoal: Public-like names are still flagged", () => {
  acha(["C:", "Users", "Publicador"].join(B));
});

test("achaCaminhoPessoal: reports the line number and the match", () => {
  const r = achaCaminhoPessoal(`a\nb\nx ${["C:", "Users", "fulano"].join(B)} y`);
  assert.deepEqual(r, [{ linha: 3, trecho: ["C:", "Users", "fulano"].join(B) }]);
});

// Fake tokens are assembled at run time, so this source file never holds a token-shaped string.
const falso = (tipo) => ["sk", "ant", tipo, "A1b2C3d4E5f6G7h8I9j0K1l2M3n4"].join("-");

test("achaToken: finds OAuth access, refresh and API key shapes by line, never returns the text", () => {
  const texto = ["linha ok", `token=${falso("oat01")}`, "x", `"${falso("ort01")}"`, `ANTHROPIC_API_KEY=${falso("api03")}`].join("\n");
  const achados = achaToken(texto);
  assert.deepEqual(achados, [{ linha: 2 }, { linha: 4 }, { linha: 5 }]);
  assert.equal(JSON.stringify(achados).includes("A1b2C3"), false);
});

test("achaToken: finds a credential JSON with a long access or refresh token", () => {
  const chave = ["access", "Token"].join("");
  assert.deepEqual(achaToken(`{"${chave}": "${"x".repeat(30)}"}`), [{ linha: 1 }]);
});

test("achaToken: ignores the bare prefix, short values, empty tokens and ordinary text", () => {
  const chave = ["refresh", "Token"].join("");
  for (const t of ["sk-ant-", `sk-ant-oat01-curto`, `{"${chave}": ""}`, `{"${chave}": "abc"}`, "a skate-ant 01", ""]) {
    assert.deepEqual(achaToken(t), [], t);
  }
});

test("flags the lab installer cache and any Claude credential file", () => {
  const lista = ["sandbox-lab/shared/cache/rust.msi", ".credentials.json", "x/.claude/.credentials.json", ".sandbox-work/credencial/.credentials.json"];
  assert.deepEqual(nomes(lista), lista);
  assert.deepEqual(nomes(["docs/credentials.md", "x/credentials.json.example"]), []);
});
