mod assets;
mod logic;
mod project;
mod settings;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(assets::Watch::default())
        .invoke_handler(tauri::generate_handler![
            project::default_projects_dir,
            project::list_recent_projects,
            project::forget_recent_project,
            project::create_project,
            project::open_project,
            project::load_scene,
            project::save_scene,
            settings::load_settings,
            settings::save_settings,
            logic::list_scripts,
            logic::load_script,
            logic::save_script,
            logic::create_script,
            logic::rename_script,
            logic::duplicate_script,
            logic::delete_script,
            assets::list_dir,
            assets::list_assets,
            assets::read_file,
            assets::write_text,
            assets::create_folder,
            assets::create_scene,
            assets::rename_entry,
            assets::trash_entry,
            assets::import_files,
            assets::ensure_imports,
            assets::watch_project,
            assets::stop_watch,
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o editor Nexo");
}
