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
    let sql = if let Some(ref f) = filtro {
        if f.is_empty() || f == "todas" {
            "SELECT id,titulo,empresa,plataforma,url,localizacao,modelo_trabalho,idioma,descoberta_em,status,motivo_status,match_score FROM vagas ORDER BY descoberta_em DESC LIMIT 200".to_string()
        } else {
            format!("SELECT id,titulo,empresa,plataforma,url,localizacao,modelo_trabalho,idioma,descoberta_em,status,motivo_status,match_score FROM vagas WHERE status='{}' ORDER BY descoberta_em DESC LIMIT 200", f)
        }
    } else {
        "SELECT id,titulo,empresa,plataforma,url,localizacao,modelo_trabalho,idioma,descoberta_em,status,motivo_status,match_score FROM vagas ORDER BY descoberta_em DESC LIMIT 200".to_string()
    };

    let mut stmt = conn.prepare(&sql)?;
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
