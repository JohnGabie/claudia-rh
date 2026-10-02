// Trigger eval for the claudia-teste skill, without Python's select() on pipes
// (it raises WinError 10038 on Windows and made the official runner report 0 for everything).
//
// Criterion: the skill counts as triggered when the model's FIRST tool call is
// Skill with input.skill === "claudia-teste". Decided from the complete assistant
// event, not from streamed deltas: the tool input arrives in fragments, and an
// earlier version that matched on accumulated deltas missed real triggers.
import { spawn } from "node:child_process";
import fs from "node:fs";

const RAIZ = new URL("../../../../", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
// Reads evals.json (deve_disparar / nao_deve_disparar) and flattens it.
const E = JSON.parse(fs.readFileSync(new URL("./evals.json", import.meta.url), "utf8"));
const SET = [...E.deve_disparar.map((query) => ({ query, should_trigger: true })), ...E.nao_deve_disparar.map((query) => ({ query, should_trigger: false }))];
const MODELO = process.argv[2] ?? "sonnet";
const NOME = "claudia-teste";

function rodar(query) {
  return new Promise((resolve) => {
    const env = { ...process.env }; delete env.CLAUDECODE;
    const p = spawn("claude", ["-p", query, "--output-format", "stream-json", "--verbose", "--model", MODELO],
      { cwd: RAIZ, env, shell: false });
    let buf = "", decidido = null, erro = null;
    const fim = (v, motivo) => { if (decidido === null) { decidido = { disparou: v === true, motivo }; try { p.kill(); } catch {} resolve(decidido); } };
    const t = setTimeout(() => fim(false, "timeout"), 110000);
    p.stdout.on("data", (d) => {
      buf += d.toString();
      let i;
      while ((i = buf.indexOf("\n")) >= 0) {
        const linha = buf.slice(0, i); buf = buf.slice(i + 1);
        let ev; try { ev = JSON.parse(linha); } catch { continue; }
        if (ev.type === "result" && ev.is_error) erro = String(ev.result || "erro").slice(0, 80);
        if (ev.type !== "assistant") continue;
        const uso = (ev.message?.content ?? []).find((c) => c.type === "tool_use");
        if (!uso) continue;                       // text-only message so far
        // The model's first tool call decides it.
        if (uso.name === "Skill") { clearTimeout(t); fim(uso.input?.skill === NOME, `Skill(${uso.input?.skill})`); }
        else { clearTimeout(t); fim(false, `primeira ferramenta: ${uso.name}`); }
      }
    });
    p.on("close", () => { clearTimeout(t); if (erro) { if (decidido === null) { decidido = { disparou: false, invalido: true, motivo: "EXECUCAO FALHOU: " + erro }; resolve(decidido); } } else fim(false, "terminou sem ferramenta"); });
    p.on("error", () => fim(false, "erro ao lançar claude"));
  });
}

const res = new Array(SET.length);
let prox = 0;
await Promise.all(Array.from({ length: 3 }, async () => {
  while (prox < SET.length) { const i = prox++; res[i] = await rodar(SET[i].query); process.stderr.write("."); }
}));

let ok = 0, invalidos = 0;
for (const [i, item] of SET.entries()) {
  const r = res[i];
  if (r.invalido) { invalidos++; console.log(`INVALIDO  ${item.query}  [${r.motivo}]`); continue; }
  const passou = r.disparou === item.should_trigger;
  if (passou) ok++;
  console.log(`${passou ? "PASS" : "FAIL"}  disparou=${String(r.disparou).padEnd(5)} esperado=${String(item.should_trigger).padEnd(5)}  ${item.query}  [${r.motivo}]`);
}
// Invalid runs (usage limit, crash) are neither passes nor misses, so report them apart.
console.log(`\n${ok}/${SET.length - invalidos} validos, ${invalidos} invalidos (modelo ${MODELO})`);
if (invalidos) process.exitCode = 3;
