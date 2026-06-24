pub mod cover_letter;
pub mod credenciais;
pub mod curriculos;
pub mod estado;
pub mod feedback;
pub mod notif;
pub mod perfil;
pub mod prompts;
pub mod pty;
pub mod sessao;

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
    }

    // Fallback (não-Windows, ou Windows com claude.exe já resolúvel pelo PATH).
    "claude".to_string()
}
