//! Projetos Nexo no disco: criação, abertura, migração de layout, recentes e cenas.
//! Layout atual (SCN-001): `project.nexo` e `scenes/principal.scene.ron`. O layout da Fase 0
//! (`projeto.nexo` e `cenas/`) é migrado sozinho ao abrir o projeto, sem apagar nada.

use nexo_core::{ProjectSettings, Scene, seed_now};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub(crate) const PROJECT_FILE: &str = "project.nexo";
pub(crate) const LEGACY_PROJECT_FILE: &str = "projeto.nexo";
pub(crate) const SCENE_DIR: &str = "scenes";
const LEGACY_SCENE_DIR: &str = "cenas";
const SCENE_FILE: &str = "principal.scene.ron";
/// Cena principal, relativa à pasta do projeto.
const MAIN_SCENE: &str = "scenes/principal.scene.ron";

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

/// Verdadeiro se a pasta tem arquivo de projeto, no nome atual ou no antigo.
pub(crate) fn project_file_exists(dir: &Path) -> bool {
    dir.join(PROJECT_FILE).is_file() || dir.join(LEGACY_PROJECT_FILE).is_file()
}

fn read_project(dir: &Path) -> CmdResult<ProjectInfo> {
    let file = [PROJECT_FILE, LEGACY_PROJECT_FILE]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            format!(
                "{} não é um projeto Nexo (falta {PROJECT_FILE})",
                dir.display()
            )
        })?;
    let text = fs::read_to_string(&file).map_err(err)?;
    let parsed: ProjectFile = toml::from_str(&text).map_err(err)?;
    Ok(ProjectInfo {
        name: parsed.projeto.nome,
        path: dir.to_string_lossy().into_owned(),
        template: parsed.projeto.modelo,
        engine_version: parsed.projeto.versao_engine,
    })
}

/// Grava num arquivo temporário oculto e troca pelo destino: nunca deixa o arquivo pela metade.
pub(crate) fn write_atomic(target: &Path, bytes: &[u8]) -> CmdResult<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(err)?;
    }
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arquivo");
    let tmp = target.with_file_name(format!(".{name}.tmp"));
    fs::write(&tmp, bytes).map_err(err)?;
    if let Err(e) = fs::rename(&tmp, target) {
        let _ = fs::remove_file(&tmp);
        return Err(err(e));
    }
    Ok(())
}

/// Formato 2 é RON com o campo `format`. O formato 1 é JSON, ou RON sem esse campo.
fn is_format_2(text: &str) -> bool {
    #[derive(Deserialize)]
    struct Probe {
        format: u32,
    }
    let t = text.trim_start();
    !t.starts_with('{')
        && ron::from_str::<Probe>(t)
            .map(|p| p.format == 2)
            .unwrap_or(false)
}

/// Migra o layout antigo para o atual (SCN-001). Pode rodar a cada abertura: sem efeito quando já está em dia.
/// Renomeia `projeto.nexo` e `cenas/` e reescreve a cena principal no formato 2. Antes de reescrever,
/// copia o original para `scenes/principal.scene.ron.v1.bak` (sem sobrescrever um backup existente).
/// Nunca apaga arquivos do usuário.
fn migrate_layout(dir: &Path, seed: u64) -> CmdResult<()> {
    let legacy_project = dir.join(LEGACY_PROJECT_FILE);
    if legacy_project.is_file() && !dir.join(PROJECT_FILE).exists() {
        fs::rename(&legacy_project, dir.join(PROJECT_FILE)).map_err(err)?;
    }
    let legacy_dir = dir.join(LEGACY_SCENE_DIR);
    let scene_dir = dir.join(SCENE_DIR);
    if legacy_dir.is_dir() && !scene_dir.exists() {
        fs::rename(&legacy_dir, &scene_dir).map_err(err)?;
    }

    let target = dir.join(MAIN_SCENE);
    if !target.is_file() {
        // Só chega aqui quando as duas pastas coexistem: copia a cena antiga, sem mexer no original.
        let legacy_scene = legacy_dir.join(SCENE_FILE);
        if !legacy_scene.is_file() {
            return Ok(());
        }
        fs::create_dir_all(&scene_dir).map_err(err)?;
        fs::copy(&legacy_scene, &target).map_err(err)?;
    }

    let text = fs::read_to_string(&target).map_err(err)?;
    if is_format_2(&text) {
        return Ok(());
    }
    let scene = Scene::from_text(&text, seed)
        .map_err(|e| format!("Não foi possível migrar a cena: {e}"))?;
    let backup = scene_dir.join(format!("{SCENE_FILE}.v1.bak"));
    if !backup.exists() {
        fs::copy(&target, &backup).map_err(err)?;
    }
    write_atomic(&target, scene.to_ron()?.as_bytes())
}

/// Cria a estrutura do projeto em `dir`. A cena é validada antes de qualquer arquivo ser criado.
fn scaffold(
    dir: &Path,
    name: &str,
    template: &str,
    scene: &serde_json::Value,
) -> CmdResult<ProjectInfo> {
    let text = Scene::from_value(scene, seed_now())?.to_ron()?;
    for sub in [SCENE_DIR, "logica", "assets", ".nexo/cache"] {
        fs::create_dir_all(dir.join(sub)).map_err(err)?;
    }
    let file = ProjectFile {
        projeto: ProjectSection {
            nome: name.to_string(),
            modelo: template.to_string(),
            versao_engine: env!("CARGO_PKG_VERSION").to_string(),
            resolucao: [1280, 720],
        },
    };
    fs::write(
        dir.join(PROJECT_FILE),
        toml::to_string_pretty(&file).map_err(err)?,
    )
    .map_err(err)?;
    fs::write(dir.join(".gitignore"), ".nexo/cache/\n").map_err(err)?;
    write_atomic(&dir.join(MAIN_SCENE), text.as_bytes())?;
    // Projeto novo grava os padrões (INP-002): as ações ui_* e os botões do controle ficam no settings.ron.
    let settings = ProjectSettings::default().to_ron()?;
    write_atomic(&dir.join(crate::settings::SETTINGS_FILE), settings.as_bytes())?;
    read_project(dir)
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
        .filter(|r| project_file_exists(Path::new(&r.path)))
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
        return Err(format!(
            "A pasta {} já existe e não está vazia",
            dir.display()
        ));
    }
    let info = scaffold(&dir, name, &template, &scene)?;
    remember(&app, &info)?;
    Ok(info)
}

#[tauri::command]
pub fn open_project(app: AppHandle, path: String) -> CmdResult<ProjectInfo> {
    let dir = PathBuf::from(&path);
    if !project_file_exists(&dir) {
        return Err(format!(
            "{path} não é um projeto Nexo (falta {PROJECT_FILE})"
        ));
    }
    migrate_layout(&dir, seed_now())?;
    let info = read_project(&dir)?;
    remember(&app, &info)?;
    Ok(info)
}

/// Texto RON da cena principal, como está no disco. O frontend interpreta com o editor WASM.
#[tauri::command]
pub fn load_scene(path: String) -> CmdResult<String> {
    let dir = PathBuf::from(&path);
    if !dir.join(MAIN_SCENE).is_file() {
        // Projeto que não passou pela abertura (ou ainda no layout antigo): migra antes de ler.
        migrate_layout(&dir, seed_now())?;
    }
    fs::read_to_string(dir.join(MAIN_SCENE))
        .map_err(|e| format!("Não foi possível ler a cena principal: {e}"))
}

/// Grava o texto da cena principal. Recusa texto que não é uma cena, para não estragar o arquivo.
#[tauri::command]
pub fn save_scene(path: String, text: String) -> CmdResult<()> {
    Scene::from_text(&text, seed_now()).map_err(|e| format!("A cena não foi salva: {e}"))?;
    write_atomic(&Path::new(&path).join(MAIN_SCENE), text.as_bytes())
}

/// Resultado de `rename_prop_in_scenes`: as cenas do disco que mudaram e as que não puderam ser lidas ou gravadas.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneRenameReport {
    pub changed: Vec<String>,
    pub failed: Vec<String>,
}

/// Caminhos relativos (com `/`) de todas as cenas do projeto, em qualquer pasta. Itens ocultos e links
/// simbólicos ficam de fora: a troca nunca escreve fora da pasta do projeto.
fn scene_files(dir: &Path, rel: &str, out: &mut Vec<String>) -> CmdResult<()> {
    for entry in fs::read_dir(dir).map_err(err)? {
        let entry = entry.map_err(err)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        let kind = entry.file_type().map_err(err)?;
        if kind.is_dir() {
            scene_files(&entry.path(), &child, out)?;
        } else if kind.is_file() && name.to_lowercase().ends_with(".scene.ron") {
            out.push(child);
        }
    }
    Ok(())
}

/// Troca `from` por `to` na propriedade `key` de uma cena do disco. Só regrava o arquivo se algum nó mudou.
fn rename_in_scene_file(path: &Path, key: &str, from: &str, to: &str) -> CmdResult<bool> {
    let text = fs::read_to_string(path).map_err(err)?;
    let mut scene = Scene::from_text(&text, seed_now())?;
    if scene.rename_prop_value(key, from, to)? == 0 {
        return Ok(false);
    }
    write_atomic(path, scene.to_ron()?.as_bytes())?;
    Ok(true)
}

/// Troca um nome de referência (PHY-023: o material de física) nas cenas do projeto gravadas no disco.
/// `skip` é o caminho relativo da cena aberta: o editor a atualiza na memória e a grava pelo autosave, então
/// o disco dela não é mexido aqui. Cena que não pode ser lida ou gravada vai para `failed`; as outras seguem.
#[tauri::command]
pub fn rename_prop_in_scenes(
    project: String,
    skip: String,
    key: String,
    from: String,
    to: String,
) -> CmdResult<SceneRenameReport> {
    let dir = PathBuf::from(&project);
    if !project_file_exists(&dir) {
        return Err(format!("{project} não é um projeto Nexo"));
    }
    let skip = skip.replace('\\', "/");
    let mut files = Vec::new();
    scene_files(&dir, "", &mut files)?;
    let mut report = SceneRenameReport::default();
    for rel in files.into_iter().filter(|rel| *rel != skip) {
        match rename_in_scene_file(&dir.join(&rel), &key, &from, &to) {
            Ok(true) => report.changed.push(rel),
            Ok(false) => {}
            Err(e) => report.failed.push(format!("{rel}: {e}")),
        }
    }
    Ok(report)
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use serde_json::json;

    /// Pasta temporária exclusiva para um teste.
    pub(crate) fn pasta_temporaria(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nexo-test-{tag}-{}", seed_now()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub(crate) fn cena_v1() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "Principal",
            "screen": [1280, 720],
            "objects": [{
                "id": "a1", "name": "Moeda \"dourada\"", "shape": 2,
                "x": 380, "y": 436.5, "w": 34, "h": 34, "rotation": -12.25,
                "color": "#ffd84d", "visible": true, "locked": false,
                "behaviors": [{ "kind": "girar", "speed": 120 }, { "kind": "flutuar", "speed": 0.8 }]
            }]
        })
    }

    #[test]
    fn cena_sobrevive_ida_e_volta_em_ron() {
        let dir = pasta_temporaria("ida-volta");
        let text = Scene::from_value(&cena_v1(), 7).unwrap().to_ron().unwrap();
        let path = dir.to_string_lossy().into_owned();
        save_scene(path.clone(), text.clone()).unwrap();
        let back = load_scene(path).unwrap();
        fs::remove_dir_all(&dir).ok();
        assert_eq!(back, text);
        assert!(Scene::from_text(&back, 7).is_ok());
    }

    #[test]
    fn migracao_move_o_layout_antigo_e_reescreve_a_cena() {
        let dir = pasta_temporaria("migracao");
        fs::create_dir_all(dir.join(LEGACY_SCENE_DIR)).unwrap();
        fs::write(
            dir.join(LEGACY_PROJECT_FILE),
            "[projeto]\nnome = \"Teste\"\nmodelo = \"vazio\"\nversao_engine = \"0.1.0\"\nresolucao = [1280, 720]\n",
        )
        .unwrap();
        let legacy = cena_v1().to_string();
        fs::write(dir.join(LEGACY_SCENE_DIR).join(SCENE_FILE), &legacy).unwrap();

        migrate_layout(&dir, 7).unwrap();

        assert!(dir.join(PROJECT_FILE).is_file());
        assert!(!dir.join(LEGACY_PROJECT_FILE).exists());
        assert!(!dir.join(LEGACY_SCENE_DIR).exists());
        let migrated = fs::read_to_string(dir.join(MAIN_SCENE)).unwrap();
        assert!(
            migrated.contains("format: 2"),
            "cena migrada sem format 2:\n{migrated}"
        );
        assert!(is_format_2(&migrated));
        let backup =
            fs::read_to_string(dir.join(SCENE_DIR).join(format!("{SCENE_FILE}.v1.bak"))).unwrap();
        assert_eq!(backup, legacy);

        // Segunda abertura: nada muda, nem o backup.
        migrate_layout(&dir, 9).unwrap();
        assert_eq!(fs::read_to_string(dir.join(MAIN_SCENE)).unwrap(), migrated);
        let info = read_project(&dir).unwrap();
        assert_eq!(info.name, "Teste");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migracao_le_o_ron_que_a_fase_0_gravava() {
        // A Fase 0 gravava a cena como RON de um valor JSON (não como JSON puro). Esse arquivo também tem de migrar.
        let dir = pasta_temporaria("migracao-ron");
        fs::create_dir_all(dir.join(LEGACY_SCENE_DIR)).unwrap();
        fs::write(
            dir.join(LEGACY_PROJECT_FILE),
            "[projeto]\nnome = \"Antiga\"\nmodelo = \"vazio\"\nversao_engine = \"0.1.0\"\nresolucao = [1280, 720]\n",
        )
        .unwrap();
        let cfg = ron::ser::PrettyConfig::new().struct_names(false);
        let legacy_ron = ron::ser::to_string_pretty(&cena_v1(), cfg).unwrap();
        fs::write(dir.join(LEGACY_SCENE_DIR).join(SCENE_FILE), &legacy_ron).unwrap();

        migrate_layout(&dir, 7).unwrap();

        let migrated = fs::read_to_string(dir.join(MAIN_SCENE)).unwrap();
        assert!(is_format_2(&migrated));
        let scene = Scene::from_text(&migrated, 7).unwrap();
        assert_eq!(scene.name, "Principal");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn criar_projeto_grava_cena_no_formato_2() {
        let dir = pasta_temporaria("novo");
        let info = scaffold(&dir, "Novo", "vazio", &cena_v1()).unwrap();
        let text = fs::read_to_string(dir.join(MAIN_SCENE)).unwrap();
        assert!(is_format_2(&text));
        assert_eq!(info.name, "Novo");
        assert!(dir.join("assets").is_dir() && dir.join(".nexo/cache").is_dir());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn projeto_novo_grava_as_acoes_padrao_no_settings() {
        // INP-002: o settings.ron do projeto novo tem as ações ui_* com teclas, cruz e botões do controle.
        let dir = pasta_temporaria("novo-settings");
        scaffold(&dir, "Novo", "vazio", &cena_v1()).unwrap();
        let text = fs::read_to_string(dir.join(crate::settings::SETTINGS_FILE)).unwrap();
        let settings = ProjectSettings::from_ron(&text).unwrap();
        assert_eq!(settings, ProjectSettings::default());
        assert!(settings.input.keys("ui_left").contains(&"dpad_left".to_string()));
        assert!(settings.input.keys("ui_accept").contains(&"gamepad_a".to_string()));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn renomear_material_nas_cenas_do_disco_poupa_a_cena_aberta() {
        let dir = pasta_temporaria("renomear-cenas");
        scaffold(&dir, "Teste", "vazio", &cena_v1()).unwrap();
        let path = dir.to_string_lossy().into_owned();
        // Uma cena nova com um corpo que usa o material dado.
        let com_material = |material: &str| {
            let mut s = Scene::new_default("Cena", 3);
            let root = s.roots()[0];
            let corpo = s.add_node(Some(root), "StaticBody2D", None).unwrap();
            s.set_prop(corpo, "physics_material", json!(material), None).unwrap();
            s.to_ron().unwrap()
        };
        let pasta = dir.join(SCENE_DIR);
        save_scene(path.clone(), com_material("Gelo")).unwrap();
        fs::write(pasta.join("Nova.scene.ron"), com_material("Gelo")).unwrap();
        fs::create_dir_all(pasta.join("fases")).unwrap();
        fs::write(pasta.join("fases").join("Fase 2.scene.ron"), com_material("Gelo")).unwrap();
        fs::write(pasta.join("Borracha.scene.ron"), com_material("Borracha")).unwrap();
        fs::write(pasta.join(".rascunho.scene.ron"), com_material("Gelo")).unwrap();
        fs::write(pasta.join("quebrada.scene.ron"), "isto não é ron (").unwrap();
        let aberta_antes = fs::read_to_string(dir.join(MAIN_SCENE)).unwrap();
        let borracha_antes = fs::read_to_string(pasta.join("Borracha.scene.ron")).unwrap();

        let report = rename_prop_in_scenes(
            path.clone(),
            MAIN_SCENE.to_string(),
            "physics_material".into(),
            "Gelo".into(),
            "Gelo2".into(),
        )
        .unwrap();

        let mut changed = report.changed.clone();
        changed.sort();
        assert_eq!(changed, ["scenes/Nova.scene.ron", "scenes/fases/Fase 2.scene.ron"]);
        assert_eq!(report.failed.len(), 1, "{:?}", report.failed);
        assert!(report.failed[0].starts_with("scenes/quebrada.scene.ron: "));
        // A cena aberta fica como está: quem a grava é o editor (autosave).
        assert_eq!(fs::read_to_string(dir.join(MAIN_SCENE)).unwrap(), aberta_antes);
        // Cena sem o nome antigo e item oculto não são regravados.
        assert_eq!(fs::read_to_string(pasta.join("Borracha.scene.ron")).unwrap(), borracha_antes);
        assert!(fs::read_to_string(pasta.join(".rascunho.scene.ron")).unwrap().contains("\"Gelo\""));
        // A cena alterada usa o nome novo.
        let nova = fs::read_to_string(pasta.join("Nova.scene.ron")).unwrap();
        assert!(nova.contains("Gelo2") && !nova.contains("\"Gelo\""), "{nova}");

        let fora = pasta_temporaria("renomear-sem-projeto");
        let recusa = rename_prop_in_scenes(
            fora.to_string_lossy().into_owned(),
            MAIN_SCENE.to_string(),
            "physics_material".into(),
            "Gelo".into(),
            "Gelo2".into(),
        );
        assert!(recusa.is_err());
        fs::remove_dir_all(&fora).ok();
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cena_invalida_nao_cria_projeto_nem_sobrescreve_cena() {
        let dir = pasta_temporaria("invalida");
        let bad = json!({ "version": 9 });
        assert!(scaffold(&dir, "X", "vazio", &bad).is_err());
        assert!(!dir.join(PROJECT_FILE).exists());

        let path = dir.to_string_lossy().into_owned();
        let good = Scene::from_value(&cena_v1(), 7).unwrap().to_ron().unwrap();
        save_scene(path.clone(), good.clone()).unwrap();
        assert!(save_scene(path.clone(), "isto não é uma cena".into()).is_err());
        assert_eq!(load_scene(path).unwrap(), good);
        fs::remove_dir_all(&dir).ok();
    }
}
