mod database;
mod manifest;
mod detection;
mod auto_backup;
mod game_manager;
mod launch_utils;
mod git_manager;
mod pcgaming_wiki;
mod commands;

use crate::database::connection::{EncryptedDatabase, DatabasePaths};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize database at startup - app waits for this to complete
    println!("Initializing database...");
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        let db_path = DatabasePaths::database_file();
        let db = EncryptedDatabase::new(&db_path, "default_password")
            .await
            .expect("Failed to connect to database - cannot start application");

        db.initialize_database()
            .await
            .expect("Failed to initialize database schema - cannot start application");

        println!("Database initialization complete");
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::system::greet,
            commands::system::identify_game_by_pid,
            commands::system::scan_running_games,
            commands::system::launch_game,
            commands::game::add_manual_game,
            commands::game::add_manual_game_sync,
            commands::game::get_all_games,
            commands::game::update_game_sync,
            commands::game::delete_game_sync,
            commands::game::search_pcgw_games,
            commands::game::get_pcgw_save_locations,
            commands::game::detect_game_executable,
            commands::git::enable_git_for_game,
            commands::git::create_save_checkpoint,
            commands::git::create_save_branch,
            commands::git::switch_save_branch,
            commands::git::restore_to_commit,
            commands::git::restore_to_timestamp,
            commands::git::get_git_history,
            commands::git::sync_to_cloud,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
