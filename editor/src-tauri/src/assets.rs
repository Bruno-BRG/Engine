//! Assets do projeto: listagem das pastas, importação com UID estável (arquivo `.import` ao lado),
//! criação, renomeação e lixeira, e o vigia que avisa o editor quando algo muda no disco (AST-004).
//! Todos os caminhos chegam relativos à pasta do projeto, com `/`. Nada sai dessa pasta.

use crate::project::{LEGACY_PROJECT_FILE, PROJECT_FILE, write_atomic};
use nexo_core::{Scene, seed_now};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

type CmdResult<T> = Result<T, String>;

/// Extensões aceitas pelo import (AST-003, AUD-002). Outras são recusadas com uma mensagem clara.
const IMPORTABLE: &[&str] = &["png", "jpg", "jpeg", "webp", "wav", "ogg", "mp3", "ttf", "otf"];
/// Evento enviado ao frontend quando algo muda na pasta do projeto.
const FS_EVENT: &str = "project-fs-changed";
/// Espera do vigia: uma rajada de escritas vira um único aviso.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Uma entrada da pasta (ou um asset, em `list_assets`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub rel: String,
    pub is_dir: bool,
    pub size: u64,
    pub kind: &'static str,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub rel: String,
    pub uid: String,
    pub kind: &'static str,
}

/// Conteúdo do arquivo `.import` ao lado de cada asset.
#[derive(Serialize, Deserialize)]
struct ImportFile {
    uid: String,
    kind: String,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn file_name(p: &Path) -> &str {
    p.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// Confere que a pasta é um projeto Nexo e devolve o caminho dela.
fn project_root(project: &str) -> CmdResult<PathBuf> {
    let root = PathBuf::from(project);
    if !root.join(PROJECT_FILE).is_file() && !root.join(LEGACY_PROJECT_FILE).is_file() {
        return Err(format!("{project} não é um projeto Nexo"));
    }
    Ok(root)
}

/// Caminho relativo vindo do frontend, convertido em caminho seguro. Recusa absolutos, unidades e `..`.
fn rel_path(rel: &str) -> CmdResult<PathBuf> {
    let norm = rel.replace('\\', "/");
    let mut out = PathBuf::new();
    for c in Path::new(&norm).components() {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            _ => return Err(format!("Caminho fora do projeto: “{rel}”")),
        }
    }
    Ok(out)
}

fn locate(project: &str, rel: &str) -> CmdResult<(PathBuf, PathBuf)> {
    Ok((project_root(project)?, rel_path(rel)?))
}

/// Forma usada pelo frontend: componentes separados por `/`, sem raiz.
fn rel_string(p: &Path) -> String {
    p.components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn join_rel(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Nomes que o painel não mostra: pontos (metadados e temporários do Nexo), sidecars, backups e temporários.
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
        || name.ends_with(".import")
        || name.ends_with(".v1.bak")
        || name.ends_with(".tmp")
}

/// Arquivo do projeto na raiz: fica oculto e protegido.
fn is_metadata_at_root(rel_dir: &str, name: &str) -> bool {
    rel_dir.is_empty() && (name == PROJECT_FILE || name == LEGACY_PROJECT_FILE)
}

/// Recusa operações em arquivos internos do Nexo (pontos, sidecars e o arquivo do projeto).
fn check_user_path(rp: &Path) -> CmdResult<()> {
    let names: Vec<&str> = rp
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    let Some(last) = names.last().copied() else {
        return Err("Caminho vazio".into());
    };
    let root_file = names.len() == 1 && is_metadata_at_root("", last);
    if names.iter().any(|n| n.starts_with('.')) || is_hidden(last) || root_file {
        return Err(format!("“{last}” é um arquivo interno do Nexo"));
    }
    Ok(())
}

/// Tipo do arquivo pelo nome. `scene` é a cena RON e `script` o grafo de lógica (VIS-001): nenhum dos dois é
/// asset importado, então não ganha sidecar nem UID.
fn classify(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".scene.ron") {
        return "scene";
    }
    if lower.ends_with(".logic.ron") {
        return "script";
    }
    match lower.rsplit_once('.').map(|(_, e)| e) {
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp") => "texture",
        Some("wav" | "ogg" | "mp3" | "flac") => "audio",
        Some("ttf" | "otf" | "woff" | "woff2") => "font",
        _ => "other",
    }
}

fn ext_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}

/// Arquivo `.import` ao lado do asset. O UID nele não muda mesmo quando o arquivo é renomeado ou movido (AST-005).
fn sidecar_of(asset: &Path) -> PathBuf {
    let mut s = asset.as_os_str().to_owned();
    s.push(".import");
    PathBuf::from(s)
}

fn read_uid(asset: &Path) -> Option<String> {
    let text = fs::read_to_string(sidecar_of(asset)).ok()?;
    toml::from_str::<ImportFile>(&text).ok().map(|f| f.uid)
}

/// 64 bits em 16 dígitos hexadecimais minúsculos.
fn new_uid() -> String {
    format!("{:016x}", rand::random::<u64>())
}

fn write_sidecar(asset: &Path, uid: &str, kind: &str) -> CmdResult<()> {
    let body = toml::to_string(&ImportFile {
        uid: uid.to_string(),
        kind: kind.to_string(),
    })
    .map_err(err)?;
    let text = format!(
        "# Gerado pelo Nexo. Guarde junto do arquivo: o uid liga as cenas a este asset.\n{body}"
    );
    write_atomic(&sidecar_of(asset), text.as_bytes())
}

/// Devolve o UID do asset, criando o sidecar se ele não existir. Um sidecar ilegível dá erro:
/// trocar o UID em silêncio quebraria as referências das cenas.
fn ensure_sidecar(asset: &Path, kind: &str) -> CmdResult<String> {
    let side = sidecar_of(asset);
    if side.exists() {
        let text = fs::read_to_string(&side).map_err(err)?;
        let parsed: ImportFile = toml::from_str(&text)
            .map_err(|e| format!("Arquivo .import inválido ({}): {e}", side.display()))?;
        return Ok(parsed.uid);
    }
    let uid = new_uid();
    write_sidecar(asset, &uid, kind)?;
    Ok(uid)
}

/// Lista recursiva dos assets: todos os arquivos, exceto cenas e internos. Pastas ocultas não entram.
fn collect_assets(dir: &Path, rel: &str, out: &mut Vec<DirEntry>) -> CmdResult<()> {
    for entry in fs::read_dir(dir).map_err(err)? {
        let entry = entry.map_err(err)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_hidden(&name) || is_metadata_at_root(rel, &name) {
            continue;
        }
        let path = entry.path();
        let meta = entry.metadata().map_err(err)?;
        let child = join_rel(rel, &name);
        if meta.is_dir() {
            collect_assets(&path, &child, out)?;
            continue;
        }
        let kind = classify(&name);
        if matches!(kind, "scene" | "script") {
            continue;
        }
        out.push(DirEntry {
            name,
            rel: child,
            is_dir: false,
            size: meta.len(),
            kind,
            uid: read_uid(&path),
        });
    }
    Ok(())
}

/// Cria o sidecar de todo asset que ainda não tem um. Continua mesmo se um sidecar estiver ruim, e devolve o primeiro erro.
fn ensure_all(root: &Path) -> CmdResult<()> {
    let mut found = Vec::new();
    collect_assets(root, "", &mut found)?;
    let mut first: Option<String> = None;
    for asset in found {
        if let Err(e) = ensure_sidecar(&root.join(&asset.rel), asset.kind) {
            first.get_or_insert(e);
        }
    }
    match first {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Primeiro nome livre na pasta: `hero.png`, depois `hero 2.png`, `hero 3.png`.
fn free_name(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !taken(&first) {
        return first;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let mut n = 2;
    loop {
        let candidate = dir.join(format!("{stem} {n}{ext}"));
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn taken(p: &Path) -> bool {
    p.exists() || sidecar_of(p).exists()
}

/// Importa os arquivos em `dest_rel`. Confere tudo antes de copiar, para não deixar a importação pela metade.
/// Arquivo que já está dentro do projeto não é copiado de novo: só ganha o sidecar, se faltar.
fn import_into(root: &Path, dest_rel: &str, sources: &[String]) -> CmdResult<Vec<ImportResult>> {
    let dest_rp = rel_path(dest_rel)?;
    if !dest_rp.as_os_str().is_empty() {
        check_user_path(&dest_rp)?;
    }
    let dest_dir = root.join(&dest_rp);
    if !dest_dir.is_dir() {
        return Err(format!("A pasta de destino não existe: “{dest_rel}”"));
    }

    let mut plan = Vec::new();
    for src in sources {
        let path = PathBuf::from(src);
        let name = file_name(&path).to_string();
        if name.is_empty() {
            return Err(format!("Caminho inválido: {src}"));
        }
        if !IMPORTABLE.contains(&ext_of(&name).as_str()) {
            return Err(format!(
                "Formato não suportado: “{name}”. Use PNG, JPG, WEBP, WAV, OGG, MP3, TTF ou OTF."
            ));
        }
        if !path.is_file() {
            return Err(format!("Arquivo não encontrado: {src}"));
        }
        plan.push((path, name));
    }

    let root_canon = fs::canonicalize(root).map_err(err)?;
    let mut out = Vec::new();
    for (src, name) in plan {
        let kind = classify(&name);
        let src_canon = fs::canonicalize(&src).map_err(err)?;
        if let Ok(inside) = src_canon.strip_prefix(&root_canon) {
            let rp = inside.to_path_buf();
            check_user_path(&rp)?;
            let uid = ensure_sidecar(&root.join(&rp), kind)?;
            out.push(ImportResult {
                rel: rel_string(&rp),
                uid,
                kind,
            });
            continue;
        }
        let target = free_name(&dest_dir, &name);
        let tmp = target.with_file_name(format!(".{}.tmp", file_name(&target)));
        fs::copy(&src, &tmp).map_err(|e| format!("Não foi possível copiar “{name}”: {e}"))?;
        if let Err(e) = fs::rename(&tmp, &target) {
            let _ = fs::remove_file(&tmp);
            return Err(format!("Não foi possível importar “{name}”: {e}"));
        }
        let uid = new_uid();
        write_sidecar(&target, &uid, kind)?;
        let rel = target
            .strip_prefix(root)
            .map(rel_string)
            .unwrap_or_default();
        out.push(ImportResult { rel, uid, kind });
    }
    Ok(out)
}

// ---------- Comandos ----------

#[tauri::command]
pub fn list_dir(project: String, rel: String) -> CmdResult<Vec<DirEntry>> {
    let (root, rp) = locate(&project, &rel)?;
    let here = rel_string(&rp);
    let dir = root.join(&rp);
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("Não foi possível abrir “{rel}”: {e}"))?
    {
        let entry = entry.map_err(err)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_hidden(&name) || is_metadata_at_root(&here, &name) {
            continue;
        }
        let path = entry.path();
        let meta = entry.metadata().map_err(err)?;
        let child = join_rel(&here, &name);
        if meta.is_dir() {
            out.push(DirEntry {
                name,
                rel: child,
                is_dir: true,
                size: 0,
                kind: "folder",
                uid: None,
            });
        } else {
            let kind = classify(&name);
            let uid = if matches!(kind, "scene" | "script") {
                None
            } else {
                read_uid(&path)
            };
            out.push(DirEntry {
                name,
                rel: child,
                is_dir: false,
                size: meta.len(),
                kind,
                uid,
            });
        }
    }
    // Pastas primeiro, depois por nome, sem diferenciar maiúsculas.
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

/// Todos os assets do projeto, de todas as pastas, com UID. Usado pelo editor para as texturas.
#[tauri::command]
pub async fn list_assets(project: String) -> CmdResult<Vec<DirEntry>> {
    let root = project_root(&project)?;
    let mut out = Vec::new();
    collect_assets(&root, "", &mut out)?;
    Ok(out)
}

/// Bytes de um arquivo, sem serialização JSON (usado por miniaturas e pela decodificação de texturas).
#[tauri::command]
pub async fn read_file(project: String, rel: String) -> CmdResult<tauri::ipc::Response> {
    let (root, rp) = locate(&project, &rel)?;
    let bytes =
        fs::read(root.join(&rp)).map_err(|e| format!("Não foi possível ler “{rel}”: {e}"))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn write_text(project: String, rel: String, text: String) -> CmdResult<()> {
    let (root, rp) = locate(&project, &rel)?;
    check_user_path(&rp)?;
    write_atomic(&root.join(rp), text.as_bytes())
}

#[tauri::command]
pub fn create_folder(project: String, rel: String) -> CmdResult<()> {
    let (root, rp) = locate(&project, &rel)?;
    check_user_path(&rp)?;
    let abs = root.join(&rp);
    if abs.exists() {
        return Err(format!("Já existe um item chamado “{}”", file_name(&rp)));
    }
    fs::create_dir(&abs).map_err(|e| format!("Não foi possível criar a pasta: {e}"))
}

/// Cria uma cena nova (formato 2, com o nó raiz e a câmera). Devolve o caminho relativo criado.
#[tauri::command]
pub fn create_scene(project: String, rel: String, name: String) -> CmdResult<String> {
    let file_rel = if rel.ends_with(".scene.ron") {
        rel
    } else {
        format!("{rel}.scene.ron")
    };
    let (root, rp) = locate(&project, &file_rel)?;
    check_user_path(&rp)?;
    let abs = root.join(&rp);
    if abs.exists() {
        return Err(format!("Já existe um item chamado “{}”", file_name(&rp)));
    }
    if !abs.parent().is_some_and(Path::is_dir) {
        return Err("A pasta da nova cena não existe".into());
    }
    let title = name.trim();
    let title = if title.is_empty() {
        file_name(&rp).trim_end_matches(".scene.ron")
    } else {
        title
    };
    let text = Scene::new_default(title, seed_now()).to_ron()?;
    write_atomic(&abs, text.as_bytes())?;
    Ok(rel_string(&rp))
}

/// Move ou renomeia uma pasta ou arquivo. O sidecar `.import` vai junto e mantém o UID.
#[tauri::command]
pub fn rename_entry(project: String, from: String, to: String) -> CmdResult<()> {
    let (root, from_rp) = locate(&project, &from)?;
    let (_, to_rp) = locate(&project, &to)?;
    check_user_path(&from_rp)?;
    check_user_path(&to_rp)?;
    if from_rp == to_rp {
        return Ok(());
    }
    let from_abs = root.join(&from_rp);
    let to_abs = root.join(&to_rp);
    if !from_abs.exists() {
        return Err(format!("“{from}” não existe mais"));
    }
    if to_rp.starts_with(&from_rp) {
        return Err("Não é possível mover uma pasta para dentro dela mesma".into());
    }
    if !to_abs.parent().is_some_and(Path::is_dir) {
        return Err(format!("A pasta de destino não existe: “{to}”"));
    }
    // Só trocar maiúsculas e minúsculas (Hero.png para hero.png) aponta para o mesmo arquivo.
    let same_file =
        to_abs.exists() && fs::canonicalize(&from_abs).ok() == fs::canonicalize(&to_abs).ok();
    if to_abs.exists() && !same_file {
        return Err(format!("Já existe um item chamado “{}”", file_name(&to_rp)));
    }
    let from_side = sidecar_of(&from_abs);
    let to_side = sidecar_of(&to_abs);
    if !same_file && to_side.exists() {
        return Err(format!(
            "Já existe um arquivo .import chamado “{}”",
            file_name(&to_rp)
        ));
    }

    fs::rename(&from_abs, &to_abs).map_err(|e| format!("Não foi possível renomear: {e}"))?;
    if from_side.is_file() {
        if let Err(e) = fs::rename(&from_side, &to_side) {
            // Desfaz o nome do arquivo para não deixar o asset sem o sidecar.
            let _ = fs::rename(&to_abs, &from_abs);
            return Err(format!("Não foi possível mover o arquivo .import: {e}"));
        }
    } else if to_abs.is_file() {
        let kind = classify(file_name(&to_rp));
        if !matches!(kind, "scene" | "script") {
            // Asset sem sidecar (criado fora do editor): ganha o UID agora.
            ensure_sidecar(&to_abs, kind)?;
        }
    }
    Ok(())
}

/// Manda para a lixeira do sistema, nunca apaga de vez. O sidecar também vai.
#[tauri::command]
pub async fn trash_entry(project: String, rel: String) -> CmdResult<()> {
    let (root, rp) = locate(&project, &rel)?;
    check_user_path(&rp)?;
    let abs = root.join(&rp);
    if !abs.exists() {
        return Err(format!("“{rel}” não existe mais"));
    }
    let is_file = abs.is_file();
    trash::delete(&abs)
        .map_err(|e| format!("Não foi possível mover “{rel}” para a lixeira: {e}"))?;
    let side = sidecar_of(&abs);
    if is_file && side.exists() {
        trash::delete(&side)
            .map_err(|e| format!("Não foi possível mover o arquivo .import para a lixeira: {e}"))?;
    }
    Ok(())
}

/// Importa arquivos PNG, JPG, WEBP, WAV, OGG, MP3, TTF ou OTF. `dir` é a pasta de destino (padrão `assets`).
#[tauri::command]
pub async fn import_files(
    project: String,
    sources: Vec<String>,
    dir: Option<String>,
) -> CmdResult<Vec<ImportResult>> {
    let root = project_root(&project)?;
    let dest = dir.unwrap_or_else(|| "assets".to_string());
    import_into(&root, &dest, &sources)
}

/// Cria o sidecar de todo asset que não tem um (feito ao abrir o projeto).
#[tauri::command]
pub async fn ensure_imports(project: String) -> CmdResult<()> {
    let root = project_root(&project)?;
    ensure_all(&root)
}

/// Vigia da pasta do projeto aberto. Fica no estado do Tauri: trocar de projeto substitui o vigia anterior.
#[derive(Default)]
pub struct Watch(Mutex<Option<Debouncer<RecommendedWatcher>>>);

#[derive(Clone, Serialize)]
struct FsChanged {
    paths: Vec<String>,
}

/// Caminho relativo de um evento, sem os internos do Nexo. None se o evento não interessa.
fn visible_rel(base: &Path, path: &Path) -> Option<String> {
    let rel = rel_string(path.strip_prefix(base).ok()?);
    if rel.is_empty() || rel.split('/').any(|n| n.starts_with('.')) {
        return None;
    }
    Some(rel)
}

#[tauri::command]
pub fn watch_project(app: AppHandle, watch: State<'_, Watch>, project: String) -> CmdResult<()> {
    // Caminho canônico para vigiar e para cortar os caminhos dos eventos: assim o prefixo sempre bate.
    let root = fs::canonicalize(project_root(&project)?).map_err(err)?;
    let base = root.clone();
    let mut debouncer = new_debouncer(DEBOUNCE, move |res: DebounceEventResult| match res {
        Ok(events) => {
            let mut paths: Vec<String> = events
                .iter()
                .filter_map(|ev| visible_rel(&base, &ev.path))
                .collect();
            paths.sort();
            paths.dedup();
            if !paths.is_empty() {
                let _ = app.emit(FS_EVENT, FsChanged { paths });
            }
        }
        Err(e) => eprintln!("vigia do projeto: {e:?}"),
    })
    .map_err(err)?;
    debouncer
        .watcher()
        .watch(&root, RecursiveMode::Recursive)
        .map_err(err)?;
    *watch.0.lock().unwrap_or_else(|p| p.into_inner()) = Some(debouncer);
    Ok(())
}

#[tauri::command]
pub fn stop_watch(watch: State<'_, Watch>) {
    *watch.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::SCENE_DIR;
    use crate::project::testes::pasta_temporaria;

    /// Projeto mínimo: pasta com `project.nexo`.
    fn projeto(tag: &str) -> PathBuf {
        let dir = pasta_temporaria(tag);
        fs::write(dir.join(PROJECT_FILE), "[projeto]\nnome = \"T\"\nmodelo = \"vazio\"\nversao_engine = \"0.1.0\"\nresolucao = [1280, 720]\n").unwrap();
        fs::create_dir_all(dir.join(SCENE_DIR)).unwrap();
        fs::create_dir_all(dir.join("assets")).unwrap();
        dir
    }

    #[test]
    fn caminhos_fora_do_projeto_sao_recusados() {
        assert!(rel_path("../x").is_err());
        assert!(rel_path("assets/../../x").is_err());
        assert!(rel_path("/etc/passwd").is_err());
        assert_eq!(
            rel_string(&rel_path("assets\\sprites/hero.png").unwrap()),
            "assets/sprites/hero.png"
        );
    }

    #[test]
    fn listagem_esconde_os_internos_do_nexo() {
        let dir = projeto("lista");
        fs::create_dir_all(dir.join(".nexo/cache")).unwrap();
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(dir.join("assets/hero.png"), b"png").unwrap();
        fs::write(dir.join(".gitignore"), "x").unwrap();
        fs::write(
            dir.join("assets/hero.png.import"),
            "uid = \"00000000000000aa\"\nkind = \"texture\"\n",
        )
        .unwrap();
        fs::write(dir.join("assets/old.png.v1.bak"), "x").unwrap();
        let p = dir.to_string_lossy().into_owned();

        let root = list_dir(p.clone(), String::new()).unwrap();
        let names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["assets", "scenes"]);
        assert!(root[0].is_dir);

        let inside = list_dir(p, "assets".into()).unwrap();
        assert_eq!(inside.len(), 1);
        assert_eq!(inside[0].kind, "texture");
        assert_eq!(inside[0].uid.as_deref(), Some("00000000000000aa"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn importar_copia_para_assets_e_cria_sidecar() {
        let dir = projeto("importa");
        let src_dir = pasta_temporaria("origem");
        let src = src_dir.join("hero.png");
        fs::write(&src, b"fake-png").unwrap();
        let sources = vec![src.to_string_lossy().into_owned()];

        let first = import_into(&dir, "assets", &sources).unwrap();
        assert_eq!(first[0].rel, "assets/hero.png");
        assert_eq!(first[0].kind, "texture");
        assert_eq!(first[0].uid.len(), 16);
        assert!(dir.join("assets/hero.png.import").is_file());

        // Mesmo nome de novo: vira "hero 2.png" com UID próprio.
        let second = import_into(&dir, "assets", &sources).unwrap();
        assert_eq!(second[0].rel, "assets/hero 2.png");
        assert_ne!(second[0].uid, first[0].uid);

        // Formato fora da lista é recusado, e nada é copiado.
        let txt = src_dir.join("notas.txt");
        fs::write(&txt, b"x").unwrap();
        let bad = vec![txt.to_string_lossy().into_owned()];
        assert!(import_into(&dir, "assets", &bad).is_err());
        assert!(!dir.join("assets/notas.txt").exists());

        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&src_dir).ok();
    }

    #[test]
    fn importar_mp3_grava_kind_audio_no_sidecar_e_na_listagem() {
        let dir = projeto("mp3");
        let src_dir = pasta_temporaria("origem-mp3");
        let src = src_dir.join("musica.mp3");
        fs::write(&src, b"fake-mp3").unwrap();
        let sources = vec![src.to_string_lossy().into_owned()];

        let res = import_into(&dir, "assets", &sources).unwrap();
        assert_eq!(res[0].rel, "assets/musica.mp3");
        assert_eq!(res[0].kind, "audio");
        let side = fs::read_to_string(dir.join("assets/musica.mp3.import")).unwrap();
        assert!(side.contains("kind = \"audio\""), "sidecar: {side}");

        let inside = list_dir(dir.to_string_lossy().into_owned(), "assets".into()).unwrap();
        assert_eq!(inside[0].kind, "audio");
        assert_eq!(inside[0].uid.as_deref(), Some(res[0].uid.as_str()));
        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&src_dir).ok();
    }

    #[test]
    fn renomear_e_mover_mantem_o_uid() {
        let dir = projeto("renomeia");
        fs::create_dir_all(dir.join("assets/sprites")).unwrap();
        fs::write(dir.join("assets/hero.png"), b"png").unwrap();
        ensure_all(&dir).unwrap();
        let uid = read_uid(&dir.join("assets/hero.png")).unwrap();
        let p = dir.to_string_lossy().into_owned();

        rename_entry(
            p.clone(),
            "assets/hero.png".into(),
            "assets/sprites/heroi.png".into(),
        )
        .unwrap();
        assert!(!dir.join("assets/hero.png.import").exists());
        assert_eq!(
            read_uid(&dir.join("assets/sprites/heroi.png")).unwrap(),
            uid
        );

        // Pasta inteira: os sidecars de dentro vão junto.
        rename_entry(p.clone(), "assets/sprites".into(), "arte".into()).unwrap();
        assert_eq!(read_uid(&dir.join("arte/heroi.png")).unwrap(), uid);

        // Não sobrescreve nem entra em si mesma.
        fs::write(dir.join("arte/outro.png"), b"x").unwrap();
        assert!(rename_entry(p.clone(), "arte/outro.png".into(), "arte/heroi.png".into()).is_err());
        assert!(rename_entry(p, "arte".into(), "arte/dentro".into()).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nome_livre_acrescenta_numero_antes_da_extensao() {
        let dir = pasta_temporaria("livre");
        fs::write(dir.join("a.png"), b"x").unwrap();
        fs::write(dir.join("a 2.png.import"), b"x").unwrap();
        assert_eq!(file_name(&free_name(&dir, "a.png")), "a 3.png");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn criar_cena_acrescenta_o_sufixo_e_recusa_sobrescrever() {
        let dir = projeto("cena");
        let p = dir.to_string_lossy().into_owned();
        let rel = create_scene(p.clone(), "scenes/Nova".into(), "Nova".into()).unwrap();
        assert_eq!(rel, "scenes/Nova.scene.ron");
        let text = fs::read_to_string(dir.join(&rel)).unwrap();
        assert!(text.contains(&format!("format: {}", nexo_core::SCENE_FORMAT)));
        assert!(create_scene(p, "scenes/Nova".into(), "Nova".into()).is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
