//! Arquivos de cena: RON no formato 2 (SCN-004) e migração automática do formato 1 da Fase 0 (SCN-001).
//! Os nós são gravados em pré-ordem e só as propriedades não padrão aparecem, então o diff de um arquivo
//! mostra só o que mudou.

use crate::registry;
use crate::scene::{Connection, Node, Scene, Uid, check_link, normalize_hex};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

/// Versão atual do formato de arquivo de cena. O formato 3 guarda as ligações de sinais (SCN-034). A versão mudou
/// porque `NodeFile` ignora campos desconhecidos: sem o número novo, um editor antigo salvaria a cena sem as ligações.
pub const FORMAT: u32 = 3;

#[derive(Serialize, Deserialize)]
struct SceneFile {
    format: u32,
    name: String,
    screen: [u32; 2],
    nodes: Vec<NodeFile>,
}

#[derive(Serialize, Deserialize)]
struct NodeFile {
    uid: Uid,
    name: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<Uid>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    props: BTreeMap<String, Value>,
    /// Ligações de sinais deste nó como emissor (formato 3). Ausente nos arquivos anteriores.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    connections: Vec<Connection>,
}

impl Scene {
    /// Texto RON da cena, no formato atual.
    pub fn to_ron(&self) -> Result<String, String> {
        let nodes = self
            .pre_order()
            .into_iter()
            .filter_map(|uid| {
                let n = self.node(uid)?;
                Some(NodeFile {
                    uid,
                    name: n.name.clone(),
                    kind: n.kind.clone(),
                    parent: n.parent,
                    props: n.props.clone(),
                    connections: n.connections.clone(),
                })
            })
            .collect();
        let file = SceneFile { format: FORMAT, name: self.name.clone(), screen: self.screen, nodes };
        let cfg = ron::ser::PrettyConfig::new().struct_names(false);
        ron::ser::to_string_pretty(&file, cfg).map_err(|e| e.to_string())
    }

    /// Lê um texto de cena: RON no formato atual, ou o JSON/RON do formato 1 (migrado).
    /// `seed` alimenta a geração de UIDs para os nós migrados e para os novos.
    pub fn from_text(text: &str, seed: u64) -> Result<Scene, String> {
        let t = text.trim();
        if t.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(t) {
                return Scene::from_value(&v, seed);
            }
        }
        if let Ok(file) = ron::from_str::<SceneFile>(t) {
            return scene_from_file(file, seed);
        }
        let v: Value = ron::from_str(t).map_err(|e| format!("Cena ilegível: {e}"))?;
        Scene::from_value(&v, seed)
    }

    /// Lê uma cena do formato 1 (objetos planos da Fase 0) a partir de um valor JSON.
    pub fn from_value(v: &Value, seed: u64) -> Result<Scene, String> {
        if v.get("version").and_then(Value::as_u64) == Some(1) {
            return migrate_v1(v, seed);
        }
        Err("Formato de cena não reconhecido".into())
    }
}

fn scene_from_file(file: SceneFile, seed: u64) -> Result<Scene, String> {
    if file.format > FORMAT {
        return Err(format!(
            "A cena usa o formato {} e esta versão do Nexo lê até o formato {FORMAT}",
            file.format
        ));
    }
    let mut scene = Scene::new(&file.name, seed);
    scene.screen = file.screen;
    for nf in &file.nodes {
        registry::find(&nf.kind).ok_or_else(|| format!("Tipo de nó desconhecido: {}", nf.kind))?;
        if scene.node(nf.uid).is_some() {
            return Err(format!("UID repetido na cena: {}", nf.uid));
        }
        scene.nodes.insert(
            nf.uid,
            Node {
                name: nf.name.clone(),
                kind: nf.kind.clone(),
                parent: nf.parent,
                children: Vec::new(),
                props: BTreeMap::new(),
                connections: Vec::new(),
            },
        );
    }
    // Pré-ordem: a ordem do arquivo já é a ordem dos irmãos.
    for nf in &file.nodes {
        match nf.parent {
            Some(p) => {
                let parent = scene.nodes.get_mut(&p).ok_or_else(|| format!("Pai ausente na cena: {p}"))?;
                parent.children.push(nf.uid);
            }
            None => scene.roots.push(nf.uid),
        }
    }
    check_tree(&scene)?;
    for nf in file.nodes {
        check_links(&nf.kind, &nf.connections)?;
        if let Some(node) = scene.nodes.get_mut(&nf.uid) {
            node.connections = nf.connections;
        }
        for (key, value) in nf.props {
            scene.put_raw(nf.uid, &key, value)?;
        }
    }
    Ok(scene)
}

/// Confere as ligações de um nó lido: o sinal precisa ser um que o tipo emite, o método precisa ser válido e nenhuma
/// ligação pode se repetir. O alvo não é conferido: se ele não existe, a interface avisa e a ligação fica no arquivo.
fn check_links(kind: &str, links: &[Connection]) -> Result<(), String> {
    for (i, link) in links.iter().enumerate() {
        check_link(kind, &link.signal, &link.method)?;
        if links[..i].contains(link) {
            return Err(format!("Ligação repetida no nó {kind}: {} → {}", link.signal, link.method));
        }
    }
    Ok(())
}

/// Confere que a árvore é uma floresta válida: sem ciclos e com todos os nós alcançáveis.
fn check_tree(scene: &Scene) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut stack: Vec<Uid> = scene.roots().to_vec();
    while let Some(u) = stack.pop() {
        if !seen.insert(u) {
            return Err(format!("Árvore inconsistente no nó {u}"));
        }
        let n = scene.node(u).ok_or_else(|| format!("Nó ausente: {u}"))?;
        stack.extend(n.children.iter().copied());
    }
    if seen.len() != scene.len() {
        return Err("Há nós que não pertencem a nenhuma árvore".into());
    }
    Ok(())
}

/// Formato 1 da Fase 0: uma lista plana de objetos com forma, cor, comportamentos e tamanho.
/// Cada objeto vira um `Sprite2D` filho de um `Node2D` raiz; cada comportamento vira um nó `Behavior`.
fn migrate_v1(v: &Value, seed: u64) -> Result<Scene, String> {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("Principal");
    let mut scene = Scene::new(name, seed);
    if let Some(s) = v.get("screen").and_then(Value::as_array) {
        if let (Some(w), Some(h)) = (s.first().and_then(Value::as_u64), s.get(1).and_then(Value::as_u64)) {
            scene.screen = [w as u32, h as u32];
        }
    }
    let root = scene.add_raw(None, "Node2D", name);
    let [sw, sh] = scene.screen;
    let cam = scene.add_raw(Some(root), "Camera2D", "Câmera principal");
    scene.put_raw(cam, "position", json!([sw as f64 / 2.0, sh as f64 / 2.0]))?;

    let objects = v.get("objects").and_then(Value::as_array).cloned().unwrap_or_default();
    for o in &objects {
        let oname = o.get("name").and_then(Value::as_str).unwrap_or("Objeto");
        let num = |k: &str, d: f64| o.get(k).and_then(Value::as_f64).unwrap_or(d);
        let uid = scene.add_raw(Some(root), "Sprite2D", oname);
        scene.put_raw(uid, "position", json!([num("x", 0.0), num("y", 0.0)]))?;
        scene.put_raw(uid, "rotation", json!(num("rotation", 0.0)))?;
        scene.put_raw(uid, "size", json!([num("w", 96.0), num("h", 96.0)]))?;
        let shape = match o.get("shape").and_then(Value::as_u64) {
            Some(1) => "circle",
            Some(2) => "diamond",
            _ => "rect",
        };
        scene.put_raw(uid, "shape", json!(shape))?;
        let color = o.get("color").and_then(Value::as_str).and_then(normalize_hex);
        scene.put_raw(uid, "modulate", json!(color.unwrap_or_else(|| "#ffffff".into())))?;
        if o.get("visible").and_then(Value::as_bool) == Some(false) {
            scene.put_raw(uid, "visible", json!(false))?;
        }
        if o.get("locked").and_then(Value::as_bool) == Some(true) {
            scene.put_raw(uid, "locked", json!(true))?;
        }
        for b in o.get("behaviors").and_then(Value::as_array).into_iter().flatten() {
            let kind = match b.get("kind").and_then(Value::as_str) {
                Some("flutuar") => "float",
                Some("pulsar") => "pulse",
                _ => "spin",
            };
            let label = match kind {
                "float" => "Flutuar",
                "pulse" => "Pulsar",
                _ => "Girar",
            };
            let behavior = scene.add_raw(Some(uid), "Behavior", label);
            scene.put_raw(behavior, "kind", json!(kind))?;
            scene.put_raw(behavior, "speed", json!(b.get("speed").and_then(Value::as_f64).unwrap_or(1.0)))?;
        }
    }
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cena com um Timer ligado a um Sprite2D. Devolve o texto RON e os UIDs do emissor e do alvo.
    fn text_with_link() -> (String, Uid, Uid) {
        let mut s = Scene::new_default("Principal", 3);
        let root = s.roots()[0];
        let timer = s.add_node(Some(root), "Timer", None).unwrap();
        let sprite = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.connect(timer, "timeout", sprite, "on_timeout").unwrap();
        (s.to_ron().unwrap(), timer, sprite)
    }

    #[test]
    fn ligacoes_vao_e_voltam_no_formato_3() {
        let (text, timer, sprite) = text_with_link();
        assert!(text.contains("format: 3"));
        let back = Scene::from_text(&text, 9).unwrap();
        assert_eq!(
            back.node(timer).unwrap().connections,
            vec![Connection { signal: "timeout".into(), target: sprite, method: "on_timeout".into() }]
        );
        // Ler e gravar de novo não muda o texto: a ordem e os UIDs se mantêm.
        assert_eq!(back.to_ron().unwrap(), text);
    }

    #[test]
    fn no_sem_ligacoes_nao_grava_o_campo() {
        let s = Scene::new_default("Principal", 3);
        assert!(!s.to_ron().unwrap().contains("connections"));
    }

    #[test]
    fn formato_2_carrega_sem_ligacoes_e_regrava_no_3() {
        let text = r#"(
    format: 2,
    name: "Antiga",
    screen: (1280, 720),
    nodes: [
        (uid: "0000000000000001", name: "Raiz", kind: "Node2D"),
        (uid: "0000000000000002", name: "Timer", kind: "Timer", parent: Some("0000000000000001")),
    ],
)"#;
        let s = Scene::from_text(text, 5).unwrap();
        assert!(s.node(Uid(2)).unwrap().connections.is_empty());
        assert!(s.to_ron().unwrap().contains("format: 3"));
    }

    #[test]
    fn formato_4_e_recusado() {
        let (text, _, _) = text_with_link();
        let futuro = text.replace("format: 3", "format: 4");
        let err = Scene::from_text(&futuro, 9).err().expect("formato 4 deve ser recusado");
        assert!(err.contains("formato 4"), "{err}");
    }

    #[test]
    fn sinal_que_o_tipo_nao_emite_e_recusado_no_arquivo() {
        let (text, _, _) = text_with_link();
        let trocado = text.replace("signal: \"timeout\"", "signal: \"body_entered\"");
        assert!(Scene::from_text(&trocado, 9).is_err());
    }

    #[test]
    fn metodo_invalido_e_recusado_no_arquivo() {
        let (text, _, _) = text_with_link();
        let trocado = text.replace("method: \"on_timeout\"", "method: \"on timeout\"");
        assert!(Scene::from_text(&trocado, 9).is_err());
    }

    #[test]
    fn ligacao_repetida_e_recusada_no_arquivo() {
        let (text, _, sprite) = text_with_link();
        let entrada = format!("(signal: \"timeout\", target: \"{sprite}\", method: \"on_timeout\")");
        let repetido = text.replace("connections: [", &format!("connections: [{entrada}, "));
        assert!(Scene::from_text(&repetido, 9).is_err());
    }

    #[test]
    fn alvo_ausente_carrega_e_fica_na_ligacao() {
        let (text, timer, sprite) = text_with_link();
        // Só o alvo da ligação muda para um UID que não existe na cena; o UID do nó continua o mesmo.
        let trocado = text.replace(&format!("target: \"{sprite}\""), "target: \"00000000000000ff\"");
        let s = Scene::from_text(&trocado, 9).unwrap();
        assert_eq!(s.node(timer).unwrap().connections[0].target, Uid(0xff));
        assert!(s.node(Uid(0xff)).is_none());
    }
}
