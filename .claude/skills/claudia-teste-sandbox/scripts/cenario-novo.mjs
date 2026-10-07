// Clean-machine scenario "novo": empty app data folder, so the app starts from scratch.
// Usage: node cenario-novo.mjs [--saida <folder>]   (default Z:\claudia-teste-sandbox\novo, recreated on every run; only inside the guest, from the PC use cenario-no-guest.mjs)
import { parseArgs } from "node:util";
import { novoCenarioDir, escreverCenario, falhaDe } from "./comum.mjs";

try {
  const { values } = parseArgs({ options: { saida: { type: "string" } } });
  console.log(escreverCenario(novoCenarioDir(values.saida, "novo"), "novo"));
} catch (e) { falhaDe(e); }
