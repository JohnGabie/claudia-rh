//! Validation for the "open folder" button. A path saved by the agent (register_candidatura) is
//! untrusted: it must never reach `explorer` unless it is an existing FOLDER inside the app data dir.
//! The core is pure: filesystem access (canonicalize / is-dir) is injected, so tests use invented paths.
use std::io;
use std::path::{Component, Path, PathBuf};

/// Pure check. `canon` resolves links and returns the real path; `eh_pasta` says if it is a directory.
/// Returns the VALIDATED (canonical) path, which is the only thing that may be opened.
pub fn pasta_permitida_com(
    caminho: &str,
    raiz_dados: &Path,
    canon: impl Fn(&Path) -> io::Result<PathBuf>,
    eh_pasta: impl Fn(&Path) -> bool,
) -> Result<PathBuf, &'static str> {
    let texto = caminho.trim();
    if texto.is_empty() {
        return Err("Caminho vazio.");
    }
    // Network and device paths (UNC, \\?\, \\.\, //host) are refused before touching the disk.
    if texto.starts_with("\\\\") || texto.starts_with("//") {
        return Err("Caminhos de rede ou de dispositivo não são permitidos.");
    }
    let bruto = Path::new(texto);
    if !bruto.is_absolute() {
        return Err("O caminho precisa ser absoluto.");
    }
    if bruto.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err("O caminho não pode conter '.' ou '..'.");
    }
    let real = canon(bruto).map_err(|_| "A pasta não existe.")?;
    let raiz = canon(raiz_dados).map_err(|_| "A pasta de dados do app não está acessível.")?;
    // A disk root (or a root with no folder name) is never acceptable, on either side.
    if !tem_nome(&real) || !tem_nome(&raiz) {
        return Err("Raiz de disco não é permitida.");
    }
    // Component-wise: `<dados>-x` is not inside `<dados>`.
    if !real.starts_with(&raiz) {
        return Err("A pasta está fora da pasta de dados do app.");
    }
    if !eh_pasta(&real) {
        return Err("O caminho não é uma pasta.");
    }
    Ok(real)
}

fn tem_nome(p: &Path) -> bool {
    p.components().any(|c| matches!(c, Component::Normal(_)))
}

/// Real-disk wrapper.
pub fn pasta_permitida(caminho: &str, raiz_dados: &Path) -> Result<PathBuf, &'static str> {
    pasta_permitida_com(caminho, raiz_dados, |p| std::fs::canonicalize(p), |p| p.is_dir())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Invented paths only; nothing here touches the disk.
    const RAIZ: &str = r"Y:\inventado\dados";

    /// Fake canonicalize: known paths map to a real target (so links can point outside).
    fn fake(mapa: &'static [(&'static str, &'static str)]) -> impl Fn(&Path) -> io::Result<PathBuf> {
        let m: HashMap<String, String> = mapa.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        move |p: &Path| {
            m.get(&p.to_string_lossy().to_string())
                .map(PathBuf::from)
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }
    // Every path that a rule must refuse is ALSO mapped here to an existing folder inside the data
    // dir, so that without that rule it would be accepted (and the test catches the missing rule
    // instead of falling through to "A pasta não existe.").
    const MAPA: &[(&str, &str)] = &[
        (r"Y:\inventado\dados", r"Y:\inventado\dados"),
        (r"Y:\inventado\dados\vaga1", r"Y:\inventado\dados\vaga1"),
        (r"Y:\inventado\dados\vaga1\cv.exe", r"Y:\inventado\dados\vaga1\cv.exe"),
        (r"Y:\inventado\dados-x\vaga1", r"Y:\inventado\dados-x\vaga1"),
        (r"Y:\inventado\dados\link", r"Y:\outro\lugar"),
        (r"Y:\", r"Y:\"),
        // network / device forms
        (r"\\servidor\share\x", r"Y:\inventado\dados\vaga1"),
        (r"\\?\Y:\inventado\dados\vaga1", r"Y:\inventado\dados\vaga1"),
        (r"\\.\Y:\inventado\dados\vaga1", r"Y:\inventado\dados\vaga1"),
        ("//servidor/share", r"Y:\inventado\dados\vaga1"),
        // relative and drive-relative forms
        ("vaga1", r"Y:\inventado\dados\vaga1"),
        (r"Y:inventado\dados\vaga1", r"Y:\inventado\dados\vaga1"),
        // '..' forms
        (r"Y:\inventado\dados\vaga1\..", r"Y:\inventado\dados"),
        (r"Y:\inventado\dados\vaga1\..\..\..", r"Y:\inventado\dados\vaga1"),
    ];
    fn so_pastas(p: &Path) -> bool {
        !p.to_string_lossy().ends_with(".exe")
    }
    fn run(c: &str) -> Result<PathBuf, &'static str> {
        pasta_permitida_com(c, Path::new(RAIZ), fake(MAPA), so_pastas)
    }
    /// Refused, and by the expected rule (exact message).
    fn recusa(c: &str, motivo: &'static str) {
        assert_eq!(run(c), Err(motivo), "path {c:?}");
    }

    const VAZIO: &str = "Caminho vazio.";
    const REDE: &str = "Caminhos de rede ou de dispositivo não são permitidos.";
    const RELATIVO: &str = "O caminho precisa ser absoluto.";
    const PONTO: &str = "O caminho não pode conter '.' ou '..'.";
    const NAO_EXISTE: &str = "A pasta não existe.";
    const RAIZ_DISCO: &str = "Raiz de disco não é permitida.";
    const FORA: &str = "A pasta está fora da pasta de dados do app.";
    const NAO_PASTA: &str = "O caminho não é uma pasta.";

    #[test]
    fn aceita_pasta_dentro() {
        assert_eq!(run(r"Y:\inventado\dados\vaga1").unwrap(), PathBuf::from(r"Y:\inventado\dados\vaga1"));
    }
    #[test]
    fn aceita_a_propria_raiz_de_dados() {
        assert_eq!(run(RAIZ).unwrap(), PathBuf::from(RAIZ));
    }
    #[test]
    fn recusa_arquivo_exe() {
        recusa(r"Y:\inventado\dados\vaga1\cv.exe", NAO_PASTA);
    }
    #[test]
    fn recusa_prefixo_de_texto() {
        recusa(r"Y:\inventado\dados-x\vaga1", FORA);
    }
    #[test]
    fn recusa_link_para_fora() {
        recusa(r"Y:\inventado\dados\link", FORA);
    }
    #[test]
    fn recusa_unc_e_verbatim() {
        recusa(r"\\servidor\share\x", REDE);
        recusa(r"\\?\Y:\inventado\dados\vaga1", REDE);
        recusa(r"\\.\Y:\inventado\dados\vaga1", REDE);
        recusa("//servidor/share", REDE);
    }
    #[test]
    fn recusa_relativo_vazio_e_ponto_ponto() {
        recusa("", VAZIO);
        recusa("   ", VAZIO);
        recusa("vaga1", RELATIVO);
        recusa(r"Y:inventado\dados\vaga1", RELATIVO);
        recusa(r"Y:\inventado\dados\vaga1\..", PONTO);
        recusa(r"Y:\inventado\dados\vaga1\..\..\..", PONTO);
    }
    #[test]
    fn recusa_raiz_de_disco() {
        recusa(r"Y:\", RAIZ_DISCO);
        // a data dir that is itself a disk root is also refused, even for a folder inside it
        for c in [r"Y:\", r"Y:\inventado\dados\vaga1"] {
            let r = pasta_permitida_com(c, Path::new(r"Y:\"), fake(MAPA), so_pastas);
            assert_eq!(r, Err(RAIZ_DISCO), "path {c:?}");
        }
    }
    #[test]
    fn recusa_inexistente() {
        recusa(r"Y:\inventado\dados\nao_existe", NAO_EXISTE);
    }
}
