//! Projetos Nexo no disco: criação, abertura, recentes e cenas.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const PROJECT_FILE: &str = "projeto.nexo";
const MAIN_SCENE: &str = "cenas/principal.scene.ron";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub template: String,
    pub engine_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub name: String,
    pub path: String,
    pub template: String,
    pub opened_at: u64,
}

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    projeto: ProjectSection,
}

#[derive(Serialize, Deserialize)]
struct ProjectSection {
    nome: String,
    modelo: String,
    versao_engine: String,
    resolucao: [u32; 2],
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn recents_file(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(err)?;
    fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir.join("recentes.json"))
}

fn read_recents(app: &AppHandle) -> Vec<RecentProject> {
    recents_file(app)
        .ok()
        .and_then(|f| fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_recents(app: &AppHandle, list: &[RecentProject]) -> CmdResult<()> {
    let f = recents_file(app)?;
    fs::write(f, serde_json::to_string_pretty(list).map_err(err)?).map_err(err)
}

fn remember(app: &AppHandle, info: &ProjectInfo) -> CmdResult<()> {
    let mut list = read_recents(app);
    list.retain(|r| r.path != info.path);
    list.insert(
        0,
        RecentProject {
            name: info.name.clone(),
            path: info.path.clone(),
            template: info.template.clone(),
            opened_at: now(),
        },
    );
    list.truncate(24);
    write_recents(app, &list)
}

fn read_project(dir: &Path) -> CmdResult<ProjectInfo> {
    let text = fs::read_to_string(dir.join(PROJECT_FILE))
        .map_err(|_| format!("{} não é um projeto Nexo (falta {PROJECT_FILE})", dir.display()))?;
    let file: ProjectFile = toml::from_str(&text).map_err(err)?;
    Ok(ProjectInfo {
        name: file.projeto.nome,
        path: dir.to_string_lossy().into_owned(),
        template: file.projeto.modelo,
        engine_version: file.projeto.versao_engine,
    })
}

#[tauri::command]
pub fn default_projects_dir() -> CmdResult<String> {
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or("pasta de documentos não encontrada")?;
    Ok(base.join("Nexo Projects").to_string_lossy().into_owned())
}

#[tauri::command]
pub fn list_recent_projects(app: AppHandle) -> Vec<RecentProject> {
    read_recents(&app)
        .into_iter()
        .filter(|r| Path::new(&r.path).join(PROJECT_FILE).exists())
        .collect()
}

#[tauri::command]
pub fn forget_recent_project(app: AppHandle, path: String) -> CmdResult<()> {
    let mut list = read_recents(&app);
    list.retain(|r| r.path != path);
    write_recents(&app, &list)
}

#[tauri::command]
pub fn create_project(
    app: AppHandle,
    name: String,
    parent: String,
    template: String,
    scene: serde_json::Value,
) -> CmdResult<ProjectInfo> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Dê um nome ao projeto".into());
    }
    let dir = Path::new(&parent).join(name);
    if dir.exists() && fs::read_dir(&dir).map_err(err)?.next().is_some() {
        return Err(format!("A pasta {} já existe e não está vazia", dir.display()));
    }
    for sub in ["cenas", "logica", "assets", ".nexo/cache"] {
        fs::create_dir_all(dir.join(sub)).map_err(err)?;
    }
    let file = ProjectFile {
        projeto: ProjectSection {
            nome: name.to_string(),
            modelo: template.clone(),
            versao_engine: env!("CARGO_PKG_VERSION").to_string(),
            resolucao: [1280, 720],
        },
    };
    fs::write(dir.join(PROJECT_FILE), toml::to_string_pretty(&file).map_err(err)?).map_err(err)?;
    fs::write(dir.join(".gitignore"), ".nexo/cache/\n").map_err(err)?;
    write_scene(&dir, &scene)?;

    let info = read_project(&dir)?;
    remember(&app, &info)?;
    Ok(info)
}

#[tauri::command]
pub fn open_project(app: AppHandle, path: String) -> CmdResult<ProjectInfo> {
    let info = read_project(Path::new(&path))?;
    remember(&app, &info)?;
    Ok(info)
}

fn write_scene(dir: &Path, scene: &serde_json::Value) -> CmdResult<()> {
    let cfg = ron::ser::PrettyConfig::new().struct_names(false);
    let text = ron::ser::to_string_pretty(scene, cfg).map_err(err)?;
    let target = dir.join(MAIN_SCENE);
    // Grava num arquivo temporário e troca, para nunca deixar a cena pela metade.
    let tmp = target.with_extension("ron.tmp");
    fs::write(&tmp, text).map_err(err)?;
    fs::rename(&tmp, &target).map_err(err)
}

#[tauri::command]
pub fn load_scene(path: String) -> CmdResult<serde_json::Value> {
    let text = fs::read_to_string(Path::new(&path).join(MAIN_SCENE)).map_err(err)?;
    ron::from_str(&text).map_err(err)
}

#[tauri::command]
pub fn save_scene(path: String, scene: serde_json::Value) -> CmdResult<()> {
    write_scene(Path::new(&path), &scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cena_sobrevive_ida_e_volta_em_ron() {
        let dir = std::env::temp_dir().join(format!("nexo-test-{}", now()));
        fs::create_dir_all(dir.join("cenas")).unwrap();
        let scene = serde_json::json!({
            "version": 1,
            "name": "Principal",
            "screen": [1280, 720],
            "objects": [{
                "id": "a1", "name": "Moeda \"dourada\"", "shape": 2,
                "x": 380, "y": 436.5, "w": 34, "h": 34, "rotation": -12.25,
                "color": "#ffd84d", "visible": true, "locked": false,
                "behaviors": [{ "kind": "girar", "speed": 120 }, { "kind": "flutuar", "speed": 0.8 }]
            }]
        });
        write_scene(&dir, &scene).unwrap();
        let back = load_scene(dir.to_string_lossy().into_owned()).unwrap();
        fs::remove_dir_all(&dir).ok();
        assert_eq!(back, scene);
    }
}
