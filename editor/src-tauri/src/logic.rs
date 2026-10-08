//! Scripts de lógica no disco: `scripts/<nome>.logic.ron` na raiz do projeto (VIS-001, decisão de contrato 2). O
//! frontend troca o texto RON do grafo (o `GraphEditor` do WASM lê e grava). O Rust confere o formato antes de
//! gravar e grava de forma atômica (project::write_atomic). Um script só vira arquivo quando é criado ou
//! alterado: abrir um projeto ou um script nunca cria nada.

use crate::project::{project_file_exists, write_atomic};
use nexo_graph::Graph;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const SCRIPT_DIR: &str = "scripts";
pub(crate) const SCRIPT_EXT: &str = ".logic.ron";

type CmdResult<T> = Result<T, String>;

/// Caracteres que o sistema de arquivos recusa, mais os que o painel de Projeto também recusa.
const PROIBIDOS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Confere a pasta do projeto e devolve o caminho da pasta de scripts.
fn pasta_de_scripts(path: &str) -> CmdResult<PathBuf> {
    let dir = Path::new(path);
    if !project_file_exists(dir) {
        return Err(format!("{path} não é um projeto Nexo"));
    }
    Ok(dir.join(SCRIPT_DIR))
}

/// Nome de script válido: sem caracteres proibidos, sem ponto no início e não vazio. Devolve o nome sem espaços
/// nas pontas.
fn nome_valido(name: &str) -> CmdResult<String> {
    let nome = name.trim();
    if nome.is_empty() {
        return Err("Dê um nome ao script".into());
    }
    if nome.contains(PROIBIDOS) || nome.chars().any(char::is_control) {
        return Err("O nome não pode ter \\ / : * ? \" < > | nem caracteres de controle".into());
    }
    if nome.starts_with('.') {
        return Err("O nome não pode começar com ponto".into());
    }
    Ok(nome.to_string())
}

fn caminho_do_script(dir: &Path, name: &str) -> CmdResult<PathBuf> {
    Ok(dir.join(format!("{}{SCRIPT_EXT}", nome_valido(name)?)))
}

/// Texto RON de um grafo vazio. É o conteúdo de um script novo.
fn grafo_vazio() -> CmdResult<String> {
    Graph::default().to_ron()
}

/// Escreve o grafo, depois de conferir que o texto é um grafo que o editor lê de volta.
fn grava(arquivo: &Path, text: &str) -> CmdResult<()> {
    Graph::from_ron(text).map_err(|e| format!("O script não foi salvo: {e}"))?;
    write_atomic(arquivo, text.as_bytes())
}

/// Nomes dos scripts do projeto, sem a pasta e sem a extensão. Ordem alfabética, sem diferenciar maiúsculas.
#[tauri::command]
pub fn list_scripts(path: String) -> CmdResult<Vec<String>> {
    let dir = pasta_de_scripts(&path)?;
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in fs::read_dir(&dir).map_err(|e| format!("Não foi possível ler os scripts: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file = entry.file_name().to_string_lossy().into_owned();
        if file.starts_with('.') || !entry.path().is_file() {
            continue;
        }
        if let Some(stem) = file.strip_suffix(SCRIPT_EXT) {
            out.push(stem.to_string());
        }
    }
    out.sort_by_key(|s| s.to_lowercase());
    Ok(out)
}

/// Texto RON do script. Sem arquivo, devolve um grafo vazio sem criar o arquivo. Arquivo que não é um grafo
/// vira erro e nada é alterado.
#[tauri::command]
pub fn load_script(path: String, name: String) -> CmdResult<String> {
    let dir = pasta_de_scripts(&path)?;
    let arquivo = caminho_do_script(&dir, &name)?;
    if !arquivo.is_file() {
        return grafo_vazio();
    }
    let text = fs::read_to_string(&arquivo).map_err(|e| format!("{name}: {e}"))?;
    Graph::from_ron(&text).map_err(|e| format!("{name}{SCRIPT_EXT}: {e}"))?;
    Ok(text)
}

/// Grava o texto RON do script. Chamada a cada alteração (salvamento automático do editor).
#[tauri::command]
pub fn save_script(path: String, name: String, text: String) -> CmdResult<()> {
    let dir = pasta_de_scripts(&path)?;
    grava(&caminho_do_script(&dir, &name)?, &text)
}

/// Cria um script vazio com o nome dado. Recusa nome que já existe: quem chama escolhe um nome livre.
#[tauri::command]
pub fn create_script(path: String, name: String) -> CmdResult<String> {
    let dir = pasta_de_scripts(&path)?;
    let arquivo = caminho_do_script(&dir, &name)?;
    if arquivo.exists() {
        return Err(format!("Já existe um script chamado “{}”", nome_valido(&name)?));
    }
    grava(&arquivo, &grafo_vazio()?)?;
    nome_valido(&name)
}

/// Renomeia o script. Só trocar maiúsculas e minúsculas é permitido mesmo com o nome antigo existindo.
#[tauri::command]
pub fn rename_script(path: String, from: String, to: String) -> CmdResult<()> {
    let dir = pasta_de_scripts(&path)?;
    let origem = caminho_do_script(&dir, &from)?;
    let destino = caminho_do_script(&dir, &to)?;
    if !origem.is_file() {
        return Err(format!("O script “{from}” não existe mais"));
    }
    let so_maiusculas = from.trim().to_lowercase() == to.trim().to_lowercase();
    if destino.exists() && !so_maiusculas {
        return Err(format!("Já existe um script chamado “{}”", to.trim()));
    }
    fs::rename(&origem, &destino).map_err(|e| format!("Não foi possível renomear: {e}"))
}

/// Copia o script para um nome novo, “<nome> cópia”, acrescentando número se o nome já existir. Devolve o nome.
#[tauri::command]
pub fn duplicate_script(path: String, name: String) -> CmdResult<String> {
    let dir = pasta_de_scripts(&path)?;
    let origem = caminho_do_script(&dir, &name)?;
    let texto = fs::read_to_string(&origem).map_err(|e| format!("“{name}”: {e}"))?;
    let base = nome_valido(&name)?;
    let mut novo = format!("{base} cópia");
    let mut n = 2;
    while dir.join(format!("{novo}{SCRIPT_EXT}")).exists() {
        novo = format!("{base} cópia {n}");
        n += 1;
    }
    grava(&dir.join(format!("{novo}{SCRIPT_EXT}")), &texto)?;
    Ok(novo)
}

/// Manda o script para a lixeira do sistema, nunca apaga de vez.
#[tauri::command]
pub fn delete_script(path: String, name: String) -> CmdResult<()> {
    let dir = pasta_de_scripts(&path)?;
    let arquivo = caminho_do_script(&dir, &name)?;
    if !arquivo.is_file() {
        return Err(format!("O script “{name}” não existe mais"));
    }
    trash::delete(&arquivo).map_err(|e| format!("Não foi possível mover “{name}” para a lixeira: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{PROJECT_FILE, testes};
    use nexo_graph::Value;

    fn projeto(tag: &str) -> PathBuf {
        let dir = testes::pasta_temporaria(tag);
        fs::write(dir.join(PROJECT_FILE), "").unwrap();
        dir
    }

    fn caminho(dir: &Path) -> String {
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn abrir_script_sem_arquivo_nao_cria_nada() {
        let dir = projeto("script-sem-arquivo");
        let text = load_script(caminho(&dir), "Novo".into()).unwrap();
        assert_eq!(Graph::from_ron(&text).unwrap(), Graph::default());
        assert!(!dir.join(SCRIPT_DIR).exists(), "abrir não deve criar a pasta");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn salvar_e_carregar_mantem_o_grafo() {
        let dir = projeto("script-ida-volta");
        let path = caminho(&dir);
        let mut g = Graph::default();
        let ev = g.add_node("OnReady", [0.0, 0.0]).unwrap();
        let imp = g.add_node("Print", [200.0, 0.0]).unwrap();
        g.connect((ev, "exec"), (imp, "exec")).unwrap();
        g.set_value(imp, "msg", Value::String("oi".into())).unwrap();
        save_script(path.clone(), "Movimento".into(), g.to_ron().unwrap()).unwrap();
        assert!(dir.join(SCRIPT_DIR).join("Movimento.logic.ron").is_file());
        let de_volta = Graph::from_ron(&load_script(path.clone(), "Movimento".into()).unwrap()).unwrap();
        assert_eq!(de_volta, g);
        assert_eq!(list_scripts(path).unwrap(), vec!["Movimento".to_string()]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recusa_texto_que_nao_e_grafo_sem_tocar_no_arquivo() {
        let dir = projeto("script-invalido");
        let path = caminho(&dir);
        save_script(path.clone(), "A".into(), Graph::default().to_ron().unwrap()).unwrap();
        let antes = fs::read_to_string(dir.join(SCRIPT_DIR).join("A.logic.ron")).unwrap();
        assert!(save_script(path.clone(), "A".into(), "isto não é um grafo (".into()).is_err());
        assert_eq!(fs::read_to_string(dir.join(SCRIPT_DIR).join("A.logic.ron")).unwrap(), antes);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn criar_recusa_nome_repetido_e_invalido() {
        let dir = projeto("script-criar");
        let path = caminho(&dir);
        assert_eq!(create_script(path.clone(), " Script 1 ".into()).unwrap(), "Script 1");
        assert!(create_script(path.clone(), "script 1".into()).is_err(), "no Windows, nomes sem caixa são o mesmo arquivo");
        assert!(create_script(path.clone(), "a/b".into()).is_err());
        assert!(create_script(path.clone(), "   ".into()).is_err());
        assert!(create_script(path.clone(), ".oculto".into()).is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn renomear_e_duplicar_mantem_os_nomes_certos() {
        let dir = projeto("script-renomear");
        let path = caminho(&dir);
        create_script(path.clone(), "Pular".into()).unwrap();
        create_script(path.clone(), "Correr".into()).unwrap();
        assert!(rename_script(path.clone(), "Pular".into(), "Correr".into()).is_err());
        rename_script(path.clone(), "Pular".into(), "Saltar".into()).unwrap();
        assert_eq!(list_scripts(path.clone()).unwrap(), vec!["Correr", "Saltar"]);
        assert_eq!(duplicate_script(path.clone(), "Saltar".into()).unwrap(), "Saltar cópia");
        assert_eq!(duplicate_script(path.clone(), "Saltar".into()).unwrap(), "Saltar cópia 2");
        assert_eq!(list_scripts(path).unwrap().len(), 4);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pasta_que_nao_e_projeto_e_recusada() {
        let dir = testes::pasta_temporaria("script-sem-projeto");
        assert!(list_scripts(caminho(&dir)).is_err());
        assert!(create_script(caminho(&dir), "X".into()).is_err());
        assert!(!dir.join(SCRIPT_DIR).exists());
        fs::remove_dir_all(&dir).ok();
    }
}
