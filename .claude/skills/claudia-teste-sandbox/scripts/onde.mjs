// Prints where the lab, the work folder and the project under test are, as JSON. Read-only (git rev-parse only).
// Usage: node .claude/skills/claudia-teste-sandbox/scripts/onde.mjs
import { onde, falhaDe } from "./comum.mjs";

try {
  console.log(JSON.stringify(onde()));
} catch (e) { falhaDe(e); }
