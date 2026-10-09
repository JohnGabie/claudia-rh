CREATE TABLE IF NOT EXISTS vagas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    titulo TEXT NOT NULL,
    empresa TEXT NOT NULL,
    plataforma TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    localizacao TEXT,
    modelo_trabalho TEXT,
    idioma TEXT,
    descoberta_em TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'descoberta',
    motivo_status TEXT,
    match_score TEXT
);

CREATE TABLE IF NOT EXISTS candidaturas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER NOT NULL REFERENCES vagas(id),
    enviada_em TEXT NOT NULL,
    pasta_arquivos TEXT NOT NULL,
    metodo TEXT NOT NULL,
    documento_id INTEGER REFERENCES documentos(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS pendencias (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER NOT NULL REFERENCES vagas(id),
    criada_em TEXT NOT NULL,
    categoria TEXT NOT NULL,
    descricao TEXT NOT NULL,
    resolvida BOOLEAN NOT NULL DEFAULT 0,
    resolvida_em TEXT,
    resolucao TEXT
);

CREATE TABLE IF NOT EXISTS propostas_perfil (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER REFERENCES vagas(id),
    criada_em TEXT NOT NULL,
    pergunta TEXT NOT NULL,
    contexto TEXT,
    promovida BOOLEAN NOT NULL DEFAULT 0,
    promovida_em TEXT
);

CREATE TABLE IF NOT EXISTS sessoes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    iniciada_em TEXT NOT NULL,
    terminada_em TEXT,
    motivo_disparo TEXT NOT NULL,
    motivo_termino TEXT,
    vagas_processadas INTEGER DEFAULT 0
);

-- Documents the agent may attach (CVs, cover letters). `arquivo` is a bare file name inside
-- <app data>/documentos/: never a path.
CREATE TABLE IF NOT EXISTS documentos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tipo TEXT NOT NULL CHECK (tipo IN ('curriculo', 'carta')),
    nome TEXT NOT NULL,
    origem TEXT NOT NULL CHECK (origem IN ('gerado', 'importado')),
    arquivo TEXT NOT NULL,
    arquivo_html TEXT,
    formato TEXT NOT NULL CHECK (formato IN ('pdf', 'docx')),
    idioma TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    sha256 TEXT NOT NULL,
    tamanho INTEGER NOT NULL,
    vaga_id INTEGER REFERENCES vagas(id),
    variante_id TEXT,
    criado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One preferred document per (tipo, variante). variante_id '' = the general preference.
CREATE TABLE IF NOT EXISTS preferidos (
    tipo TEXT NOT NULL,
    variante_id TEXT NOT NULL DEFAULT '',
    documento_id INTEGER NOT NULL REFERENCES documentos(id) ON DELETE CASCADE,
    PRIMARY KEY (tipo, variante_id)
);

-- Which document was attached to a vaga: written by the app on every successful browser_upload.
-- documento_id goes NULL if the document is deleted later (the attachment still happened).
CREATE TABLE IF NOT EXISTS anexos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vaga_id INTEGER NOT NULL REFERENCES vagas(id),
    tipo TEXT NOT NULL CHECK (tipo IN ('curriculo', 'carta')),
    documento_id INTEGER REFERENCES documentos(id) ON DELETE SET NULL,
    anexado_em TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Old CVs/letters already brought into the library (origem = "curriculos/<file>.html"). Deleting the
-- document does not touch this table, so a deleted migration never comes back at the next start.
CREATE TABLE IF NOT EXISTS migrados (
    origem TEXT PRIMARY KEY,
    migrado_em TEXT NOT NULL DEFAULT (datetime('now'))
);
