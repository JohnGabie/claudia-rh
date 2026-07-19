// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Headless MCP modes (--mcp-serve / --mcp-call) never start the GUI.
    if claudia_rh_lib::mcp::cli_main() {
        return;
    }
    claudia_rh_lib::run()
}
