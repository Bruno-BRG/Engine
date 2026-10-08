//! Árvore de nós da cena: UIDs estáveis, hierarquia, propriedades e desfazer/refazer (SCN-001, SCN-003).
//! É a fonte da verdade da cena; o editor só lê e envia operações.

use crate::registry::{self, PropDef, PropKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::str::FromStr;

const MAX_HISTORY: usize = 200;
/// Deslocamento aplicado a cópias e colagens, para não cobrirem o original.
const PASTE_OFFSET: f64 = 32.0;
/// Maximum length of a connection's method name.
const MAX_METHOD_LEN: usize = 64;

/// Identificador estável de um nó. Vai para o arquivo como 16 dígitos hexadecimais,
/// porque números de 64 bits perdem precisão em JavaScript.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Uid(pub u64);

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl FromStr for Uid {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(s, 16)
            .map(Uid)
            .map_err(|_| format!("UID inválido: {s}"))
    }
}

impl Serialize for Uid {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Uid {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// A connection from a node's signal to a target node (SCN-034). Stored on the emitter, outside `props`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connection {
    /// Name of the signal the emitter emits, e.g. "timeout" or "body_entered".
    pub signal: String,
    /// Node that receives the signal.
    pub target: Uid,
    /// Name of the method (event) on the target's script that runs when the signal arrives.
    pub method: String,
}

/// Um nó da cena. `props` guarda só os valores que diferem do padrão do tipo.
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub kind: String,
    pub parent: Option<Uid>,
    pub children: Vec<Uid>,
    pub props: BTreeMap<String, Value>,
    /// Connections from this node as emitter, in the order they were made.
    pub connections: Vec<Connection>,
}

#[derive(Clone)]
struct Snapshot {
    label: String,
    nodes: BTreeMap<Uid, Node>,
    roots: Vec<Uid>,
}

pub struct Scene {
    pub name: String,
    /// Resolução da tela do jogo, em pixels.
    pub screen: [u32; 2],
    pub(crate) nodes: BTreeMap<Uid, Node>,
    pub(crate) roots: Vec<Uid>,
    rng: u64,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Chave de agrupamento ativa: operações com a mesma chave viram um só passo de desfazer.
    group: Option<String>,
    clipboard: Option<(Uid, BTreeMap<Uid, Node>)>,
}

impl Scene {
    /// Cena vazia. `seed` alimenta a geração de UIDs; use valores diferentes para cenas diferentes.
    pub fn new(name: &str, seed: u64) -> Scene {
        Scene {
            name: name.to_string(),
            screen: [1280, 720],
            nodes: BTreeMap::new(),
            roots: Vec::new(),
            rng: seed,
            undo: Vec::new(),
            redo: Vec::new(),
            group: None,
            clipboard: None,
        }
    }

    /// Cena nova com um nó raiz `Node2D` e uma `Camera2D` centralizada na tela do jogo.
    pub fn new_default(name: &str, seed: u64) -> Scene {
        let mut s = Scene::new(name, seed);
        let root = s.add_raw(None, "Node2D", name);
        let cam = s.add_raw(Some(root), "Camera2D", "Câmera principal");
        let [w, h] = s.screen;
        s.put_raw(cam, "position", json!([w as f64 / 2.0, h as f64 / 2.0]))
            .expect("posição padrão é válida");
        s
    }

    /// Cópia para a reprodução: mesma árvore, sem histórico nem área de transferência.
    pub fn play_copy(&self) -> Scene {
        Scene {
            name: self.name.clone(),
            screen: self.screen,
            nodes: self.nodes.clone(),
            roots: self.roots.clone(),
            rng: self.rng,
            undo: Vec::new(),
            redo: Vec::new(),
            group: None,
            clipboard: None,
        }
    }

    // ---------- Consultas ----------

    pub fn node(&self, uid: Uid) -> Option<&Node> {
        self.nodes.get(&uid)
    }

    pub fn roots(&self) -> &[Uid] {
        &self.roots
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Todos os nós em ordem de árvore (pai antes dos filhos, irmãos na ordem em que aparecem).
    pub fn pre_order(&self) -> Vec<Uid> {
        order_in(&self.nodes, &self.roots)
    }

    /// O nó e seus descendentes, em ordem de árvore.
    pub fn subtree(&self, root: Uid) -> Vec<Uid> {
        order_in(&self.nodes, &[root])
    }

    pub fn index_in_parent(&self, uid: Uid) -> Option<usize> {
        let parent = self.nodes.get(&uid)?.parent;
        self.siblings(parent).iter().position(|u| *u == uid)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|s| s.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    pub fn has_clipboard(&self) -> bool {
        self.clipboard.is_some()
    }

    // ---------- Operações de edição (cada uma gera um passo de desfazer) ----------

    /// Cria um nó do tipo `kind` como filho de `parent` (ou raiz), na posição `index`.
    pub fn add_node(&mut self, parent: Option<Uid>, kind: &str, index: Option<usize>) -> Result<Uid, String> {
        check_kind(kind)?;
        check_parent(self, parent)?;
        self.begin(&format!("Criar {kind}"), None);
        let name = self.unique_name(parent, kind, None);
        let uid = self.next_uid();
        self.nodes.insert(
            uid,
            Node {
                name,
                kind: kind.to_string(),
                parent,
                children: Vec::new(),
                props: BTreeMap::new(),
                connections: Vec::new(),
            },
        );
        self.attach(uid, parent, index);
        Ok(uid)
    }

    /// Removes the node and everything below it. Connections that point to those nodes are removed too (SCN-034).
    pub fn remove_node(&mut self, uid: Uid) -> Result<(), String> {
        let name = self.node_or_err(uid)?.name.clone();
        self.begin(&format!("Excluir {name}"), None);
        self.detach(uid);
        let removed = self.subtree(uid);
        for u in &removed {
            self.nodes.remove(u);
        }
        self.drop_links_to(&removed);
        Ok(())
    }

    /// Connects signal `signal` of node `emitter` to node `target`, calling `method` (SCN-034). The method's
    /// leading and trailing spaces are trimmed. Refuses a signal the emitter's type does not emit, an invalid
    /// method, a missing target and a duplicate connection.
    pub fn connect(&mut self, emitter: Uid, signal: &str, target: Uid, method: &str) -> Result<(), String> {
        let method = method.trim();
        let kind = self.node_or_err(emitter)?.kind.clone();
        check_link(&kind, signal, method)?;
        if !self.nodes.contains_key(&target) {
            return Err("O nó alvo não existe mais".into());
        }
        let link = Connection { signal: signal.to_string(), target, method: method.to_string() };
        if self.node_or_err(emitter)?.connections.contains(&link) {
            return Err("Essa ligação já existe".into());
        }
        self.begin("Conectar sinal", None);
        if let Some(n) = self.nodes.get_mut(&emitter) {
            n.connections.push(link);
        }
        Ok(())
    }

    /// Removes the connection from `emitter` with the same signal, target and method (SCN-034).
    pub fn disconnect(&mut self, emitter: Uid, signal: &str, target: Uid, method: &str) -> Result<(), String> {
        let link = Connection { signal: signal.to_string(), target, method: method.trim().to_string() };
        let Some(i) = self.node_or_err(emitter)?.connections.iter().position(|c| *c == link) else {
            return Err("Ligação não encontrada".into());
        };
        self.begin("Desconectar sinal", None);
        if let Some(n) = self.nodes.get_mut(&emitter) {
            n.connections.remove(i);
        }
        Ok(())
    }

    /// Renomeia o nó. Se o nome já existe entre os irmãos, recebe um número ("Sprite2D 2").
    pub fn rename_node(&mut self, uid: Uid, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("O nome não pode ficar vazio".into());
        }
        let parent = self.node_or_err(uid)?.parent;
        let unique = self.unique_name(parent, name, Some(uid));
        if self.node_or_err(uid)?.name == unique {
            return Ok(());
        }
        self.begin("Renomear", None);
        if let Some(n) = self.nodes.get_mut(&uid) {
            n.name = unique;
        }
        Ok(())
    }

    /// Define uma propriedade. Com `coalesce`, alterações seguidas com a mesma chave viram um passo só
    /// (arrastar um gizmo, arrastar um número no Inspector) até `end_group`.
    pub fn set_prop(&mut self, uid: Uid, key: &str, value: Value, coalesce: Option<&str>) -> Result<(), String> {
        let kind = self.node_or_err(uid)?.kind.clone();
        let def = registry::prop_def(&kind, key).ok_or_else(|| format!("Propriedade desconhecida: {key}"))?;
        let value = coerce(def, &value)?;
        if value_of(self.node_or_err(uid)?, key) == value {
            return Ok(());
        }
        self.begin(&format!("Editar {}", def.label), coalesce);
        self.store(uid, def, value);
        Ok(())
    }

    /// Replaces `from` with `to` in the text property `key` of every node that holds exactly that value, as
    /// when a physics material is renamed (PHY-023). Other nodes are untouched. The whole replacement is one
    /// undo step; when no node changes, nothing enters the history. Returns how many nodes changed.
    /// An empty `from` matches nothing: nodes without a material hold the empty default, which is not a name.
    pub fn rename_prop_value(&mut self, key: &str, from: &str, to: &str) -> Result<usize, String> {
        let to = to.trim();
        if to.is_empty() {
            return Err("O nome não pode ficar vazio".into());
        }
        if from.trim().is_empty() {
            return Ok(0);
        }
        let targets: Vec<(Uid, &'static PropDef)> = self
            .nodes
            .iter()
            .filter_map(|(uid, n)| {
                let def = registry::prop_def(&n.kind, key).filter(|d| d.kind == PropKind::Str)?;
                (value_of(n, key) == json!(from)).then_some((*uid, def))
            })
            .collect();
        if targets.is_empty() || from == to {
            return Ok(0);
        }
        // The new name is validated before the first change, so the replacement is never half-applied.
        let value = coerce(targets[0].1, &json!(to))?;
        self.begin(&format!("Renomear {}", targets[0].1.label), None);
        for (uid, def) in &targets {
            self.store(*uid, def, value.clone());
        }
        Ok(targets.len())
    }

    /// Move o nó para `parent` (ou raiz), na posição final `index` entre os irmãos.
    pub fn move_node(&mut self, uid: Uid, parent: Option<Uid>, index: usize) -> Result<(), String> {
        let node = self.node_or_err(uid)?;
        let (name, old_parent) = (node.name.clone(), node.parent);
        check_parent(self, parent)?;
        if let Some(p) = parent {
            if self.subtree(uid).contains(&p) {
                return Err("Não é possível mover um nó para dentro de si mesmo".into());
            }
        }
        let old_index = self.index_in_parent(uid);
        if old_parent == parent && old_index == Some(index) {
            return Ok(());
        }
        let label = if old_parent == parent { format!("Reordenar {name}") } else { format!("Mover {name}") };
        self.begin(&label, None);
        self.detach(uid);
        self.attach(uid, parent, Some(index));
        Ok(())
    }

    /// Cria uma cópia do nó logo após ele, com novos UIDs. Retorna o UID da cópia.
    pub fn duplicate_node(&mut self, uid: Uid) -> Result<Uid, String> {
        let node = self.node_or_err(uid)?;
        let (parent, name) = (node.parent, node.name.clone());
        let index = self.index_in_parent(uid).map(|i| i + 1);
        let frag = self.fragment(uid);
        self.begin(&format!("Duplicar {name}"), None);
        let new = self.instantiate(&frag, uid, parent, index);
        self.nudge(new);
        Ok(new)
    }

    /// Guarda o nó e seus descendentes na área de transferência. Não altera a cena.
    pub fn copy_node(&mut self, uid: Uid) -> Result<(), String> {
        self.node_or_err(uid)?;
        self.clipboard = Some((uid, self.fragment(uid)));
        Ok(())
    }

    /// Cola a área de transferência como filho de `parent` (ou raiz). Retorna o UID colado, se houver.
    pub fn paste_node(&mut self, parent: Option<Uid>) -> Result<Option<Uid>, String> {
        let Some((root, frag)) = self.clipboard.clone() else {
            return Ok(None);
        };
        check_parent(self, parent)?;
        let name = frag[&root].name.clone();
        self.begin(&format!("Colar {name}"), None);
        let new = self.instantiate(&frag, root, parent, None);
        self.nudge(new);
        Ok(Some(new))
    }

    /// Changes the node's type. Properties the new type does not have are dropped, and so are the signal
    /// connections the new type does not emit (SCN-034).
    pub fn change_kind(&mut self, uid: Uid, kind: &str) -> Result<(), String> {
        check_kind(kind)?;
        if self.node_or_err(uid)?.kind == kind {
            return Ok(());
        }
        self.begin(&format!("Trocar tipo para {kind}"), None);
        let Some(n) = self.nodes.get_mut(&uid) else {
            return Ok(());
        };
        n.kind = kind.to_string();
        let kept: BTreeMap<String, Value> = n
            .props
            .iter()
            .filter_map(|(k, v)| {
                let def = registry::prop_def(kind, k)?;
                Some((k.clone(), coerce(def, v).ok()?))
            })
            .collect();
        n.props = kept;
        n.connections.retain(|c| registry::signal_def(kind, &c.signal).is_some());
        Ok(())
    }

    /// New scene with the subtree of `uid` as its root (SCN-022: "Salvar ramo como cena").
    /// UIDs are kept, so references inside the branch stay valid. Connections to nodes outside the branch are
    /// dropped in the copy, because the target would not exist there (SCN-034).
    pub fn branch_as_scene(&self, uid: Uid, seed: u64) -> Result<Scene, String> {
        let node = self.node_or_err(uid)?;
        let mut scene = Scene::new(&node.name, seed);
        scene.screen = self.screen;
        let frag = self.fragment(uid);
        let inside: BTreeSet<Uid> = frag.keys().copied().collect();
        for (u, mut n) in frag {
            n.connections.retain(|c| inside.contains(&c.target));
            let parent = if u == uid { None } else { n.parent };
            scene.nodes.insert(u, Node { parent, ..n });
        }
        scene.roots.push(uid);
        Ok(scene)
    }

    // ---------- Desfazer e refazer ----------

    /// Fecha o agrupamento atual. Chame ao soltar o mouse ou sair de um campo numérico.
    pub fn end_group(&mut self) {
        self.group = None;
    }

    /// Desfaz o último passo. Retorna o nome do passo desfeito.
    pub fn undo(&mut self) -> Option<String> {
        let step = self.undo.pop()?;
        let label = step.label.clone();
        let current = Snapshot {
            label: step.label,
            nodes: std::mem::take(&mut self.nodes),
            roots: std::mem::take(&mut self.roots),
        };
        self.redo.push(current);
        self.nodes = step.nodes;
        self.roots = step.roots;
        self.group = None;
        Some(label)
    }

    /// Refaz o último passo desfeito. Retorna o nome do passo refeito.
    pub fn redo(&mut self) -> Option<String> {
        let step = self.redo.pop()?;
        let label = step.label.clone();
        let current = Snapshot {
            label: step.label,
            nodes: std::mem::take(&mut self.nodes),
            roots: std::mem::take(&mut self.roots),
        };
        self.undo.push(current);
        self.nodes = step.nodes;
        self.roots = step.roots;
        self.group = None;
        Some(label)
    }

    // ---------- Internos ----------

    pub(crate) fn node_or_err(&self, uid: Uid) -> Result<&Node, String> {
        self.nodes.get(&uid).ok_or_else(|| format!("Nó não encontrado: {uid}"))
    }

    pub(crate) fn siblings(&self, parent: Option<Uid>) -> &[Uid] {
        match parent {
            Some(p) => self.nodes.get(&p).map(|n| n.children.as_slice()).unwrap_or(&[]),
            None => &self.roots,
        }
    }

    /// Registra o estado atual antes de uma mudança, a menos que a mudança continue o agrupamento.
    fn begin(&mut self, label: &str, coalesce: Option<&str>) {
        let merge = matches!((coalesce, self.group.as_deref()), (Some(a), Some(b)) if a == b);
        if !merge {
            self.undo.push(Snapshot {
                label: label.to_string(),
                nodes: self.nodes.clone(),
                roots: self.roots.clone(),
            });
            if self.undo.len() > MAX_HISTORY {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.group = coalesce.map(str::to_string);
    }

    fn next_uid(&mut self) -> Uid {
        loop {
            // SplitMix64: sequência determinística a partir da semente, sem dependência de sistema.
            self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.rng;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            let uid = Uid(z);
            if z != 0 && !self.nodes.contains_key(&uid) {
                return uid;
            }
        }
    }

    /// Nome livre entre os irmãos: `base`, ou `base 2`, `base 3`… se já houver.
    fn unique_name(&self, parent: Option<Uid>, base: &str, except: Option<Uid>) -> String {
        let taken: Vec<&str> = self
            .siblings(parent)
            .iter()
            .filter(|u| Some(**u) != except)
            .filter_map(|u| self.nodes.get(u))
            .map(|n| n.name.as_str())
            .collect();
        if !taken.contains(&base) {
            return base.to_string();
        }
        let mut i = 2;
        loop {
            let candidate = format!("{base} {i}");
            if !taken.contains(&candidate.as_str()) {
                return candidate;
            }
            i += 1;
        }
    }

    /// Tira o nó da lista de filhos do pai (ou das raízes). Não remove o nó do mapa.
    fn detach(&mut self, uid: Uid) {
        let parent = self.nodes.get(&uid).and_then(|n| n.parent);
        let list: Option<&mut Vec<Uid>> = match parent {
            Some(p) => self.nodes.get_mut(&p).map(|n| &mut n.children),
            None => Some(&mut self.roots),
        };
        if let Some(list) = list {
            list.retain(|u| *u != uid);
        }
    }

    /// Insere o nó na lista de filhos do pai (ou das raízes), na posição `index` (fim por padrão).
    fn attach(&mut self, uid: Uid, parent: Option<Uid>, index: Option<usize>) {
        if let Some(n) = self.nodes.get_mut(&uid) {
            n.parent = parent;
        }
        let list: Option<&mut Vec<Uid>> = match parent {
            Some(p) => self.nodes.get_mut(&p).map(|n| &mut n.children),
            None => Some(&mut self.roots),
        };
        if let Some(list) = list {
            let i = index.unwrap_or(list.len()).min(list.len());
            list.insert(i, uid);
        }
    }

    fn store(&mut self, uid: Uid, def: &PropDef, value: Value) {
        if let Some(n) = self.nodes.get_mut(&uid) {
            if value == def.default {
                n.props.remove(def.name);
            } else {
                n.props.insert(def.name.to_string(), value);
            }
        }
    }

    /// Cópia de um subconjunto da árvore, com os UIDs originais.
    fn fragment(&self, root: Uid) -> BTreeMap<Uid, Node> {
        self.subtree(root)
            .into_iter()
            .filter_map(|u| self.nodes.get(&u).map(|n| (u, n.clone())))
            .collect()
    }

    /// Cria os nós de `frag` (a partir de `root`) com UIDs novos e os liga a `parent`.
    fn instantiate(
        &mut self,
        frag: &BTreeMap<Uid, Node>,
        root: Uid,
        parent: Option<Uid>,
        index: Option<usize>,
    ) -> Uid {
        let order = order_in(frag, &[root]);
        let mut map: HashMap<Uid, Uid> = HashMap::new();
        for old in &order {
            let new = self.next_uid();
            map.insert(*old, new);
        }
        let root_name = self.unique_name(parent, &frag[&root].name, None);
        for old in &order {
            let src = &frag[old];
            let new_parent = if *old == root {
                parent
            } else {
                src.parent.and_then(|p| map.get(&p).copied())
            };
            let children = src.children.iter().filter_map(|c| map.get(c).copied()).collect();
            let name = if *old == root { root_name.clone() } else { src.name.clone() };
            // A target inside the copy points at the copy; a target outside it stays the same node (SCN-034).
            let connections = src
                .connections
                .iter()
                .map(|c| Connection { target: map.get(&c.target).copied().unwrap_or(c.target), ..c.clone() })
                .collect();
            self.nodes.insert(
                map[old],
                Node {
                    name,
                    kind: src.kind.clone(),
                    parent: new_parent,
                    children,
                    props: src.props.clone(),
                    connections,
                },
            );
        }
        let new_root = map[&root];
        self.attach(new_root, parent, index);
        new_root
    }

    /// Desloca um nó 2D colado ou duplicado, para que a cópia apareça ao lado do original.
    fn nudge(&mut self, uid: Uid) {
        let Some(n) = self.nodes.get(&uid) else {
            return;
        };
        if !registry::is_2d(&n.kind) {
            return;
        }
        let kind = n.kind.clone();
        let [x, y] = as_vec2(&value_of(n, "position"));
        if let Some(def) = registry::prop_def(&kind, "position") {
            self.store(uid, def, json!([x + PASTE_OFFSET, y + PASTE_OFFSET]));
        }
    }

    /// Removes the connections that point to any of the `removed` nodes, from every node in the scene. Records no
    /// history: the caller already recorded the operation, and undo restores the whole previous state.
    fn drop_links_to(&mut self, removed: &[Uid]) {
        for n in self.nodes.values_mut() {
            n.connections.retain(|c| !removed.contains(&c.target));
        }
    }

    /// Adiciona um nó sem histórico. Usado pela migração e pela cena padrão.
    pub(crate) fn add_raw(&mut self, parent: Option<Uid>, kind: &str, name: &str) -> Uid {
        let uid = self.next_uid();
        self.nodes.insert(
            uid,
            Node {
                name: name.to_string(),
                kind: kind.to_string(),
                parent,
                children: Vec::new(),
                props: BTreeMap::new(),
                connections: Vec::new(),
            },
        );
        self.attach(uid, parent, None);
        uid
    }

    /// Define uma propriedade sem histórico, validando o valor. Usado ao carregar arquivos.
    pub(crate) fn put_raw(&mut self, uid: Uid, key: &str, value: Value) -> Result<(), String> {
        let kind = self.node_or_err(uid)?.kind.clone();
        let def = registry::prop_def(&kind, key).ok_or_else(|| format!("Propriedade desconhecida: {key}"))?;
        let value = coerce(def, &value)?;
        self.store(uid, def, value);
        Ok(())
    }

    /// Define uma propriedade sem histórico. Só a cópia de reprodução usa isto: a física grava as
    /// poses a cada quadro, e isso não deve encher a fila de desfazer.
    pub fn set_runtime_prop(&mut self, uid: Uid, key: &str, value: Value) -> Result<(), String> {
        self.put_raw(uid, key, value)
    }

    /// Remove o nó e tudo abaixo dele sem histórico. Usado por Destruir nó na reprodução (VIS-023): a cópia de
    /// reprodução não tem desfazer, e cada remoção com histórico copiaria a árvore inteira.
    pub fn remove_runtime(&mut self, uid: Uid) -> Result<(), String> {
        self.node_or_err(uid)?;
        self.detach(uid);
        let removed = self.subtree(uid);
        for u in &removed {
            self.nodes.remove(u);
        }
        self.drop_links_to(&removed);
        Ok(())
    }
}

fn check_kind(kind: &str) -> Result<(), String> {
    registry::find(kind).map(|_| ()).ok_or_else(|| format!("Tipo de nó desconhecido: {kind}"))
}

/// Checks a connection against the emitter's type: the signal must be one the type emits, and the method a
/// valid name. Used when the connection is created and when a file is loaded (SCN-034).
pub(crate) fn check_link(kind: &str, signal: &str, method: &str) -> Result<(), String> {
    if registry::signal_def(kind, signal).is_none() {
        return Err(format!("O tipo {kind} não emite o sinal {signal}"));
    }
    check_method(method)
}

/// Method name: starts with a letter or `_`, continues with unaccented letters, digits or `_`, up to 64 characters.
fn check_method(method: &str) -> Result<(), String> {
    let mut chars = method.chars();
    let starts_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let rest_ok = chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if starts_ok && rest_ok && method.len() <= MAX_METHOD_LEN {
        return Ok(());
    }
    Err(format!(
        "Nome de método inválido: “{method}”. Use letras sem acento, números e _, começando por letra ou _ (até {MAX_METHOD_LEN} caracteres)."
    ))
}

fn check_parent(scene: &Scene, parent: Option<Uid>) -> Result<(), String> {
    match parent {
        Some(p) => scene.node_or_err(p).map(|_| ()),
        None => Ok(()),
    }
}

/// Pré-ordem a partir de uma lista de raízes.
fn order_in(nodes: &BTreeMap<Uid, Node>, roots: &[Uid]) -> Vec<Uid> {
    let mut out = Vec::new();
    let mut stack: Vec<Uid> = roots.iter().rev().copied().collect();
    while let Some(u) = stack.pop() {
        out.push(u);
        if let Some(n) = nodes.get(&u) {
            stack.extend(n.children.iter().rev().copied());
        }
    }
    out
}

/// Valor de uma propriedade: o valor guardado, ou o padrão do tipo.
pub fn value_of(node: &Node, key: &str) -> Value {
    node.props
        .get(key)
        .cloned()
        .or_else(|| registry::default_value(&node.kind, key))
        .unwrap_or(Value::Null)
}

/// Converte um valor [x, y] (ou nulo) em par de números.
pub fn as_vec2(v: &Value) -> [f64; 2] {
    match v.as_array() {
        Some(a) if a.len() == 2 => [a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0)],
        _ => [0.0, 0.0],
    }
}

/// Valida e normaliza um valor de acordo com o tipo da propriedade.
pub(crate) fn coerce(def: &PropDef, v: &Value) -> Result<Value, String> {
    let bad = || format!("Valor inválido para {}", def.name);
    let out = match def.kind {
        PropKind::Float => {
            let mut n = v.as_f64().ok_or_else(bad)?;
            if !n.is_finite() {
                return Err(bad());
            }
            if let Some(min) = def.min {
                n = n.max(min);
            }
            if let Some(max) = def.max {
                n = n.min(max);
            }
            json!(n)
        }
        PropKind::Int => json!(v.as_f64().ok_or_else(bad)?.round() as i64),
        PropKind::Bool => json!(v.as_bool().ok_or_else(bad)?),
        PropKind::Str => json!(v.as_str().ok_or_else(bad)?),
        PropKind::Vec2 => {
            let a = v.as_array().filter(|a| a.len() == 2).ok_or_else(bad)?;
            let x = a[0].as_f64().filter(|x| x.is_finite()).ok_or_else(bad)?;
            let y = a[1].as_f64().filter(|y| y.is_finite()).ok_or_else(bad)?;
            json!([x, y])
        }
        PropKind::Color => json!(normalize_hex(v.as_str().ok_or_else(bad)?).ok_or_else(bad)?),
        PropKind::Enum => {
            let s = v.as_str().ok_or_else(bad)?;
            if !def.options.contains(&s) {
                return Err(bad());
            }
            json!(s)
        }
        PropKind::Uid => {
            let s = v.as_str().ok_or_else(bad)?;
            if !s.is_empty() {
                Uid::from_str(s)?;
            }
            json!(s)
        }
        PropKind::Mask => {
            // 32 bits: aceita o inteiro com sinal (-1 = todas) e o sem sinal.
            let n = v.as_f64().ok_or_else(bad)?.round() as i64;
            if !(i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&n) {
                return Err(bad());
            }
            json!(n)
        }
        PropKind::Points => {
            // Cada ponto é um par [x, y] de números finitos.
            let mut pts = Vec::new();
            for p in v.as_array().ok_or_else(bad)? {
                let xy = p.as_array().filter(|a| a.len() == 2).ok_or_else(bad)?;
                let x = xy[0].as_f64().filter(|x| x.is_finite()).ok_or_else(bad)?;
                let y = xy[1].as_f64().filter(|y| y.is_finite()).ok_or_else(bad)?;
                pts.push(json!([x, y]));
            }
            json!(pts)
        }
    };
    Ok(out)
}

/// "#RRGGBB" ou "#RRGGBBAA", em minúsculas. Aceita também "#RGB".
pub fn normalize_hex(s: &str) -> Option<String> {
    let body = s.strip_prefix('#')?;
    let expanded: String = match body.len() {
        3 => body.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => body.to_string(),
        _ => return None,
    };
    if !expanded.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", expanded.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cena() -> Scene {
        Scene::new_default("Principal", 42)
    }

    fn filhos_do_raiz(s: &Scene) -> Vec<String> {
        s.roots()
            .iter()
            .flat_map(|r| s.node(*r).unwrap().children.clone())
            .map(|u| s.node(u).unwrap().name.clone())
            .collect()
    }

    #[test]
    fn cena_padrao_tem_raiz_e_camera() {
        let s = cena();
        assert_eq!(s.roots().len(), 1);
        assert_eq!(s.len(), 2);
        assert_eq!(filhos_do_raiz(&s), ["Câmera principal"]);
    }

    #[test]
    fn nomes_irmaos_sao_unicos() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        let b = s.add_node(Some(root), "Sprite2D", None).unwrap();
        assert_eq!(s.node(a).unwrap().name, "Sprite2D");
        assert_eq!(s.node(b).unwrap().name, "Sprite2D 2");
        // "Sprite2D 2" já existe entre os irmãos, então o novo nome ganha outro número.
        s.rename_node(a, "Sprite2D 2").unwrap();
        assert_eq!(s.node(a).unwrap().name, "Sprite2D 2 2");
    }

    #[test]
    fn desfazer_e_refazer_restauram_a_arvore() {
        let mut s = cena();
        let root = s.roots()[0];
        let before = s.pre_order();
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.set_prop(a, "rotation", json!(45.0), None).unwrap();
        assert_eq!(s.undo().as_deref(), Some("Editar Rotação"));
        assert_eq!(s.undo().as_deref(), Some("Criar Sprite2D"));
        assert_eq!(s.pre_order(), before);
        assert!(s.redo().is_some());
        assert!(s.redo().is_some());
        assert_eq!(s.node(a).unwrap().props.get("rotation"), Some(&json!(45.0)));
    }

    #[test]
    fn valores_iguais_ao_padrao_nao_sao_guardados() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.set_prop(a, "rotation", json!(10.0), None).unwrap();
        s.set_prop(a, "rotation", json!(0.0), None).unwrap();
        assert!(s.node(a).unwrap().props.is_empty());
    }

    #[test]
    fn agrupamento_junta_alteracoes_seguidas() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        for x in 1..=5 {
            s.set_prop(a, "position", json!([x as f64, 0.0]), Some("drag:a")).unwrap();
        }
        s.end_group();
        // As cinco alterações viraram um único passo; o "Criar" continua separado.
        assert_eq!(s.undo_label(), Some("Editar Posição"));
        assert_eq!(s.undo().as_deref(), Some("Editar Posição"));
        assert!(s.node(a).unwrap().props.get("position").is_none());
        assert_eq!(s.undo().as_deref(), Some("Criar Sprite2D"));
        assert!(s.node(a).is_none());
    }

    #[test]
    fn nao_permite_ciclos() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Node2D", None).unwrap();
        let b = s.add_node(Some(a), "Node2D", None).unwrap();
        assert!(s.move_node(a, Some(b), 0).is_err());
        assert!(s.move_node(a, Some(a), 0).is_err());
    }

    #[test]
    fn mover_entre_irmaos_mantem_ordem() {
        let mut s = cena();
        let root = s.roots()[0];
        let cam = s.node(root).unwrap().children[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        let b = s.add_node(Some(root), "Sprite2D", None).unwrap();
        let c = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.move_node(c, Some(root), 0).unwrap();
        assert_eq!(s.node(root).unwrap().children, vec![c, cam, a, b]);
        assert_eq!(s.node(b).unwrap().parent, Some(root));
    }

    #[test]
    fn remover_apaga_a_subarvore() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Node2D", None).unwrap();
        let b = s.add_node(Some(a), "Sprite2D", None).unwrap();
        s.remove_node(a).unwrap();
        assert!(s.node(a).is_none());
        assert!(s.node(b).is_none());
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn remocao_de_reproducao_nao_empilha_desfazer() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Node2D", None).unwrap();
        let b = s.add_node(Some(a), "Sprite2D", None).unwrap();
        let antes = s.undo.len();
        s.remove_runtime(a).unwrap();
        assert!(s.node(a).is_none() && s.node(b).is_none());
        assert!(!s.node(root).unwrap().children.contains(&a));
        assert_eq!(s.undo.len(), antes, "a remoção de reprodução não entra no histórico");
        assert!(s.remove_runtime(a).is_err(), "nó já removido não existe mais");
    }

    #[test]
    fn duplicar_cria_copia_deslocada_com_uids_novos() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.set_prop(a, "position", json!([100.0, 50.0]), None).unwrap();
        let copia = s.duplicate_node(a).unwrap();
        assert_ne!(copia, a);
        let n = s.node(copia).unwrap();
        assert_eq!(n.name, "Sprite2D 2");
        assert_eq!(value_of(n, "position"), json!([132.0, 82.0]));
        assert_eq!(s.index_in_parent(copia), Some(s.index_in_parent(a).unwrap() + 1));
    }

    #[test]
    fn copiar_e_colar_leva_os_filhos() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Node2D", None).unwrap();
        s.add_node(Some(a), "Sprite2D", None).unwrap();
        s.copy_node(a).unwrap();
        let colado = s.paste_node(None).unwrap().unwrap();
        assert_eq!(s.subtree(colado).len(), 2);
        assert_eq!(s.node(colado).unwrap().parent, None);
        assert_eq!(s.roots().len(), 2);
    }

    #[test]
    fn trocar_tipo_descarta_propriedades_que_nao_existem_mais() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.set_prop(a, "flip_h", json!(true), None).unwrap();
        s.change_kind(a, "Node2D").unwrap();
        assert!(!s.node(a).unwrap().props.contains_key("flip_h"));
    }

    #[test]
    fn valida_valores() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Camera2D", None).unwrap();
        s.set_prop(a, "zoom", json!(99.0), None).unwrap();
        assert_eq!(value_of(s.node(a).unwrap(), "zoom"), json!(16.0));
        assert!(s.set_prop(a, "zoom", json!("x"), None).is_err());
        assert!(s.set_prop(a, "nao_existe", json!(1), None).is_err());
        assert!(s.set_prop(a, "modulate", json!("azul"), None).is_err());
        let sp = s.add_node(Some(root), "Sprite2D", None).unwrap();
        assert!(s.set_prop(sp, "shape", json!("triangulo"), None).is_err());
    }

    #[test]
    fn mascara_aceita_32_bits_com_ou_sem_sinal() {
        let mut s = cena();
        let root = s.roots()[0];
        let body = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        s.set_prop(body, "collision_mask", json!(-1), None).unwrap();
        s.set_prop(body, "collision_mask", json!(4294967295u64), None).unwrap();
        s.set_prop(body, "collision_mask", json!(0b1010), None).unwrap();
        assert!(s.set_prop(body, "collision_mask", json!(4294967296u64), None).is_err());
        assert!(s.set_prop(body, "collision_mask", json!(-2147483649i64), None).is_err());
    }

    #[test]
    fn tag_e_layer_aceitam_nomes_do_projeto() {
        // Os nomes válidos vêm das Configurações do Projeto, então o nó aceita qualquer texto.
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Node", None).unwrap();
        s.set_prop(a, "tag", json!("boss"), None).unwrap();
        s.set_prop(a, "layer", json!("plataforma"), None).unwrap();
        assert_eq!(value_of(s.node(a).unwrap(), "tag"), json!("boss"));
    }

    #[test]
    fn pontos_de_poligono_sao_validados() {
        let mut s = cena();
        let root = s.roots()[0];
        let p = s.add_node(Some(root), "CollisionPolygon2D", None).unwrap();
        s.set_prop(p, "points", json!([[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]]), None).unwrap();
        assert_eq!(value_of(s.node(p).unwrap(), "points"), json!([[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]]));
        assert!(s.set_prop(p, "points", json!([[0.0]]), None).is_err());
        assert!(s.set_prop(p, "points", json!([[0.0, "x"]]), None).is_err());
        assert!(s.set_prop(p, "points", json!("não é lista"), None).is_err());
    }

    #[test]
    fn camada_aceita_nome_livre() {
        // A validação do nome contra as configurações do projeto fica no editor; aqui o valor é texto.
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "RigidBody2D", None).unwrap();
        s.set_prop(a, "layer", json!("moeda"), None).unwrap();
        assert_eq!(value_of(s.node(a).unwrap(), "layer"), json!("moeda"));
        assert!(s.set_prop(a, "layer", json!(3), None).is_err());
    }

    #[test]
    fn renomear_material_troca_so_os_nos_com_o_nome_antigo() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        let b = s.add_node(Some(root), "RigidBody2D", None).unwrap();
        let c = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        let sem = s.add_node(Some(root), "Node2D", None).unwrap();
        s.set_prop(a, "physics_material", json!("Gelo"), None).unwrap();
        s.set_prop(b, "physics_material", json!("Gelo"), None).unwrap();
        s.set_prop(c, "physics_material", json!("Borracha"), None).unwrap();
        let antes = s.undo.len();

        assert_eq!(s.rename_prop_value("physics_material", "Gelo", "Gelo2").unwrap(), 2);
        assert_eq!(value_of(s.node(a).unwrap(), "physics_material"), json!("Gelo2"));
        assert_eq!(value_of(s.node(b).unwrap(), "physics_material"), json!("Gelo2"));
        assert_eq!(value_of(s.node(c).unwrap(), "physics_material"), json!("Borracha"));
        assert!(!s.node(sem).unwrap().props.contains_key("physics_material"));
        // Both nodes change in a single undo step.
        assert_eq!(s.undo.len(), antes + 1);
        assert_eq!(s.undo_label(), Some("Renomear Material de física"));
        s.undo();
        assert_eq!(value_of(s.node(a).unwrap(), "physics_material"), json!("Gelo"));
        assert_eq!(value_of(s.node(b).unwrap(), "physics_material"), json!("Gelo"));
    }

    #[test]
    fn renomear_material_sem_nos_afetados_nao_entra_no_historico() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        s.set_prop(a, "physics_material", json!("Gelo"), None).unwrap();
        let antes = s.undo.len();
        assert_eq!(s.rename_prop_value("physics_material", "Madeira", "Pedra").unwrap(), 0);
        assert_eq!(s.rename_prop_value("physics_material", "Gelo", "Gelo").unwrap(), 0);
        assert_eq!(s.undo.len(), antes);
        assert_eq!(value_of(s.node(a).unwrap(), "physics_material"), json!("Gelo"));
    }

    #[test]
    fn renomear_material_vazio_nao_mexe_em_corpos_sem_material() {
        // Bodies without a material hold the empty default, which is not a name and must never be replaced.
        let mut s = cena();
        let root = s.roots()[0];
        let sem = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        let com = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        s.set_prop(com, "physics_material", json!("Gelo"), None).unwrap();
        let antes = s.undo.len();
        assert_eq!(s.rename_prop_value("physics_material", "", "Gelo2").unwrap(), 0);
        assert_eq!(s.rename_prop_value("physics_material", "  ", "Gelo2").unwrap(), 0);
        assert!(!s.node(sem).unwrap().props.contains_key("physics_material"));
        assert_eq!(value_of(s.node(com).unwrap(), "physics_material"), json!("Gelo"));
        assert_eq!(s.undo.len(), antes);
    }

    #[test]
    fn renomear_material_recusa_nome_vazio_sem_mexer_na_cena() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        s.set_prop(a, "physics_material", json!("Gelo"), None).unwrap();
        assert!(s.rename_prop_value("physics_material", "Gelo", "  ").is_err());
        assert_eq!(value_of(s.node(a).unwrap(), "physics_material"), json!("Gelo"));
    }

    #[test]
    fn hex_normalizado() {
        assert_eq!(normalize_hex("#FFF"), Some("#ffffff".into()));
        assert_eq!(normalize_hex("#4F8FD6"), Some("#4f8fd6".into()));
        assert_eq!(normalize_hex("4f8fd6"), None);
        assert_eq!(normalize_hex("#zzzzzz"), None);
    }

    #[test]
    fn ramo_vira_cena_com_os_mesmos_uids() {
        let mut s = cena();
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Node2D", None).unwrap();
        let b = s.add_node(Some(a), "Sprite2D", None).unwrap();
        let ramo = s.branch_as_scene(a, 9).unwrap();
        assert_eq!(ramo.name, s.node(a).unwrap().name);
        assert_eq!(ramo.roots(), &[a]);
        assert_eq!(ramo.len(), 2);
        assert_eq!(ramo.node(b).unwrap().parent, Some(a));
        assert_eq!(ramo.node(a).unwrap().parent, None);
    }

    /// Scene with a Timer (emitter) and a Sprite2D (target) at the root. Returns the UIDs of both.
    fn scene_with_timer() -> (Scene, Uid, Uid) {
        let mut s = cena();
        let root = s.roots()[0];
        let t = s.add_node(Some(root), "Timer", None).unwrap();
        let sp = s.add_node(Some(root), "Sprite2D", None).unwrap();
        (s, t, sp)
    }

    #[test]
    fn connect_can_be_undone_and_redone() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        assert_eq!(s.node(t).unwrap().connections.len(), 1);
        assert_eq!(s.undo_label(), Some("Conectar sinal"));
        assert_eq!(s.undo().as_deref(), Some("Conectar sinal"));
        assert!(s.node(t).unwrap().connections.is_empty());
        assert!(s.redo().is_some());
        assert_eq!(s.node(t).unwrap().connections[0].method, "on_timeout");
        s.disconnect(t, "timeout", sp, "on_timeout").unwrap();
        assert!(s.node(t).unwrap().connections.is_empty());
        assert_eq!(s.undo().as_deref(), Some("Desconectar sinal"));
        assert_eq!(s.node(t).unwrap().connections.len(), 1);
    }

    #[test]
    fn invalid_connection_is_not_recorded_in_history() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        let before = s.undo.len();
        // Sprite2D does not emit timeout; the method is empty, has a space, or starts with a digit; the target is missing.
        assert!(s.connect(sp, "timeout", t, "on_timeout").is_err());
        assert!(s.connect(t, "timeout", sp, "").is_err());
        assert!(s.connect(t, "timeout", sp, "1a").is_err());
        assert!(s.connect(t, "timeout", sp, "on timeout").is_err());
        assert!(s.connect(t, "timeout", Uid(0xdead), "on_timeout").is_err());
        // The same connection with spaces at the ends is a duplicate once trimmed.
        assert!(s.connect(t, "timeout", sp, "  on_timeout ").is_err());
        assert_eq!(s.undo.len(), before, "a refused connection records no undo");
        assert_eq!(s.node(t).unwrap().connections.len(), 1);
    }

    #[test]
    fn method_can_point_at_the_emitter_itself() {
        let (mut s, t, _) = scene_with_timer();
        s.connect(t, "timeout", t, "_auto_1").unwrap();
        assert_eq!(s.node(t).unwrap().connections[0].target, t);
    }

    #[test]
    fn removing_target_clears_its_connections_and_undo_restores_them() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        s.remove_node(sp).unwrap();
        assert!(s.node(t).unwrap().connections.is_empty());
        assert_eq!(s.undo().as_deref(), Some("Excluir Sprite2D"));
        assert_eq!(s.node(t).unwrap().connections.len(), 1);
    }

    #[test]
    fn removing_emitter_leaves_no_dangling_connection() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        s.remove_node(t).unwrap();
        assert!(s.node(t).is_none());
        assert!(s.node(sp).is_some());
    }

    #[test]
    fn runtime_removal_clears_connections_to_the_removed_node() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        let before = s.undo.len();
        s.remove_runtime(sp).unwrap();
        assert!(s.node(t).unwrap().connections.is_empty());
        assert_eq!(s.undo.len(), before);
    }

    #[test]
    fn duplicate_remaps_inside_targets_and_keeps_outside_ones() {
        let mut s = cena();
        let root = s.roots()[0];
        let group = s.add_node(Some(root), "Node2D", None).unwrap();
        let t = s.add_node(Some(group), "Timer", None).unwrap();
        let inside = s.add_node(Some(group), "Sprite2D", None).unwrap();
        let outside = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.connect(t, "timeout", inside, "on_inside").unwrap();
        s.connect(t, "timeout", outside, "on_outside").unwrap();
        let copy = s.duplicate_node(group).unwrap();
        let children = s.node(copy).unwrap().children.clone();
        let (t2, inside2) = (children[0], children[1]);
        let links = &s.node(t2).unwrap().connections;
        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|c| c.target == inside2 && c.method == "on_inside"));
        assert!(links.iter().any(|c| c.target == outside && c.method == "on_outside"));
        // The original still points at its own target.
        assert!(s.node(t).unwrap().connections.iter().any(|c| c.target == inside));
    }

    #[test]
    fn paste_remaps_inside_targets_of_the_copy() {
        let mut s = cena();
        let root = s.roots()[0];
        let group = s.add_node(Some(root), "Node2D", None).unwrap();
        let t = s.add_node(Some(group), "Timer", None).unwrap();
        let inside = s.add_node(Some(group), "Sprite2D", None).unwrap();
        s.connect(t, "timeout", inside, "on_inside").unwrap();
        s.copy_node(group).unwrap();
        let pasted = s.paste_node(None).unwrap().unwrap();
        let children = s.node(pasted).unwrap().children.clone();
        let link = &s.node(children[0]).unwrap().connections[0];
        assert_eq!(link.target, children[1]);
        assert_ne!(children[1], inside);
    }

    #[test]
    fn change_kind_drops_signals_the_new_kind_does_not_emit() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        s.change_kind(t, "Node2D").unwrap();
        assert!(s.node(t).unwrap().connections.is_empty());
    }

    #[test]
    fn branch_saved_as_scene_drops_outside_targets() {
        let mut s = cena();
        let root = s.roots()[0];
        let group = s.add_node(Some(root), "Node2D", None).unwrap();
        let t = s.add_node(Some(group), "Timer", None).unwrap();
        let inside = s.add_node(Some(group), "Sprite2D", None).unwrap();
        let outside = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.connect(t, "timeout", inside, "on_inside").unwrap();
        s.connect(t, "timeout", outside, "on_outside").unwrap();
        let branch = s.branch_as_scene(group, 9).unwrap();
        let links = &branch.node(t).unwrap().connections;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, inside);
    }

    #[test]
    fn removing_group_containing_target_clears_the_connection() {
        let mut s = cena();
        let root = s.roots()[0];
        let t = s.add_node(Some(root), "Timer", None).unwrap();
        let group = s.add_node(Some(root), "Node2D", None).unwrap();
        let inside = s.add_node(Some(group), "Sprite2D", None).unwrap();
        s.connect(t, "timeout", inside, "on_timeout").unwrap();
        s.remove_node(group).unwrap();
        assert!(s.node(t).unwrap().connections.is_empty());
        assert_eq!(s.undo().as_deref(), Some("Excluir Node2D"));
        assert_eq!(s.node(t).unwrap().connections.len(), 1);
    }

    #[test]
    fn paste_keeps_target_outside_the_copy() {
        let mut s = cena();
        let root = s.roots()[0];
        let t = s.add_node(Some(root), "Timer", None).unwrap();
        let outside = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.connect(t, "timeout", outside, "on_outside").unwrap();
        s.copy_node(t).unwrap();
        let pasted = s.paste_node(None).unwrap().unwrap();
        let link = &s.node(pasted).unwrap().connections[0];
        assert_eq!(link.target, outside);
        assert_eq!(link.method, "on_outside");
    }

    #[test]
    fn disconnecting_missing_connection_is_refused_without_history() {
        let (mut s, t, sp) = scene_with_timer();
        s.connect(t, "timeout", sp, "on_timeout").unwrap();
        let before = s.undo.len();
        assert!(s.disconnect(t, "timeout", sp, "on_other").is_err());
        assert!(s.disconnect(sp, "timeout", t, "on_timeout").is_err());
        assert_eq!(s.undo.len(), before, "a refused disconnect records no undo");
        assert_eq!(s.node(t).unwrap().connections.len(), 1);
    }

    #[test]
    fn uid_ida_e_volta() {
        let u = Uid(0x00ab_cdef_0123_4567);
        assert_eq!(u.to_string(), "00abcdef01234567");
        assert_eq!("00abcdef01234567".parse::<Uid>().unwrap(), u);
    }
}
