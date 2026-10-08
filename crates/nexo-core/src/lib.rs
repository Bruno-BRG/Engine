//! Tipos base compartilhados entre o runtime, o renderizador e o editor.
//! A cena (árvore de nós, registro de tipos, arquivo RON e consultas) também vive aqui.

pub mod audio;
pub mod input;
pub mod io;
pub mod query;
pub mod registry;
pub mod scene;
pub mod settings;
pub mod timer;

pub use audio::{AudioCommand, AudioPlayback};
pub use input::{Input, InputMap};
pub use settings::ProjectSettings;
pub use io::FORMAT as SCENE_FORMAT;
pub use scene::{Node, Scene, Uid};

use bytemuck::{Pod, Zeroable};

/// Semente para os UIDs de cenas novas, a partir do relógio do sistema.
/// No WebAssembly o relógio não está disponível: quem chama passa a própria semente.
pub fn seed_now() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0x5EED_0000_0000_0001
    }
}

/// Câmera 2D do mundo. Coordenadas com Y para baixo, em unidades de mundo (pixels do jogo).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Camera {
    /// Ponto do mundo no centro da tela.
    pub center: [f32; 2],
    /// Tamanho do viewport em pixels físicos.
    pub viewport: [f32; 2],
    /// Resolução da tela do jogo, desenhada como moldura no editor.
    pub screen: [f32; 2],
    /// Pixels físicos por unidade de mundo.
    pub zoom: f32,
    /// Tempo em segundos, usado por efeitos do editor.
    pub time: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            viewport: [1.0, 1.0],
            screen: [1280.0, 720.0],
            zoom: 1.0,
            time: 0.0,
        }
    }
}

/// Forma procedural usada enquanto ainda não há texturas.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    RoundRect = 0,
    Circle = 1,
    Diamond = 2,
}

/// Bits do campo `flags` de `SpriteInstance`.
pub mod flags {
    pub const SELECTED: f32 = 1.0;
    pub const HOVERED: f32 = 2.0;
    pub const FLIP_X: f32 = 4.0;
    pub const FLIP_Y: f32 = 8.0;
}

/// Uma instância de sprite enviada para a GPU. Ocupa 12 floats (48 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpriteInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub rotation: f32,
    pub shape: f32,
    pub flags: f32,
    pub _pad: f32,
    pub color: [f32; 4],
}

pub const SPRITE_FLOATS: usize = 12;

const _: () = assert!(std::mem::size_of::<SpriteInstance>() == SPRITE_FLOATS * 4);
const _: () = assert!(std::mem::size_of::<Camera>() == 32);
