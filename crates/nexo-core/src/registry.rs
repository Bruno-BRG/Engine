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
    /// Lista de pontos [[x, y], ...] (polígonos de colisão, PHY-011).
    Points,
}

/// Lista das Configurações do Projeto de onde vêm os nomes válidos de um texto (SCN-033, AUD-001, PHY-023).
/// O registro só diz qual lista; a interface lê os nomes de `projectSettings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NameList {
    Tags,
    Layers,
    Buses,
    PhysicsMaterials,
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
    /// Para `PropKind::Float` com faixa: o Inspector mostra também um controle deslizante (INS-003).
    pub slider: bool,
    /// Para `PropKind::Str` que nomeia algo do projeto: de qual lista vêm as opções.
    pub names: Option<NameList>,
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
            slider: false,
            names: None,
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
    fn slider(mut self) -> Self {
        self.slider = true;
        self
    }
    fn names(mut self, list: NameList) -> Self {
        self.names = Some(list);
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
const COLLIDER_SHAPES: &[&str] = &["rect", "circle", "capsule_y", "segment"];
const FREEZE_MODES: &[&str] = &["none", "static", "kinematic"];
const OVERRIDE_MODES: &[&str] = &["disabled", "combine", "replace"];
const BEHAVIOR_KINDS: &[&str] = &["spin", "float", "pulse"];
const JOY_SIDES: &[&str] = &["left", "right"];

/// Máscara de colisão: bits das camadas que o corpo detecta. -1 significa todas.
fn collision_mask() -> PropDef {
    PropDef::new("collision_mask", "Máscara de colisão", "Colisão", PropKind::Mask, json!(-1))
}

/// Propriedades de reprodução do áudio (AUD-001), iguais em AudioStreamPlayer e AudioStreamPlayer2D.
/// O barramento é texto: os nomes vêm das configurações do projeto, que o registro não conhece.
fn audio_player_props() -> Vec<PropDef> {
    vec![
        PropDef::new("stream", "Áudio", "Reprodução", PropKind::Uid, json!("")).asset("audio"),
        PropDef::new("volume_db", "Volume", "Reprodução", PropKind::Float, json!(0.0))
            .range(-80.0, 24.0)
            .step(0.5)
            .unit("dB")
            .slider(),
        PropDef::new("pitch_scale", "Tom", "Reprodução", PropKind::Float, json!(1.0))
            .range(0.01, 4.0)
            .step(0.01)
            .slider(),
        PropDef::new("autoplay", "Tocar ao iniciar", "Reprodução", PropKind::Bool, json!(false)),
        PropDef::new("loop", "Repetir", "Reprodução", PropKind::Bool, json!(false)),
        PropDef::new("bus", "Barramento", "Reprodução", PropKind::Str, json!("Master")).names(NameList::Buses),
    ]
}

/// Camadas de todo corpo e área: máscara, camadas incluídas e excluídas (PHY-022). As três são bits
/// das camadas nomeadas: a interface as mostra pelo nome.
fn layer_props() -> Vec<PropDef> {
    vec![
        collision_mask(),
        PropDef::new("include_layers", "Incluir camadas", "Colisão", PropKind::Mask, json!(0)),
        PropDef::new("exclude_layers", "Excluir camadas", "Colisão", PropKind::Mask, json!(0)),
    ]
}

/// As propriedades de camada mais as de um tipo específico.
fn com_camadas(mut extra: Vec<PropDef>) -> Vec<PropDef> {
    let mut props = layer_props();
    props.append(&mut extra);
    props
}

/// Material de física pelo nome (PHY-023): vazio = sem material.
fn material() -> PropDef {
    PropDef::new("physics_material", "Material de física", "Colisão", PropKind::Str, json!(""))
        .names(NameList::PhysicsMaterials)
}

fn build() -> Vec<NodeType> {
    vec![
        NodeType {
            name: "Node",
            base: None,
            category: "Básico",
            description: "Base de todos os nós da cena.",
            props: vec![
                PropDef::new("tag", "Tag", "Identificação", PropKind::Str, json!("untagged")).names(NameList::Tags),
                PropDef::new("layer", "Layer", "Identificação", PropKind::Str, json!("default"))
                    .names(NameList::Layers),
                PropDef::new("locked", "Estático", "Editor", PropKind::Bool, json!(false)),
                // Script anexado ao nó (VIS-003, INS-005): o nome do arquivo `scripts/<nome>.logic.ron`, sem pasta
                // e sem extensão. Vazio = nenhum. O Inspector mostra um slot próprio, não um campo de texto.
                PropDef::new("script", "Script", "Lógica", PropKind::Str, json!("")),
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
                    .step(0.1)
                    .slider(),
            ],
        },
        NodeType {
            name: "StaticBody2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Corpo imóvel: chão, paredes e plataformas. Com velocidade constante vira esteira.",
            props: com_camadas(vec![
                material(),
                PropDef::new(
                    "constant_linear_velocity",
                    "Velocidade constante",
                    "Física",
                    PropKind::Vec2,
                    json!([0.0, 0.0]),
                )
                .step(1.0),
                // Gira a superfície em torno da origem do corpo (disco de esteira, PHY-002).
                PropDef::new(
                    "constant_angular_velocity",
                    "Velocidade angular constante",
                    "Física",
                    PropKind::Float,
                    json!(0.0),
                )
                .range(-3600.0, 3600.0)
                .step(1.0)
                .unit("°/s"),
            ]),
        },
        NodeType {
            name: "RigidBody2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Corpo dinâmico: cai, empurra e quica sob a gravidade.",
            props: com_camadas(vec![
                material(),
                PropDef::new("gravity_scale", "Escala de gravidade", "Física", PropKind::Float, json!(1.0))
                    .range(-10.0, 10.0)
                    .step(0.1)
                    .slider(),
                PropDef::new("linear_damping", "Amortecimento linear", "Física", PropKind::Float, json!(0.0))
                    .range(0.0, 10.0)
                    .step(0.1)
                    .slider(),
                PropDef::new("angular_damping", "Amortecimento angular", "Física", PropKind::Float, json!(0.0))
                    .range(0.0, 10.0)
                    .step(0.1)
                    .slider(),
                PropDef::new("mass", "Massa", "Física", PropKind::Float, json!(1.0))
                    .range(0.01, 10000.0)
                    .step(0.1),
                PropDef::new("can_sleep", "Pode dormir", "Física", PropKind::Bool, json!(true)),
                PropDef::new("freeze", "Congelar", "Física", PropKind::Enum, json!("none")).options(FREEZE_MODES),
                PropDef::new("lock_rotation", "Travar rotação", "Física", PropKind::Bool, json!(false)),
                PropDef::new("continuous", "Detecção contínua", "Física", PropKind::Bool, json!(false)),
            ]),
        },
        NodeType {
            name: "CharacterBody2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Personagem controlado por código: anda e para em paredes e chão. A gravidade não é automática.",
            props: com_camadas(vec![
                PropDef::new("velocity", "Velocidade", "Movimento", PropKind::Vec2, json!([0.0, 0.0])).step(1.0),
                // Ao descer uma rampa, o personagem cola no chão se ele estiver até essa distância (PHY-004).
                // Zero desliga a colagem.
                PropDef::new("floor_snap_length", "Colagem ao chão", "Movimento", PropKind::Float, json!(6.0))
                    .range(0.0, 200.0)
                    .step(0.5)
                    .unit("px"),
            ]),
        },
        NodeType {
            name: "Area2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Detecta os corpos e as áreas que entram nela. Pode trocar a gravidade e o amortecimento dos corpos.",
            props: com_camadas(vec![
                PropDef::new("monitoring", "Monitorar", "Área", PropKind::Bool, json!(true)),
                PropDef::new("gravity_override", "Gravidade", "Área", PropKind::Enum, json!("disabled"))
                    .options(OVERRIDE_MODES),
                PropDef::new("gravity", "Vetor de gravidade", "Área", PropKind::Vec2, json!([0.0, 0.0])).step(1.0),
                PropDef::new("damp_override", "Amortecimento", "Área", PropKind::Enum, json!("disabled"))
                    .options(OVERRIDE_MODES),
                PropDef::new("linear_damp", "Amortecimento linear", "Área", PropKind::Float, json!(0.0))
                    .range(0.0, 10.0)
                    .step(0.1)
                    .slider(),
            ]),
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
                PropDef::new("one_way", "Colisão de uma face", "Forma", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "CollisionPolygon2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Forma de colisão por pontos. Convexa vira uma forma só; côncava, partes convexas.",
            props: vec![
                PropDef::new("points", "Pontos", "Forma", PropKind::Points, json!([])),
                PropDef::new("disabled", "Desativada", "Forma", PropKind::Bool, json!(false)),
                PropDef::new("one_way", "Colisão de uma face", "Forma", PropKind::Bool, json!(false)),
            ],
        },
        // Consultas e juntas (PHY-030, PHY-031). Bloco próprio no fim da física: as camadas de
        // `collision_mask` são os mesmos bits das camadas dos corpos (ver `LAYERS`).
        NodeType {
            name: "RayCast2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Raio de consulta: diz qual corpo o raio atinge primeiro, do nó até o alvo.",
            props: vec![
                PropDef::new("enabled", "Ativo", "Consulta", PropKind::Bool, json!(true)),
                PropDef::new("target", "Alvo (relativo ao nó)", "Consulta", PropKind::Vec2, json!([0.0, 100.0]))
                    .step(1.0),
                collision_mask(),
                PropDef::new("collide_with_areas", "Atingir áreas", "Consulta", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "ShapeCast2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Forma que varre o caminho do nó até o alvo e diz o primeiro corpo que ela toca.",
            props: vec![
                PropDef::new("enabled", "Ativo", "Consulta", PropKind::Bool, json!(true)),
                PropDef::new("shape", "Forma", "Consulta", PropKind::Enum, json!("rect")).options(COLLIDER_SHAPES),
                PropDef::new("size", "Tamanho", "Consulta", PropKind::Vec2, json!([32.0, 32.0])).step(1.0),
                PropDef::new("target", "Alvo (relativo ao nó)", "Consulta", PropKind::Vec2, json!([0.0, 100.0]))
                    .step(1.0),
                collision_mask(),
                PropDef::new("collide_with_areas", "Atingir áreas", "Consulta", PropKind::Bool, json!(false)),
            ],
        },
        NodeType {
            name: "PinJoint2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Prende o corpo pai ao corpo B por um ponto. A posição do próprio nó não é usada.",
            props: vec![
                PropDef::new("node_b", "Corpo B", "Junta", PropKind::Uid, json!("")),
                PropDef::new("anchor", "Âncora (no corpo pai)", "Junta", PropKind::Vec2, json!([0.0, 0.0]))
                    .step(1.0),
            ],
        },
        NodeType {
            name: "GrooveJoint2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Faz o corpo B deslizar num trilho de dois pontos do corpo pai.",
            props: vec![
                PropDef::new("node_b", "Corpo B", "Junta", PropKind::Uid, json!("")),
                PropDef::new("groove_start", "Início do trilho", "Junta", PropKind::Vec2, json!([0.0, 0.0]))
                    .step(1.0),
                PropDef::new("groove_end", "Fim do trilho", "Junta", PropKind::Vec2, json!([0.0, 100.0]))
                    .step(1.0),
                PropDef::new("anchor_b", "Âncora em B", "Junta", PropKind::Vec2, json!([0.0, 0.0])).step(1.0),
            ],
        },
        NodeType {
            name: "DampedSpringJoint2D",
            base: Some("Node2D"),
            category: "Física",
            description: "Mola entre o corpo pai e o corpo B: puxa até o comprimento de repouso e amortece.",
            props: vec![
                PropDef::new("node_b", "Corpo B", "Junta", PropKind::Uid, json!("")),
                PropDef::new("anchor", "Âncora (no corpo pai)", "Junta", PropKind::Vec2, json!([0.0, 0.0]))
                    .step(1.0),
                PropDef::new("anchor_b", "Âncora em B", "Junta", PropKind::Vec2, json!([0.0, 0.0])).step(1.0),
                PropDef::new("rest_length", "Comprimento de repouso", "Mola", PropKind::Float, json!(100.0))
                    .range(0.0, 10000.0)
                    .step(1.0),
                PropDef::new("stiffness", "Rigidez", "Mola", PropKind::Float, json!(20.0))
                    .range(0.0, 10000.0)
                    .step(0.5),
                PropDef::new("damping", "Amortecimento", "Mola", PropKind::Float, json!(1.0))
                    .range(0.0, 1000.0)
                    .step(0.1),
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
        // ---- Áudio e temporizadores (AUD-001, SCN-014). Bloco próprio: não misturar com outros tipos. ----
        NodeType {
            name: "AudioStreamPlayer",
            base: Some("Node"),
            category: "Áudio",
            description: "Toca um áudio sem posição: o volume é o mesmo em toda a cena.",
            props: audio_player_props(),
        },
        // Base Node2D (e não AudioStreamPlayer), para ter posição: a atenuação 2D mede a distância até a
        // câmera. O registro tem herança simples, então as propriedades de áudio são repetidas pelo helper.
        NodeType {
            name: "AudioStreamPlayer2D",
            base: Some("Node2D"),
            category: "Áudio",
            description: "Toca um áudio com posição: o volume cai com a distância até a câmera, até a distância máxima.",
            props: {
                let mut props = audio_player_props();
                props.push(
                    PropDef::new("max_distance", "Distância máxima", "Atenuação", PropKind::Float, json!(2000.0))
                        .range(1.0, 100000.0)
                        .step(10.0)
                        .unit("px"),
                );
                props.push(
                    PropDef::new("attenuation", "Curva de atenuação", "Atenuação", PropKind::Float, json!(1.0))
                        .range(0.0, 4.0)
                        .step(0.1)
                        .slider(),
                );
                props
            },
        },
        NodeType {
            name: "Timer",
            base: Some("Node"),
            category: "Tempo",
            description: "Conta a espera e emite o sinal timeout. Com disparo único, para no primeiro sinal.",
            props: vec![
                PropDef::new("wait_time", "Espera", "Temporizador", PropKind::Float, json!(1.0))
                    .range(0.01, 3600.0)
                    .step(0.05)
                    .unit("s"),
                PropDef::new("one_shot", "Disparo único", "Temporizador", PropKind::Bool, json!(false)),
                PropDef::new("autostart", "Iniciar ao jogar", "Temporizador", PropKind::Bool, json!(false)),
            ],
        },
        // Joystick de toque (INP-004). Base Node, sem transformação: fica preso à tela, não ao mundo.
        NodeType {
            name: "VirtualJoystick",
            base: Some("Node"),
            category: "Entrada",
            description: "Joystick de toque sobre a tela do jogo. Na reprodução, empurrá-lo aciona as ações de direção. No desktop fica oculto, a não ser que \"Mostrar no desktop\" esteja ligado.",
            props: vec![
                PropDef::new("side", "Lado", "Joystick", PropKind::Enum, json!("left")).options(JOY_SIDES),
                PropDef::new("radius", "Raio", "Joystick", PropKind::Float, json!(64.0))
                    .range(24.0, 200.0)
                    .step(1.0)
                    .unit("px"),
                PropDef::new("show_on_desktop", "Mostrar no desktop", "Joystick", PropKind::Bool, json!(false)),
                PropDef::new("action_left", "Ação esquerda", "Ações", PropKind::Str, json!("ui_left")),
                PropDef::new("action_right", "Ação direita", "Ações", PropKind::Str, json!("ui_right")),
                PropDef::new("action_up", "Ação cima", "Ações", PropKind::Str, json!("ui_up")),
                PropDef::new("action_down", "Ação baixo", "Ações", PropKind::Str, json!("ui_down")),
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
    fn tipos_de_consulta_e_junta_tem_propriedades() {
        assert_eq!(prop_def("RayCast2D", "collide_with_areas").unwrap().default, json!(false));
        assert_eq!(prop_def("ShapeCast2D", "collision_mask").unwrap().default, json!(-1));
        assert_eq!(prop_def("PinJoint2D", "node_b").unwrap().kind, PropKind::Uid);
        assert_eq!(prop_def("GrooveJoint2D", "groove_end").unwrap().default, json!([0.0, 100.0]));
        assert!(prop_def("DampedSpringJoint2D", "rest_length").is_some());
        assert!(is_2d("GrooveJoint2D"));
    }

    #[test]
    fn todo_no_tem_script_anexado_vazio() {
        // O script é propriedade do tipo base: vale para qualquer nó, inclusive os que herdam de Node2D.
        let def = prop_def("Sprite2D", "script").expect("Node.script");
        assert_eq!(def.kind, PropKind::Str);
        assert_eq!(def.default, json!(""));
        assert_eq!(def.group, "Lógica");
        assert!(prop_def("Timer", "script").is_some());
    }

    #[test]
    fn nomes_de_tipo_unicos() {
        let mut names: Vec<&str> = node_types().iter().map(|t| t.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), node_types().len());
    }

    #[test]
    fn players_de_audio_e_timer_registrados() {
        // O player 2D tem posição (é Node2D); o player simples não tem.
        assert!(is_2d("AudioStreamPlayer2D"));
        assert!(!is_2d("AudioStreamPlayer"));
        assert!(prop_def("AudioStreamPlayer2D", "max_distance").is_some());
        assert!(prop_def("AudioStreamPlayer2D", "bus").is_some());
        assert!(prop_def("AudioStreamPlayer", "max_distance").is_none());
        assert_eq!(default_value("AudioStreamPlayer", "bus"), Some(json!("Master")));
        assert_eq!(default_value("AudioStreamPlayer", "pitch_scale"), Some(json!(1.0)));
        assert_eq!(default_value("Timer", "wait_time"), Some(json!(1.0)));
        assert_eq!(default_value("Timer", "one_shot"), Some(json!(false)));
        assert_eq!(prop_def("AudioStreamPlayer", "stream").and_then(|p| p.asset), Some("audio"));
    }

    #[test]
    fn joystick_virtual_fica_na_tela_e_aciona_as_acoes_padrao() {
        // INP-004: sem transformação de mundo, e as ações de direção vêm do padrão de INP-002.
        assert!(!is_2d("VirtualJoystick"));
        assert!(prop_def("VirtualJoystick", "position").is_none());
        assert_eq!(default_value("VirtualJoystick", "action_left"), Some(json!("ui_left")));
        assert_eq!(default_value("VirtualJoystick", "action_down"), Some(json!("ui_down")));
        assert_eq!(default_value("VirtualJoystick", "show_on_desktop"), Some(json!(false)));
        assert_eq!(default_value("VirtualJoystick", "radius"), Some(json!(64.0)));
    }

    #[test]
    fn campos_de_nome_apontam_para_a_lista_do_projeto() {
        assert_eq!(prop_def("StaticBody2D", "tag").unwrap().names, Some(NameList::Tags));
        assert_eq!(prop_def("StaticBody2D", "layer").unwrap().names, Some(NameList::Layers));
        assert_eq!(prop_def("AudioStreamPlayer", "bus").unwrap().names, Some(NameList::Buses));
        assert_eq!(prop_def("RigidBody2D", "physics_material").unwrap().names, Some(NameList::PhysicsMaterials));
        assert_eq!(prop_def("Sprite2D", "texture").unwrap().names, None);
    }

    #[test]
    fn todo_campo_de_nome_e_texto() {
        for t in node_types() {
            for p in t.props.iter().filter(|p| p.names.is_some()) {
                assert_eq!(p.kind, PropKind::Str, "{}.{}", t.name, p.name);
            }
        }
    }

    #[test]
    fn camadas_incluir_e_excluir_sao_mascaras() {
        for kind in ["StaticBody2D", "RigidBody2D", "Area2D"] {
            assert_eq!(prop_def(kind, "include_layers").unwrap().kind, PropKind::Mask);
            assert_eq!(prop_def(kind, "exclude_layers").unwrap().kind, PropKind::Mask);
        }
        assert_eq!(default_value("Area2D", "exclude_layers"), Some(json!(0)));
    }

    #[test]
    fn controle_deslizante_so_em_faixa_limitada() {
        // Sem faixa, o controle deslizante não tem onde começar e terminar.
        for t in node_types() {
            for p in t.props.iter().filter(|p| p.slider) {
                assert!(
                    p.kind == PropKind::Float && p.min.is_some() && p.max.is_some(),
                    "{}.{}",
                    t.name,
                    p.name
                );
            }
        }
        assert!(prop_def("AudioStreamPlayer", "pitch_scale").unwrap().slider);
        assert!(!prop_def("Sprite2D", "size").unwrap().slider);
    }

    #[test]
    fn metadados_chegam_ao_editor_em_json() {
        let bus = serde_json::to_value(prop_def("AudioStreamPlayer", "bus").unwrap()).unwrap();
        assert_eq!(bus["names"], json!("buses"));
        assert_eq!(bus["slider"], json!(false));
        let material = serde_json::to_value(prop_def("RigidBody2D", "physics_material").unwrap()).unwrap();
        assert_eq!(material["names"], json!("physics_materials"));
        let pitch = serde_json::to_value(prop_def("AudioStreamPlayer", "pitch_scale").unwrap()).unwrap();
        assert_eq!(pitch["slider"], json!(true));
    }
}
