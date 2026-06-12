// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod core;
mod git;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // vault
            commands::pick_and_open_vault,
            commands::open_vault,
            commands::get_last_vault,
            // file tree
            commands::list_directory,
            // search
            commands::search_notes,
            // note CRUD
            commands::read_note,
            commands::write_note,
            commands::create_note,
            commands::delete_note,
            // knowledge graph
            commands::get_backlinks,
            commands::get_graph,
            // git
            commands::sync,
            // note history
            commands::get_note_history,
            commands::get_note_at_commit,
            commands::restore_note_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
