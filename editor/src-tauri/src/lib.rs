mod project;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            project::default_projects_dir,
            project::list_recent_projects,
            project::forget_recent_project,
            project::create_project,
            project::open_project,
            project::load_scene,
            project::save_scene,
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o editor Nexo");
}
