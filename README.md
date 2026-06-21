<div align="center">

<img src="src-tauri/icons/128x128.png" alt="Claudia RH logo" width="96" height="96" />

<h1>Claudia RH</h1>

<p><strong>Candidatura automática a empregos, orquestrada por IA.</strong><br/>
<em>Automated job-application agent powered by Claude + Chrome.</em></p>

<p>
  <a href="https://github.com/JohnGabie/claudia-rh/stargazers">
    <img src="https://img.shields.io/github/stars/JohnGabie/claudia-rh?style=for-the-badge&color=D97757&labelColor=1a1a1a" alt="GitHub stars" />
  </a>
  <img src="https://img.shields.io/badge/Tauri-v2-24C8D8?style=for-the-badge&logo=tauri&logoColor=white&labelColor=1a1a1a" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=white&labelColor=1a1a1a" alt="React 19" />
  <img src="https://img.shields.io/badge/Rust-stable-CE422B?style=for-the-badge&logo=rust&logoColor=white&labelColor=1a1a1a" alt="Rust" />
  <img src="https://img.shields.io/badge/Windows-11-0078D4?style=for-the-badge&logo=windows&logoColor=white&labelColor=1a1a1a" alt="Windows 11" />
</p>

<br/>

> ⭐ **Se este projeto te inspirou ou ajudou, considera deixar uma estrela — faz diferença!**<br/>
> *If this project inspired or helped you, please consider leaving a star — it really matters!*

</div>

---

## O que é isto? / What is this?

**Claudia RH** é uma aplicação desktop Windows que automatiza a procura e candidatura a vagas de emprego. Em vez de passar horas a copiar o CV de site em site, lança uma sessão de IA que navega, analisa e submete candidaturas por si — enquanto você faz outra coisa.

*Claudia RH is a Windows desktop app that automates job searching and application. Instead of spending hours copy-pasting your CV across job sites, it launches an AI session that browses, analyses, and submits applications on your behalf.*

<br/>

<table>
<tr>
<td width="50%" valign="top">

### Como funciona / How it works

1. **Perfil conversacional** — conta ao Claude quem és, colas o CV, defines variantes de pesquisa (ex: "dev sénior remoto" vs "líder técnico híbrido")
2. **Disparo automático** — a app deteta quando estás inativo e lança uma sessão Claude Code ligada ao Chrome
3. **Análise de vaga** — o agente lê cada oferta, cruza com o teu perfil e decide se candidata
4. **Geração de documentos** — CV e carta de apresentação adaptados para cada vaga, com honestidade forçada
5. **Pendências e notificações** — captchas, campos obrigatórios e outros bloqueios surgem como notificações Windows para resolução manual
6. **Feedback** — análise periódica dos resultados com sugestões de melhoria

</td>
<td width="50%" valign="top">

### Funcionalidades / Features

| | Funcionalidade |
|---|---|
| 🤖 | Agente Claude Code com acesso ao Chrome |
| 📄 | CV e carta gerados por IA por vaga |
| 🔍 | Múltiplas variantes de pesquisa simultâneas |
| 🔔 | Notificações Windows para captchas/bloqueios |
| 💤 | Disparo automático por inatividade |
| 🔐 | Credenciais no Windows Credential Manager |
| 📊 | Dashboard com estatísticas em tempo real |
| 💬 | Perfil gerido por chat conversacional |
| 📈 | Aba de Feedback com gráficos de tendência |
| 🖥️ | Terminal PTY integrado |

</td>
</tr>
</table>

---

## Arquitetura / Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Claudia RH (Tauri)                     │
│                                                          │
│  ┌──────────┐  ┌────────┐  ┌──────────┐  ┌──────────┐  │
│  │Dashboard │  │ Perfil │  │Pendências│  │ Feedback │  │
│  │          │  │ (chat) │  │          │  │ (graphs) │  │
│  └──────────┘  └────────┘  └──────────┘  └──────────┘  │
│                                                          │
│       React 19 + TypeScript + xterm.js                   │
│  ─────────────────────────────────────────────────────   │
│       Rust (Tauri v2) + SQLite + keyring                 │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │  PTY Manager  →  claude --chrome  →  Chrome     │    │
│  │                   (Claude Code CLI)   Extension  │    │
│  └─────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
         ↕ eventos Tauri (SQLite watcher, idle watcher)
    SQLite DB: vagas · candidaturas · pendências · sessões
```

---

## Stack técnica / Tech stack

<table>
<tr>
<th>Camada</th>
<th>Tecnologia</th>
<th>Versão</th>
<th>Para quê</th>
</tr>
<tr>
<td>Desktop shell</td>
<td><img src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white" /></td>
<td>v2</td>
<td>App nativa Windows sem Electron</td>
</tr>
<tr>
<td>Frontend</td>
<td><img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=white" /></td>
<td>19.1</td>
<td>UI reativa</td>
</tr>
<tr>
<td>Linguagem UI</td>
<td><img src="https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript&logoColor=white" /></td>
<td>5.8</td>
<td>Type-safety no frontend</td>
</tr>
<tr>
<td>Backend</td>
<td><img src="https://img.shields.io/badge/Rust-stable-CE422B?logo=rust&logoColor=white" /></td>
<td>2021 edition</td>
<td>Lógica de sistema, PTY, SQLite, credenciais</td>
</tr>
<tr>
<td>Base de dados</td>
<td><img src="https://img.shields.io/badge/SQLite-bundled-003B57?logo=sqlite&logoColor=white" /></td>
<td>via rusqlite</td>
<td>Vagas, candidaturas, sessões, pendências</td>
</tr>
<tr>
<td>Terminal</td>
<td><code>xterm.js + portable-pty</code></td>
<td>6.x</td>
<td>PTY real dentro da app</td>
</tr>
<tr>
<td>IA</td>
<td><img src="https://img.shields.io/badge/Claude_Code-CLI-D97757" /></td>
<td>latest</td>
<td>Agente de candidatura com acesso ao Chrome</td>
</tr>
<tr>
<td>Credenciais</td>
<td><code>keyring v3</code></td>
<td>3.x</td>
<td>Windows Credential Manager — nunca em texto simples</td>
</tr>
</table>

---

## Pré-requisitos / Prerequisites

<table>
<tr>
<td>

**Obrigatório / Required**
- Windows 11 (build ≥ 26200)
- [Claude Code CLI](https://claude.ai/code) instalado e autenticado
- Extensão **Claude in Chrome** instalada no Chrome
- Rust toolchain (`rustup`)
- Node.js 20+

</td>
<td>

**Recomendado / Recommended**
- Conta Claude Pro ou Team (para volumes de sessões)
- Chrome como browser principal
- 8 GB RAM+

</td>
</tr>
</table>

---

## Instalação / Installation

```bash
# 1. Clonar / Clone
git clone https://github.com/JohnGabie/claudia-rh.git
cd claudia-rh

# 2. Instalar dependências / Install dependencies
npm install

# 3. Desenvolver / Dev mode
npm run tauri dev

# 4. Build de produção / Production build
npm run tauri build
```

O instalador `.msi` fica em `src-tauri/target/release/bundle/msi/`.

---

## Estrutura do projeto / Project structure

```
claudia-rh/
├── src/                        # Frontend React
│   ├── components/
│   │   ├── Dashboard.tsx       # Estatísticas e controlo de sessão
│   │   ├── Perfil.tsx          # Chat conversacional de perfil
│   │   ├── Vagas.tsx           # Histórico de vagas e candidaturas
│   │   ├── Pendencias.tsx      # Bloqueios pendentes (captcha, etc.)
│   │   ├── Feedback.tsx        # Análise de resultados + gráficos
│   │   ├── Configuracoes.tsx   # Credenciais, disparo, estratégia
│   │   ├── Terminal.tsx        # Terminal PTY integrado
│   │   └── Sidebar.tsx         # Navegação + badges
│   └── styles/tokens.css       # Tokens de design (cores, tipografia)
│
└── src-tauri/src/              # Backend Rust
    ├── commands/
    │   ├── perfil.rs           # candidate_base.yaml + search_variants.yaml
    │   ├── sessao.rs           # Disparo e gestão de sessões
    │   ├── curriculos.rs       # Geração de CV
    │   ├── cover_letter.rs     # Geração de carta
    │   ├── feedback.rs         # Análise de feedback
    │   ├── credenciais.rs      # Keyring (Windows Credential Manager)
    │   └── estado.rs           # Queries de estatísticas
    ├── db/schema.sql           # Schema SQLite completo
    ├── pty_manager.rs          # Gestão do processo PTY + sinais da sessão
    ├── idle_watcher.rs         # Deteção de inatividade (Win32 GetLastInputInfo)
    └── notificacoes.rs         # Notificações Windows (plugin Tauri)
```

---

## Aviso legal / Disclaimer

> Esta ferramenta automatiza ações no browser em seu nome. Certifica-te de que os termos de serviço das plataformas de emprego que utilizas permitem automação. O autor não se responsabiliza por bloqueios de conta ou outros efeitos decorrentes do uso desta ferramenta.

*This tool automates browser actions on your behalf. Ensure the terms of service of the job platforms you use allow automation. The author is not responsible for account bans or other effects resulting from using this tool.*

---

## Contribuir / Contributing

Issues e PRs são bem-vindos. Antes de abrir um PR grande, abre uma issue para discutir a abordagem.

*Issues and PRs are welcome. Before opening a large PR, open an issue to discuss the approach.*

---

<div align="center">

Feito com ☕ e demasiadas candidaturas rejeitadas.<br/>
*Built with ☕ and way too many rejected applications.*

<br/>

**Se chegaste até aqui e achaste útil — uma estrela faz diferença! ⭐**<br/>
*If you made it this far and found it useful — a star makes a difference!*

<a href="https://github.com/JohnGabie/claudia-rh/stargazers">
  <img src="https://img.shields.io/github/stars/JohnGabie/claudia-rh?style=social" alt="Star on GitHub" />
</a>

</div>
