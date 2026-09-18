pub mod cover_letter;
pub mod welcome;
pub mod updater;
pub mod startup;
pub mod credenciais;
pub mod curriculos;
pub mod estado;
pub mod feedback;
pub mod linkedin;
pub mod notif;
pub mod perfil;
pub mod prompts;
pub mod pty;
pub mod sessao;

/// First candidate path that exists as a regular file.
pub(crate) fn first_existing_claude(candidates: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    candidates.iter().find(|p| p.is_file()).cloned()
}

/// Resolve o executável do Claude Code a invocar.
///
/// No Windows, o npm instala apenas shims (`claude.cmd`, `claude.ps1`) no PATH —
/// não existe `claude.exe` no PATH e o `CreateProcess` do Windows não consegue
/// executar um `.cmd` diretamente (daí o erro "program not found"). O shim aponta
/// para um `claude.exe` nativo dentro do pacote npm; resolvemos esse caminho para
/// o spawnar diretamente — funciona tanto em `std::process` como no PTY, sem
/// precisar do `cmd.exe` nem de escaping especial dos argumentos.
/// Fora do Windows (ou se já houver `claude.exe` no PATH), usa-se `claude`.
pub fn claude_program() -> String {
    #[cfg(windows)]
    {
        use std::path::PathBuf;

        // 1) Localização padrão do npm global: %APPDATA%\npm\node_modules\...
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let p = PathBuf::from(&appdata)
                .join("npm")
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("bin")
                .join("claude.exe");
            if p.exists() {
                return p.to_string_lossy().into_owned();
            }
        }

        // 2) Derivar a partir de um shim `claude.cmd` no PATH, ou um `claude.exe`
        //    já presente diretamente no PATH.
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                if dir.join("claude.cmd").exists() {
                    let exe = dir
                        .join("node_modules")
                        .join("@anthropic-ai")
                        .join("claude-code")
                        .join("bin")
                        .join("claude.exe");
                    if exe.exists() {
                        return exe.to_string_lossy().into_owned();
                    }
                }
                let direct = dir.join("claude.exe");
                if direct.exists() {
                    return direct.to_string_lossy().into_owned();
                }
            }
        }

        // 3) WinGet install (Claude Code native package)
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            let mut winget = vec![
                local.join("Microsoft").join("WinGet").join("Links").join("claude.exe"),
            ];
            let pkg = local
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if let Ok(rd) = std::fs::read_dir(&pkg) {
                for ent in rd.flatten() {
                    let name = ent.file_name();
                    let n = name.to_string_lossy();
                    if n.starts_with("Anthropic.ClaudeCode") {
                        winget.push(ent.path().join("claude.exe"));
                    }
                }
            }
            if let Some(p) = first_existing_claude(&winget) {
                return p.to_string_lossy().into_owned();
            }
        }
    }

    "claude".to_string()
}

#[cfg(test)]
mod tests {
    use super::first_existing_claude;
    use std::path::PathBuf;

    #[test]
    fn first_existing_skips_missing() {
        let missing = PathBuf::from("C:/definitely-not-a-claude-xxxx.exe");
        let cargo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let got = first_existing_claude(&[missing, cargo.clone()]);
        assert_eq!(got.as_ref(), Some(&cargo));
    }

    #[test]
    fn first_existing_none() {
        assert!(first_existing_claude(&[PathBuf::from("C:/nope-a.exe"), PathBuf::from("C:/nope-b.exe")]).is_none());
    }
}
