//! The vocabulary of the site list and the validation of the seed file.
use super::dominio::{host_and_path, host_matches, valid_domain};
use serde::Deserialize;
use std::collections::HashSet;

pub const TIPOS: &[&str] = &["geral", "regional", "tecnologia", "remoto", "freelance", "empresa", "nicho"];
pub const REGIOES: &[&str] = &["SA", "NA", "EU", "AS", "AF", "OC", "GLOBAL"];
pub const PAPEIS: &[&str] = &["candidatura", "descoberta"];
pub const REGRAS: &[&str] = &["manual", "cookie_sessao", "nenhuma"];

/// Multi-label public suffixes that must never be a site domain (matching one would match every site under it).
const PUBLIC_SUFFIXES: &[&str] = &[
    "co.uk", "com.br", "com.au", "org.uk", "co.nz", "com.mx", "co.jp", "co.kr", "com.ar", "com.co",
];

fn papel_padrao() -> String { "candidatura".into() }
fn regra_padrao() -> String { "manual".into() }

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SeedSite {
    pub chave: String,
    pub nome: String,
    pub url_login: String,
    pub dominios: Vec<String>,
    #[serde(default)]
    pub dominios_ats: Vec<String>,
    #[serde(default)]
    pub caminho_ats: Option<String>,
    pub tipo: String,
    pub regiao: String,
    #[serde(default)]
    pub pais: Option<String>,
    #[serde(default = "papel_padrao")]
    pub papel: String,
    #[serde(default = "regra_padrao")]
    pub regra_login: String,
}

#[derive(Deserialize)]
pub(super) struct SeedFile {
    #[allow(dead_code)]
    pub versao: u32,
    pub sites: Vec<SeedSite>,
}

fn key_ok(k: &str) -> bool {
    !k.is_empty()
        && k.chars().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Checks one seed entry. The message always starts with the site key so a bad entry is easy to find.
pub fn validate_site(s: &SeedSite) -> Result<(), String> {
    let e = |m: &str| Err(format!("{}: {m}", s.chave));
    if !key_ok(&s.chave) {
        return e("chave inválida (minúsculas, números e hífen)");
    }
    if s.nome.trim().is_empty() {
        return e("nome vazio");
    }
    if !s.url_login.to_ascii_lowercase().starts_with("https://") {
        return e("url_login precisa ser https");
    }
    let Some((host, _)) = host_and_path(&s.url_login) else { return e("url_login ilegível") };
    if s.dominios.is_empty() {
        return e("dominios vazio");
    }
    if let Some(d) = s.dominios.iter().chain(&s.dominios_ats).find(|d| !valid_domain(d)) {
        return e(&format!("domínio inválido '{d}' (sem esquema, sem caminho, minúsculas)"));
    }
    if let Some(d) = s.dominios.iter().chain(&s.dominios_ats).find(|d| PUBLIC_SUFFIXES.contains(&d.as_str())) {
        return e(&format!("domínio '{d}' é um sufixo público"));
    }
    if !s.dominios.iter().chain(&s.dominios_ats).any(|d| host_matches(&host, d)) {
        return e("url_login não pertence a nenhum dos domínios");
    }
    if let Some(p) = &s.caminho_ats {
        if s.dominios_ats.is_empty() {
            return e("caminho_ats exige dominios_ats");
        }
        if !p.starts_with('/') {
            return e("caminho_ats precisa começar com /");
        }
    }
    if !TIPOS.contains(&s.tipo.as_str()) { return e("tipo desconhecido"); }
    if !REGIOES.contains(&s.regiao.as_str()) { return e("regiao desconhecida"); }
    if !PAPEIS.contains(&s.papel.as_str()) { return e("papel desconhecido"); }
    if !REGRAS.contains(&s.regra_login.as_str()) { return e("regra_login desconhecida"); }
    if let Some(p) = &s.pais {
        if p.len() != 2 || !p.chars().all(|c| c.is_ascii_uppercase()) {
            return e("pais precisa ser código ISO de 2 letras maiúsculas");
        }
    }
    Ok(())
}

pub fn validate_all(sites: &[SeedSite]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for s in sites {
        if !seen.insert(s.chave.as_str()) {
            return Err(format!("{}: chave repetida", s.chave));
        }
        validate_site(s)?;
    }
    Ok(())
}
