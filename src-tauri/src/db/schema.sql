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
    metodo TEXT NOT NULL
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
