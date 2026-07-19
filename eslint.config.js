import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

// Regras de arquitetura (ver .claude/QUALIDADE.md):
// - componente ≤ 300 linhas; lógica complexa vive em hooks
// - aninhamento máximo 4 níveis
// Ficheiros legados estão isentos em bloco próprio no fim — a lista SÓ encolhe:
// ao refatorar um ficheiro, remove-o de lá. Nunca adicionar ficheiros novos.

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "max-lines": ["error", { max: 300, skipBlankLines: true, skipComments: true }],
      "max-depth": ["error", 4],
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
    },
  },
  {
    // ── Legado (pré-refactor, 2026-07-19) — remover cada ficheiro ao refatorá-lo ──
    files: [
      "src/components/Perfil.tsx",
      "src/components/Dashboard.tsx",
      "src/components/Feedback.tsx",
      "src/components/Pendencias.tsx",
      "src/components/Vagas.tsx",
      "src/components/Configuracoes.tsx",
    ],
    rules: {
      "max-lines": "off",
      "react-hooks/set-state-in-effect": "warn",
    },
  },
  {
    ignores: ["dist/", "src-tauri/", "node_modules/", "npm-package/"],
  },
);
