//! Configurações do projeto (PHY-021, INP-001, AUD-003, EDT-023): gravidade, nomes das camadas de
//! colisão, matriz de colisão, mapa de entrada e barramentos de áudio. Ficam em `settings.ron` na
//! raiz do projeto. Um projeto sem o arquivo usa os padrões.

use crate::audio::BusEffects;
use crate::input::InputMap;
use serde::{Deserialize, Serialize};

/// Número máximo de camadas de colisão: cada uma é um bit de um `u32`.
pub const LAYER_COUNT: usize = 32;

/// Camadas que os projetos novos começam com. As demais ficam vazias até serem nomeadas.
pub const DEFAULT_LAYERS: &[&str] = &["default", "ground", "player", "enemy"];

/// Material de física (PHY-023): atrito, quique e as duas formas de superfície do Rapier.
/// Um corpo usa um material pelo nome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsMaterial {
    pub name: String,
    /// Atrito entre 0 (gelo) e 1 ou mais.
    pub friction: f64,
    /// Quique entre 0 (sem quique) e 1 (quique total).
    pub bounce: f64,
    /// Superfície áspera: usa o atrito máximo entre os dois corpos, em vez da média.
    pub rough: bool,
    /// Superfície absorvente: usa o quique mínimo entre os dois corpos.
    pub absorbent: bool,
}

/// Tags padrão dos projetos novos (SCN-033). Cada nó tem uma tag, usada pela lógica visual.
pub const DEFAULT_TAGS: &[&str] = &["untagged", "player", "enemy", "coin"];

/// Barramento de áudio: um canal com volume próprio, pelo qual passam os players.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bus {
    pub name: String,
    /// Volume em decibéis. 0 é o volume original, -80 é silêncio.
    pub volume_db: f64,
    pub mute: bool,
    /// Efeitos do barramento (AUD-003). Quando estão nos padrões, não entram no arquivo.
    #[serde(default, skip_serializing_if = "BusEffects::is_default")]
    pub effects: BusEffects,
}

/// `serde(default)`: um settings.ron de versão anterior, sem os campos novos, ainda abre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    /// Gravidade em pixels por segundo², com Y para baixo.
    pub gravity: [f64; 2],
    /// Nomes das camadas. O índice é o bit da camada. Nome vazio = camada livre.
    pub layers: Vec<String>,
    /// Para cada camada, o `u32` com os bits das camadas com que ela colide.
    pub layer_matrix: Vec<u32>,
    pub input: InputMap,
    pub buses: Vec<Bus>,
    /// Tags disponíveis para os nós (SCN-033).
    pub tags: Vec<String>,
    pub physics_materials: Vec<PhysicsMaterial>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        let mut layers: Vec<String> = DEFAULT_LAYERS.iter().map(|s| s.to_string()).collect();
        layers.resize(LAYER_COUNT, String::new());
        ProjectSettings {
            gravity: [0.0, 980.0],
            layers,
            layer_matrix: vec![u32::MAX; LAYER_COUNT],
            input: InputMap::defaults(),
            buses: vec![Bus { name: "Master".into(), volume_db: 0.0, mute: false, effects: BusEffects::default() }],
            tags: DEFAULT_TAGS.iter().map(|s| s.to_string()).collect(),
            physics_materials: Vec::new(),
        }
    }
}

impl ProjectSettings {
    /// Bit da camada com o nome dado, ou `None` se não existe. Camada inexistente não colide com nada.
    pub fn layer_bit(&self, name: &str) -> Option<u32> {
        self.layers.iter().position(|l| !l.is_empty() && l == name).map(|i| 1u32 << i)
    }

    /// Máscara de colisão de uma camada (pela posição), ou 0 se o índice não existe.
    pub fn matrix_of(&self, layer_index: usize) -> u32 {
        self.layer_matrix.get(layer_index).copied().unwrap_or(0)
    }

    /// Ajusta o tamanho das listas para `LAYER_COUNT` e garante a forma correta depois de ler o arquivo.
    pub fn normalize(&mut self) {
        self.layers.truncate(LAYER_COUNT);
        self.layers.resize(LAYER_COUNT, String::new());
        self.layer_matrix.truncate(LAYER_COUNT);
        self.layer_matrix.resize(LAYER_COUNT, u32::MAX);
        for bus in &mut self.buses {
            bus.effects.normalize();
        }
    }

    pub fn to_ron(&self) -> Result<String, String> {
        let cfg = ron::ser::PrettyConfig::new().struct_names(false);
        ron::ser::to_string_pretty(self, cfg).map_err(|e| e.to_string())
    }

    /// Lê as configurações. Texto vazio vira o padrão. Listas com tamanho errado são corrigidas.
    pub fn from_ron(text: &str) -> Result<ProjectSettings, String> {
        if text.trim().is_empty() {
            return Ok(ProjectSettings::default());
        }
        let mut s: ProjectSettings =
            ron::from_str(text).map_err(|e| format!("Configurações do projeto ilegíveis: {e}"))?;
        s.normalize();
        Ok(s)
    }

    /// JSON com os mesmos nomes de campo do RON (snake_case). É o formato que o editor WASM e o
    /// TypeScript usam; o arquivo `settings.ron` continua em RON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    /// Lê o JSON das configurações, com as mesmas regras de `from_ron`.
    pub fn from_json(text: &str) -> Result<ProjectSettings, String> {
        if text.trim().is_empty() {
            return Ok(ProjectSettings::default());
        }
        let mut s: ProjectSettings = serde_json::from_str(text)
            .map_err(|e| format!("Configurações do projeto inválidas: {e}"))?;
        s.normalize();
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padroes_tem_camadas_e_matriz_completas() {
        let s = ProjectSettings::default();
        assert_eq!(s.layers.len(), LAYER_COUNT);
        assert_eq!(s.layer_matrix.len(), LAYER_COUNT);
        assert_eq!(s.layer_bit("ground"), Some(1 << 1));
        assert_eq!(s.layer_bit("inexistente"), None);
        assert_eq!(s.matrix_of(0), u32::MAX);
        assert_eq!(s.matrix_of(99), 0);
    }

    #[test]
    fn ida_e_volta_ron() {
        let mut s = ProjectSettings::default();
        s.gravity = [0.0, 1200.0];
        s.layers[5] = "coin".into();
        s.layer_matrix[0] = 0b11;
        let mut efeitos = BusEffects::default();
        efeitos.reverb.enabled = true;
        efeitos.low_pass.cutoff_hz = 900.0;
        s.buses.push(Bus { name: "Música".into(), volume_db: -6.0, mute: false, effects: efeitos });
        s.tags.push("boss".into());
        s.physics_materials.push(PhysicsMaterial {
            name: "Gelo".into(),
            friction: 0.05,
            bounce: 0.0,
            rough: false,
            absorbent: false,
        });
        let text = s.to_ron().unwrap();
        assert_eq!(ProjectSettings::from_ron(&text).unwrap(), s);
    }

    #[test]
    fn barramento_sem_efeitos_nao_grava_o_bloco_de_efeitos() {
        let text = ProjectSettings::default().to_ron().unwrap();
        assert!(!text.contains("reverb"), "efeitos nos padrões não vão para o arquivo");
        let mut s = ProjectSettings::default();
        s.buses[0].effects.delay.enabled = true;
        assert!(s.to_ron().unwrap().contains("delay"));
    }

    #[test]
    fn texto_vazio_vira_padrao_e_lista_curta_e_corrigida() {
        assert_eq!(ProjectSettings::from_ron("  ").unwrap(), ProjectSettings::default());
        let mut s = ProjectSettings::default();
        s.layers.truncate(3);
        s.layer_matrix.clear();
        let text = s.to_ron().unwrap();
        let fixed = ProjectSettings::from_ron(&text).unwrap();
        assert_eq!(fixed.layers.len(), LAYER_COUNT);
        assert_eq!(fixed.layer_matrix.len(), LAYER_COUNT);
    }

    #[test]
    fn arquivo_invalido_da_erro_legivel() {
        let err = ProjectSettings::from_ron("isto não é ron (").unwrap_err();
        assert!(err.contains("ilegíveis"));
    }

    #[test]
    fn ida_e_volta_json() {
        let mut s = ProjectSettings::default();
        s.gravity = [0.0, 1200.0];
        s.layers[2] = "heroi".into();
        s.layer_matrix[2] = 0b101;
        s.tags.push("boss".into());
        s.physics_materials.push(PhysicsMaterial {
            name: "Borracha".into(),
            friction: 0.9,
            bounce: 0.8,
            rough: true,
            absorbent: false,
        });
        let json = s.to_json().unwrap();
        assert_eq!(ProjectSettings::from_json(&json).unwrap(), s);
    }

    #[test]
    fn json_usa_os_nomes_de_campo_do_contrato_typescript() {
        // O TypeScript (editor/src/lib/settings.ts) lê estes nomes. Se mudarem aqui, mude lá também.
        let mut s = ProjectSettings::default();
        s.physics_materials.push(PhysicsMaterial {
            name: "Gelo".into(),
            friction: 0.05,
            bounce: 0.0,
            rough: false,
            absorbent: true,
        });
        let v: serde_json::Value = serde_json::from_str(&s.to_json().unwrap()).unwrap();
        for campo in ["gravity", "layers", "layer_matrix", "input", "buses", "tags", "physics_materials"] {
            assert!(v.get(campo).is_some(), "falta o campo {campo}");
        }
        assert!(v["buses"][0].get("volume_db").is_some());
        assert!(v["input"]["actions"].is_object());
        assert!(v["physics_materials"][0].get("absorbent").is_some());
        assert!(v["physics_materials"][0].get("friction").is_some());
    }

    #[test]
    fn json_vazio_vira_padrao_e_lista_curta_e_corrigida() {
        assert_eq!(ProjectSettings::from_json("").unwrap(), ProjectSettings::default());
        let mut s = ProjectSettings::default();
        s.layers.truncate(3);
        s.layer_matrix.clear();
        let fixed = ProjectSettings::from_json(&s.to_json().unwrap()).unwrap();
        assert_eq!(fixed.layers.len(), LAYER_COUNT);
        assert_eq!(fixed.layer_matrix.len(), LAYER_COUNT);
    }

    #[test]
    fn json_invalido_da_erro_legivel() {
        let err = ProjectSettings::from_json("{ isto não é json").unwrap_err();
        assert!(err.contains("inválidas"));
    }

    #[test]
    fn campo_faltando_usa_padrao() {
        // Arquivo de uma versão anterior, sem tags nem materiais: abre com os padrões desses campos.
        let s = ProjectSettings::from_json(r#"{"gravity":[0.0,500.0]}"#).unwrap();
        assert_eq!(s.gravity, [0.0, 500.0]);
        assert_eq!(s.tags, ProjectSettings::default().tags);
        assert!(s.physics_materials.is_empty());
        assert_eq!(s.layers.len(), LAYER_COUNT);
        let r = ProjectSettings::from_ron("(gravity: (0.0, 300.0))").unwrap();
        assert_eq!(r.gravity, [0.0, 300.0]);
        assert_eq!(r.tags, ProjectSettings::default().tags);
    }
}
