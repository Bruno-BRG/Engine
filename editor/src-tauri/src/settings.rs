//! Configurações do projeto no disco: `settings.ron` na raiz do projeto (PHY-021, INP-001, SCN-033,
//! PHY-023, AUD-003). O frontend troca JSON, com os mesmos nomes de campo do Rust; o arquivo fica em
//! RON, como as cenas. A gravação é atômica (project::write_atomic).

use crate::project::{project_file_exists, write_atomic};
use nexo_core::ProjectSettings;
use std::fs;
use std::path::Path;

pub(crate) const SETTINGS_FILE: &str = "settings.ron";

type CmdResult<T> = Result<T, String>;

/// Configurações do projeto em JSON. Sem `settings.ron`, devolve os padrões. Arquivo ilegível vira
/// erro e nada é apagado: o frontend não grava por cima enquanto o erro estiver ativo.
#[tauri::command]
pub fn load_settings(path: String) -> CmdResult<String> {
    let file = Path::new(&path).join(SETTINGS_FILE);
    let settings = if file.is_file() {
        let text = fs::read_to_string(&file).map_err(|e| format!("{SETTINGS_FILE}: {e}"))?;
        ProjectSettings::from_ron(&text).map_err(|e| format!("{SETTINGS_FILE}: {e}"))?
    } else {
        ProjectSettings::default()
    };
    settings.to_json()
}

/// Grava as configurações (JSON) como `settings.ron`. Recusa pasta que não é projeto e JSON inválido.
#[tauri::command]
pub fn save_settings(path: String, text: String) -> CmdResult<()> {
    let dir = Path::new(&path);
    if !project_file_exists(dir) {
        return Err(format!("{path} não é um projeto Nexo"));
    }
    let settings = ProjectSettings::from_json(&text)?;
    write_atomic(&dir.join(SETTINGS_FILE), settings.to_ron()?.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{PROJECT_FILE, testes};

    fn projeto(tag: &str) -> std::path::PathBuf {
        let dir = testes::pasta_temporaria(tag);
        fs::write(dir.join(PROJECT_FILE), "").unwrap();
        dir
    }

    #[test]
    fn sem_arquivo_carrega_os_padroes() {
        let dir = projeto("settings-padrao");
        let json = load_settings(dir.to_string_lossy().into_owned()).unwrap();
        let loaded = ProjectSettings::from_json(&json).unwrap();
        assert_eq!(loaded, ProjectSettings::default());
        assert!(!dir.join(SETTINGS_FILE).exists(), "carregar não deve criar o arquivo");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn salvar_e_carregar_mantem_os_valores() {
        let dir = projeto("settings-ida-volta");
        let path = dir.to_string_lossy().into_owned();
        let mut s = ProjectSettings::default();
        s.gravity = [0.0, 1500.0];
        s.tags.push("boss".into());
        s.input.rebind("jump", "KeyZ");
        save_settings(path.clone(), s.to_json().unwrap()).unwrap();

        let ron = fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap();
        assert!(ron.contains("gravity"), "arquivo sem gravidade:\n{ron}");
        let back = ProjectSettings::from_json(&load_settings(path).unwrap()).unwrap();
        assert_eq!(back, s);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recusa_pasta_que_nao_e_projeto() {
        let dir = testes::pasta_temporaria("settings-sem-projeto");
        let json = ProjectSettings::default().to_json().unwrap();
        assert!(save_settings(dir.to_string_lossy().into_owned(), json).is_err());
        assert!(!dir.join(SETTINGS_FILE).exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn json_invalido_nao_substitui_o_arquivo() {
        let dir = projeto("settings-invalido");
        let path = dir.to_string_lossy().into_owned();
        let bom = ProjectSettings::default().to_json().unwrap();
        save_settings(path.clone(), bom).unwrap();
        let antes = fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap();
        assert!(save_settings(path, "{ isto não é json".into()).is_err());
        assert_eq!(fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap(), antes);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn arquivo_ilegivel_da_erro_e_fica_intacto() {
        let dir = projeto("settings-ilegivel");
        fs::write(dir.join(SETTINGS_FILE), "isto não é ron (").unwrap();
        let err = load_settings(dir.to_string_lossy().into_owned()).unwrap_err();
        assert!(err.starts_with(SETTINGS_FILE), "erro sem o nome do arquivo: {err}");
        assert_eq!(fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap(), "isto não é ron (");
        fs::remove_dir_all(&dir).ok();
    }
}
