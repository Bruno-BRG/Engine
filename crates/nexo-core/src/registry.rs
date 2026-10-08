//! Registro de tipos de nó: herança, propriedades e metadados (SCN-002).
//! O Inspector, o serializador e, na Fase 2, a programação visual leem daqui.

use serde::Serialize;
use serde_json::{Value, json};
use std::sync::OnceLock;

/// Como o valor de uma propriedade é representado e editado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PropKind {
    /// Número real (f64).
    Float,
    /// Número inteiro (i64).
    Int,
    Bool,
    Str,
    /// Par [x, y].
    Vec2,
    /// Cor em hexadecimal, "#rrggbb".
    Color,
    /// Uma das opções listadas em `options`.
    Enum,
    /// Referência a um recurso pelo UID (hexadecimal de 16 dígitos). Vazio = nenhum.
    Uid,
    /// Máscara de bits das camadas de colisão (INS-003): um inteiro, um bit por camada.
    Mask,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropDef {
    pub name: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    pub kind: PropKind,
    pub default: Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub unit: Option<&'static str>,
    pub options: &'static [&'static str],
    /// Para `PropKind::Uid`: tipo de asset aceito, por exemplo "texture".
    pub asset: Option<&'static str>,
}

impl PropDef {
    fn new(name: &'static str, label: &'static str, group: &'static str, kind: PropKind, default: Value) -> Self {
        Self {
            name,
            label,
            group,
            kind,
            default,
            min: None,
            max: None,
            step: None,
            unit: None,
            options: &[],
            asset: None,
        }
    }
    fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
    fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }
    fn unit(mut self, unit: &'static str) -> Self {
        self.unit = Some(unit);
        self
    }
    fn options(mut self, options: &'static [&'static str]) -> Self {
        self.options = options;
        self
    }
    fn asset(mut self, asset: &'static str) -> Self {
        self.asset = Some(asset);
        self
    }
}

/// Um tipo de nó. `props` lista só as propriedades declaradas por este tipo;
/// as herdadas vêm da cadeia `base`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeType {
    pub name: &'static str,
    pub base: Option<&'static str>,
    pub category: &'static str,
    pub description: &'static str,
    pub props: Vec<PropDef>,
}

const SHAPES: &[&str] = &["rect", "circle", "diamond"];
const COLLIDER_SHAPES: &[&str] = &["rect", "circle"];
const BEHAVIOR_KINDS: &[&str] = &["spin", "float", "pulse"];
/// Camadas padrão do registro (usadas pela física até ela ler as Configurações do Projeto, PHY-021).
/// Tag e Layer são texto: os nomes válidos vêm de `ProjectSettings` (SCN-033, INS-001), não daqui.
pub const LAYERS: &[&str] = &["default", "ground", "player", "enemy"];

/// Máscara de colisão: bits das camadas que o corpo detecta. -1 significa todas.
fn collision_mask() -> PropDef {
    PropDef::new("collision_mask", "Máscara de colisão", "Colisão", PropKind::Mask, json!(-1))
}

fn build() -> Vec<NodeType> {
    vec![
        NodeType {
            name: "Node",
            base: None,
            category: "Básico",
            description: "Base de todos os nós da cena.",
            props: vec![
                PropDef::new("tag", "Tag", "Identificação", PropKind::Str, json!("untagged")),
                PropDef::new("layer", "Layer", "Identificação", PropKind::Str, json!("default")),
                PropDef::new("locked", "Estático", "Editor", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "Node2D",
            base: Some("Node"),
            category: "2D",
            description: "Nó com posição, rotação e escala no mundo 2D.",
            props: vec![
                PropDef::new("position", "Posição", "Transformação", PropKind::Vec2, json!([0.0, 0.0]))
                    .step(1.0),
                PropDef::new("rotation", "Rotação", "Transformação", PropKind::Float, json!(0.0))
                    .step(1.0)
                    .unit("°"),
                PropDef::new("scale", "Escala", "Transformação", PropKind::Vec2, json!([1.0, 1.0]))
                    .step(0.01),
                PropDef::new("z_index", "Ordem Z", "Exibição", PropKind::Int, json!(0)),
                PropDef::new("visible", "Visível", "Exibição", PropKind::Bool, json!(true)),
                PropDef::new("modulate", "Modulação", "Exibição", PropKind::Color, json!("#ffffff")),
            ],
        },
        NodeType {
            name: "Sprite2D",
            base: Some("Node2D"),
            category: "2D",
            description: "Desenha uma textura ou, sem textura, uma forma de teste.",
            props: vec![
                PropDef::new("texture", "Textura", "Sprite", PropKind::Uid, json!("")).asset("texture"),
                PropDef::new("shape", "Forma", "Sprite", PropKind::Enum, json!("rect"))
                    .options(SHAPES),
                PropDef::new("size", "Tamanho", "Sprite", PropKind::Vec2, json!([96.0, 96.0]))
                    .step(1.0),
                PropDef::new("centered", "Centralizado", "Sprite", PropKind::Bool, json!(true)),
                PropDef::new("offset", "Deslocamento", "Sprite", PropKind::Vec2, json!([0.0, 0.0])).step(1.0),
                PropDef::new("flip_h", "Espelhar horizontal", "Sprite", PropKind::Bool, json!(false)),
                PropDef::new("flip_v", "Espelhar vertical", "Sprite", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "Camera2D",
            base: Some("Node2D"),
            category: "2D",
            description: "Define a área do mundo que aparece na tela do jogo.",
            props: vec![
                PropDef::new("zoom", "Zoom", "Câmera", PropKind::Float, json!(1.0))
                    .range(0.05, 16.0)
                    .step(0.05),
                PropDef::new("current", "Câmera atual", "Câmera", PropKind::Bool, json!(true)),
                PropDef::new("smoothing", "Suavização", "Câmera", PropKind::Float, json!(0.0))
                    .range(0.0, 20.0)
                    .step(0.1),
            ],
        },
        NodeType {
            name: "StaticBody2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Corpo imóvel: chão, paredes e plataformas.",
            props: vec![collision_mask()],
        },
        NodeType {
            name: "RigidBody2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Corpo dinâmico: cai, empurra e quica sob a gravidade.",
            props: vec![
                collision_mask(),
                PropDef::new("gravity_scale", "Escala de gravidade", "Física", PropKind::Float, json!(1.0))
                    .range(-10.0, 10.0)
                    .step(0.1),
                PropDef::new("linear_damping", "Amortecimento linear", "Física", PropKind::Float, json!(0.0))
                    .range(0.0, 10.0)
                    .step(0.1),
                PropDef::new("lock_rotation", "Travar rotação", "Física", PropKind::Bool, json!(false)),
                PropDef::new("continuous", "Detecção contínua", "Física", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "CharacterBody2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Personagem controlado por código: anda e para em paredes e chão. A gravidade não é automática.",
            props: vec![
                collision_mask(),
                PropDef::new("velocity", "Velocidade", "Movimento", PropKind::Vec2, json!([0.0, 0.0])).step(1.0),
            ],
        },
        NodeType {
            name: "Area2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Detecta os corpos e as áreas que entram nela. Não empurra nada.",
            props: vec![
                collision_mask(),
                PropDef::new("monitoring", "Monitorar", "Área", PropKind::Bool, json!(true)),
            ],
        },
        NodeType {
            name: "CollisionShape2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Forma de colisão do corpo pai. A posição é o deslocamento em relação ao corpo.",
            props: vec![
                PropDef::new("shape", "Forma", "Forma", PropKind::Enum, json!("rect")).options(COLLIDER_SHAPES),
                PropDef::new("size", "Tamanho", "Forma", PropKind::Vec2, json!([32.0, 32.0])).step(1.0),
                PropDef::new("disabled", "Desativada", "Forma", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "Behavior",
            base: Some("Node"),
            category: "Comportamento (protótipo)",
            description: "Anima o nó pai. Substituído pela programação visual na Fase 2.",
            props: vec![
                PropDef::new("kind", "Tipo", "Comportamento", PropKind::Enum, json!("spin"))
                    .options(BEHAVIOR_KINDS),
                PropDef::new("speed", "Velocidade", "Comportamento", PropKind::Float, json!(90.0))
                    .range(0.0, 1000.0)
                    .step(0.1),
            ],
        },
    ]
}

/// Todos os tipos registrados, na ordem em que aparecem no menu de criação.
pub fn node_types() -> &'static [NodeType] {
    static TYPES: OnceLock<Vec<NodeType>> = OnceLock::new();
    TYPES.get_or_init(build)
}

pub fn find(name: &str) -> Option<&'static NodeType> {
    node_types().iter().find(|t| t.name == name)
}

/// O tipo e todos os seus ancestrais, do mais específico ao `Node`.
pub fn chain(name: &str) -> Vec<&'static NodeType> {
    let mut out = Vec::new();
    let mut cur = find(name);
    while let Some(t) = cur {
        out.push(t);
        cur = t.base.and_then(find);
    }
    out
}

/// Definição de uma propriedade, procurando também nos tipos ancestrais.
pub fn prop_def(kind: &str, prop: &str) -> Option<&'static PropDef> {
    chain(kind)
        .into_iter()
        .find_map(|t| t.props.iter().find(|p| p.name == prop))
}

/// Valor padrão de uma propriedade do tipo.
pub fn default_value(kind: &str, prop: &str) -> Option<Value> {
    prop_def(kind, prop).map(|p| p.default.clone())
}

/// True quando o tipo é ou herda de `Node2D`.
pub fn is_2d(kind: &str) -> bool {
    chain(kind).iter().any(|t| t.name == "Node2D")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadeia_de_heranca() {
        let names: Vec<&str> = chain("Sprite2D").iter().map(|t| t.name).collect();
        assert_eq!(names, ["Sprite2D", "Node2D", "Node"]);
    }

    #[test]
    fn propriedade_herdada_e_encontrada() {
        assert!(prop_def("Sprite2D", "position").is_some());
        assert!(prop_def("Sprite2D", "locked").is_some());
        assert!(prop_def("Sprite2D", "nao_existe").is_none());
        assert!(is_2d("Camera2D"));
        assert!(!is_2d("Behavior"));
    }

    #[test]
    fn nomes_de_tipo_unicos() {
        let mut names: Vec<&str> = node_types().iter().map(|t| t.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), node_types().len());
    }
}
