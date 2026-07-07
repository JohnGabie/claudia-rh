use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Candidatura {
    pub id: i64,
    pub vaga_id: i64,
    pub titulo: String,
    pub empresa: String,
    pub plataforma: String,
    pub url: String,
    pub enviada_em: String,
    pub pasta_arquivos: String,
    pub metodo: String,
    pub resultado: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VagaAtual {
    pub id: i64,
    pub titulo: String,
    pub empresa: String,
    pub url: String,
    pub etapa: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vaga {
    pub id: i64,
    pub titulo: String,
    pub empresa: String,
    pub plataforma: String,
    pub url: String,
    pub localizacao: Option<String>,
    pub modelo_trabalho: Option<String>,
    pub idioma: Option<String>,
    pub descoberta_em: String,
    pub status: String,
    pub motivo_status: Option<String>,
    pub match_score: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pendencia {
    pub id: i64,
    pub vaga_id: i64,
    pub titulo_vaga: String,
    pub empresa_vaga: String,
    pub criada_em: String,
    pub categoria: String,
    pub descricao: String,
    pub resolvida: bool,
    pub resolvida_em: Option<String>,
    pub resolucao: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumoMemoria {
    pub candidaturas_7_dias: i64,
    pub vagas_puladas_recentes: Vec<VagaPulada>,
    pub pendencias_nao_resolvidas: i64,
    pub sessoes_7_dias: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VagaPulada {
    pub titulo: String,
    pub empresa: String,
    pub motivo: Option<String>,
    pub data: String,
}

pub fn listar_vagas(conn: &Connection, filtro: Option<String>) -> Result<Vec<Vaga>> {
    const BASE: &str = "SELECT id,titulo,empresa,plataforma,url,localizacao,modelo_trabalho,idioma,descoberta_em,status,motivo_status,match_score FROM vagas";
    let map_row = |row: &rusqlite::Row<'_>| Ok(Vaga {
        id: row.get(0)?,
        titulo: row.get(1)?,
        empresa: row.get(2)?,
        plataforma: row.get(3)?,
        url: row.get(4)?,
        localizacao: row.get(5)?,
        modelo_trabalho: row.get(6)?,
        idioma: row.get(7)?,
        descoberta_em: row.get(8)?,
        status: row.get(9)?,
        motivo_status: row.get(10)?,
        match_score: row.get(11)?,
    });

    let use_filter = filtro.as_deref().map(|f| !f.is_empty() && f != "todas").unwrap_or(false);

    if use_filter {
        let f = filtro.unwrap();
        let sql = format!("{} WHERE status=? ORDER BY descoberta_em DESC LIMIT 200", BASE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([&f], map_row)?.collect::<Result<Vec<_>>>()?;
        Ok(rows)
    } else {
        let sql = format!("{} ORDER BY descoberta_em DESC LIMIT 200", BASE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], map_row)?.collect::<Result<Vec<_>>>()?;
        Ok(rows)
    }
}

pub fn listar_pendencias(conn: &Connection, apenas_nao_resolvidas: bool) -> Result<Vec<Pendencia>> {
    // LEFT JOIN so pendências inserted by the agent with an unknown vaga_id still appear.
    let sql = if apenas_nao_resolvidas {
        "SELECT p.id, p.vaga_id, COALESCE(v.titulo,'(vaga desconhecida)'), COALESCE(v.empresa,''), p.criada_em, p.categoria, p.descricao, p.resolvida, p.resolvida_em, p.resolucao FROM pendencias p LEFT JOIN vagas v ON v.id=p.vaga_id WHERE p.resolvida=0 ORDER BY p.criada_em ASC"
    } else {
        "SELECT p.id, p.vaga_id, COALESCE(v.titulo,'(vaga desconhecida)'), COALESCE(v.empresa,''), p.criada_em, p.categoria, p.descricao, p.resolvida, p.resolvida_em, p.resolucao FROM pendencias p LEFT JOIN vagas v ON v.id=p.vaga_id ORDER BY p.criada_em DESC LIMIT 100"
    };

    let mut stmt = conn.prepare(sql)?;
    let result = stmt.query_map([], |row| {
        Ok(Pendencia {
            id: row.get(0)?,
            vaga_id: row.get(1)?,
            titulo_vaga: row.get(2)?,
            empresa_vaga: row.get(3)?,
            criada_em: row.get(4)?,
            categoria: row.get(5)?,
            descricao: row.get(6)?,
            resolvida: row.get(7)?,
            resolvida_em: row.get(8)?,
            resolucao: row.get(9)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    Ok(result)
}

pub fn resumo_memoria_recente(conn: &Connection, dias: u32) -> Result<ResumoMemoria> {
    let candidaturas_7_dias: i64 = conn.query_row(
        "SELECT COUNT(*) FROM candidaturas WHERE enviada_em >= datetime('now', ?1)",
        [format!("-{} days", dias)],
        |row| row.get(0),
    )?;

    let pendencias_nao_resolvidas: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pendencias WHERE resolvida=0",
        [],
        |row| row.get(0),
    )?;

    let sessoes_7_dias: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessoes WHERE iniciada_em >= datetime('now', ?1)",
        [format!("-{} days", dias)],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT v.titulo, v.empresa, v.motivo_status, v.descoberta_em FROM vagas v WHERE v.status='pulada' AND v.descoberta_em >= datetime('now', ?1) ORDER BY v.descoberta_em DESC LIMIT 20"
    )?;
    let vagas_puladas = stmt.query_map([format!("-{} days", dias)], |row| {
        Ok(VagaPulada {
            titulo: row.get(0)?,
            empresa: row.get(1)?,
            motivo: row.get(2)?,
            data: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;

    Ok(ResumoMemoria {
        candidaturas_7_dias,
        vagas_puladas_recentes: vagas_puladas,
        pendencias_nao_resolvidas,
        sessoes_7_dias,
    })
}

pub fn contar_pendencias_nao_resolvidas(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM pendencias WHERE resolvida=0",
        [],
        |row| row.get(0),
    )
}

pub fn candidaturas_hoje(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM candidaturas WHERE date(enviada_em)=date('now')",
        [],
        |row| row.get(0),
    )
}

pub fn listar_candidaturas_historico(conn: &Connection) -> Result<Vec<Candidatura>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.vaga_id, v.titulo, v.empresa, v.plataforma, v.url, \
                c.enviada_em, c.pasta_arquivos, c.metodo, c.resultado \
         FROM candidaturas c JOIN vagas v ON v.id = c.vaga_id \
         ORDER BY c.enviada_em DESC LIMIT 200",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Candidatura {
            id: row.get(0)?,
            vaga_id: row.get(1)?,
            titulo: row.get(2)?,
            empresa: row.get(3)?,
            plataforma: row.get(4)?,
            url: row.get(5)?,
            enviada_em: row.get(6)?,
            pasta_arquivos: row.get(7)?,
            metodo: row.get(8)?,
            resultado: row.get(9)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn contar_propostas(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM propostas_perfil WHERE promovida = 0",
        [],
        |row| row.get(0),
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Proposta {
    pub id: i64,
    pub vaga_id: Option<i64>,
    pub titulo_vaga: Option<String>,
    pub empresa_vaga: Option<String>,
    pub criada_em: String,
    pub pergunta: String,
    pub contexto: Option<String>,
}

pub fn listar_propostas(conn: &Connection) -> Result<Vec<Proposta>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.vaga_id, v.titulo, v.empresa, p.criada_em, p.pergunta, p.contexto \
         FROM propostas_perfil p \
         LEFT JOIN vagas v ON p.vaga_id = v.id \
         WHERE p.promovida = 0 \
         ORDER BY p.criada_em DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Proposta {
                id: row.get(0)?,
                vaga_id: row.get(1)?,
                titulo_vaga: row.get(2)?,
                empresa_vaga: row.get(3)?,
                criada_em: row.get(4)?,
                pergunta: row.get(5)?,
                contexto: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn vaga_candidatando(conn: &Connection) -> Result<Option<VagaAtual>> {
    match conn.query_row(
        "SELECT id, titulo, empresa, url, motivo_status FROM vagas \
         WHERE status = 'candidatando' ORDER BY descoberta_em DESC LIMIT 1",
        [],
        |row| {
            Ok(VagaAtual {
                id: row.get(0)?,
                titulo: row.get(1)?,
                empresa: row.get(2)?,
                url: row.get(3)?,
                etapa: row.get(4)?,
            })
        },
    ) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn vagas_analisadas_hoje(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM vagas WHERE date(descoberta_em)=date('now') AND status NOT IN ('descoberta')",
        [],
        |r| r.get(0),
    )
}

pub fn vagas_analisadas_total(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM vagas WHERE status NOT IN ('descoberta')",
        [],
        |r| r.get(0),
    )
}

pub fn tempo_sessoes_hoje_minutos(conn: &Connection) -> Result<f64> {
    conn.query_row(
        "SELECT COALESCE(SUM(
            (julianday(COALESCE(terminada_em, datetime('now'))) - julianday(iniciada_em)) * 1440
            - COALESCE(tempo_pausado_segundos, 0) / 60.0
            - CASE WHEN pausada_em IS NOT NULL AND terminada_em IS NULL
                THEN (julianday(datetime('now')) - julianday(pausada_em)) * 1440
                ELSE 0 END
         ), 0.0) FROM sessoes WHERE date(iniciada_em) = date('now')",
        [],
        |r| r.get(0),
    )
}

pub fn atividade_recente(conn: &Connection) -> Result<Vec<Vaga>> {

    let mut stmt = conn.prepare(
        "SELECT id,titulo,empresa,plataforma,url,localizacao,modelo_trabalho,idioma,descoberta_em,status,motivo_status,match_score FROM vagas ORDER BY descoberta_em DESC LIMIT 8"
    )?;
    let vagas = stmt.query_map([], |row| {
        Ok(Vaga {
            id: row.get(0)?,
            titulo: row.get(1)?,
            empresa: row.get(2)?,
            plataforma: row.get(3)?,
            url: row.get(4)?,
            localizacao: row.get(5)?,
            modelo_trabalho: row.get(6)?,
            idioma: row.get(7)?,
            descoberta_em: row.get(8)?,
            status: row.get(9)?,
            motivo_status: row.get(10)?,
            match_score: row.get(11)?,
        })
    })?
    .collect::<Result<Vec<_>>>()?;
    Ok(vagas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        crate::db::apply(&conn).expect("apply schema");
        conn
    }

    fn insert_vaga(conn: &Connection, titulo: &str, status: &str) -> i64 {
        conn.execute(
            "INSERT INTO vagas (titulo, empresa, plataforma, url, descoberta_em, status) \
             VALUES (?1, 'ACME', 'LinkedIn', ?2, datetime('now'), ?3)",
            rusqlite::params![titulo, format!("https://jobs.test/{titulo}"), status],
        ).unwrap();
        conn.last_insert_rowid()
    }

    fn insert_candidatura(conn: &Connection, vaga_id: i64) {
        conn.execute(
            "INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) \
             VALUES (?1, datetime('now'), '/tmp', 'chrome')",
            [vaga_id],
        ).unwrap();
    }

    fn insert_pendencia(conn: &Connection, vaga_id: i64, resolvida: bool) {
        conn.execute(
            "INSERT INTO pendencias (vaga_id, criada_em, categoria, descricao, resolvida) \
             VALUES (?1, datetime('now'), 'captcha', 'test', ?2)",
            rusqlite::params![vaga_id, resolvida as i32],
        ).unwrap();
    }

    #[test]
    fn listar_vagas_empty() {
        assert!(listar_vagas(&mem(), None).unwrap().is_empty());
    }

    #[test]
    fn listar_vagas_sem_filtro_retorna_todos() {
        let conn = mem();
        insert_vaga(&conn, "Dev A", "aplicada");
        insert_vaga(&conn, "Dev B", "pulada");
        assert_eq!(listar_vagas(&conn, None).unwrap().len(), 2);
        assert_eq!(listar_vagas(&conn, Some("".into())).unwrap().len(), 2);
        assert_eq!(listar_vagas(&conn, Some("todas".into())).unwrap().len(), 2);
    }

    #[test]
    fn listar_vagas_filtra_por_status() {
        let conn = mem();
        insert_vaga(&conn, "Dev A", "aplicada");
        insert_vaga(&conn, "Dev B", "pulada");
        insert_vaga(&conn, "Dev C", "aplicada");
        let aplicadas = listar_vagas(&conn, Some("aplicada".into())).unwrap();
        assert_eq!(aplicadas.len(), 2);
        assert!(aplicadas.iter().all(|v| v.status == "aplicada"));
    }

    #[test]
    fn listar_pendencias_filtro_nao_resolvidas() {
        let conn = mem();
        let vid = insert_vaga(&conn, "Dev", "analisada");
        insert_pendencia(&conn, vid, false);
        insert_pendencia(&conn, vid, true);

        let abertas = listar_pendencias(&conn, true).unwrap();
        assert_eq!(abertas.len(), 1);
        assert!(!abertas[0].resolvida);

        let todas = listar_pendencias(&conn, false).unwrap();
        assert_eq!(todas.len(), 2);
    }

    #[test]
    fn listar_pendencias_left_join_mostra_vaga_desconhecida() {
        let conn = mem();
        let vid = insert_vaga(&conn, "Dev", "analisada");
        insert_pendencia(&conn, vid, false);

        // Disable FK to insert a pendência with a non-existent vaga_id,
        // replicating what the agent can do when it writes directly to the DB.
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        conn.execute(
            "INSERT INTO pendencias (vaga_id, criada_em, categoria, descricao, resolvida) \
             VALUES (9999, datetime('now'), 'captcha', 'orphan', 0)",
            [],
        ).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();

        let pendencias = listar_pendencias(&conn, true).unwrap();
        assert_eq!(pendencias.len(), 2);
        let orphan = pendencias.iter().find(|p| p.vaga_id == 9999).expect("orphan should appear");
        assert_eq!(orphan.titulo_vaga, "(vaga desconhecida)");
        assert_eq!(orphan.empresa_vaga, "");
    }

    #[test]
    fn pendencias_abertas_sao_contadas_corretamente() {
        let conn = mem();
        let vid = insert_vaga(&conn, "Dev", "analisada");
        insert_pendencia(&conn, vid, false);
        insert_pendencia(&conn, vid, false);
        insert_pendencia(&conn, vid, true);
        assert_eq!(contar_pendencias_nao_resolvidas(&conn).unwrap(), 2);
    }

    #[test]
    fn candidaturas_hoje_ignora_antigas() {
        let conn = mem();
        let vid = insert_vaga(&conn, "Dev", "aplicada");
        insert_candidatura(&conn, vid);
        conn.execute(
            "INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) \
             VALUES (?1, datetime('now', '-2 days'), '/tmp', 'chrome')",
            [vid],
        ).unwrap();
        assert_eq!(candidaturas_hoje(&conn).unwrap(), 1);
    }

    #[test]
    fn contar_propostas_ignora_promovidas() {
        let conn = mem();
        let vid = insert_vaga(&conn, "Dev", "analisada");
        conn.execute(
            "INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, promovida) \
             VALUES (?1, datetime('now'), 'questao 1', 0)",
            [vid],
        ).unwrap();
        conn.execute(
            "INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, promovida) \
             VALUES (?1, datetime('now'), 'questao 2', 1)",
            [vid],
        ).unwrap();
        assert_eq!(contar_propostas(&conn).unwrap(), 1);
    }

    #[test]
    fn vaga_candidatando_none_quando_vazia() {
        assert!(vaga_candidatando(&mem()).unwrap().is_none());
    }

    #[test]
    fn vaga_candidatando_retorna_vaga_ativa() {
        let conn = mem();
        let vid = insert_vaga(&conn, "Rust Engineer", "candidatando");
        let atual = vaga_candidatando(&conn).unwrap().expect("must find active job");
        assert_eq!(atual.id, vid);
        assert_eq!(atual.titulo, "Rust Engineer");
    }

    #[test]
    fn resumo_memoria_recente_zero_na_db_vazia() {
        let resumo = resumo_memoria_recente(&mem(), 7).unwrap();
        assert_eq!(resumo.candidaturas_7_dias, 0);
        assert_eq!(resumo.pendencias_nao_resolvidas, 0);
        assert_eq!(resumo.sessoes_7_dias, 0);
        assert!(resumo.vagas_puladas_recentes.is_empty());
    }
}
