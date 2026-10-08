//! Configurações do projeto (PHY-021, INP-001, AUD-003, EDT-023): gravidade, nomes das camadas de
//! colisão, matriz de colisão, mapa de entrada e barramentos de áudio. Ficam em `settings.ron` na
//! raiz do projeto. Um projeto sem o arquivo usa os padrões.

use crate::input::InputMap;
use serde::{Deserialize, Serialize};

/// Número máximo de camadas de colisão: cada uma é um bit de um `u32`.
pub const LAYER_COUNT: usize = 32;

/// Camadas que os projetos novos começam com. As demais ficam vazias até serem nomeadas.
pub const DEFAULT_LAYERS: &[&str] = &["default", "ground", "player", "enemy"];

/// Barramento de áudio: um canal com volume próprio, pelo qual passam os players.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bus {
    pub name: String,
    /// Volume em decibéis. 0 é o volume original, -80 é silêncio.
    pub volume_db: f64,
    pub mute: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Gravidade em pixels por segundo², com Y para baixo.
    pub gravity: [f64; 2],
    /// Nomes das camadas. O índice é o bit da camada. Nome vazio = camada livre.
    pub layers: Vec<String>,
    /// Para cada camada, o `u32` com os bits das camadas com que ela colide.
    pub layer_matrix: Vec<u32>,
    pub input: InputMap,
    pub buses: Vec<Bus>,
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
            buses: vec![Bus { name: "Master".into(), volume_db: 0.0, mute: false }],
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
        s.buses.push(Bus { name: "Música".into(), volume_db: -6.0, mute: false });
        let text = s.to_ron().unwrap();
        assert_eq!(ProjectSettings::from_ron(&text).unwrap(), s);
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
}
