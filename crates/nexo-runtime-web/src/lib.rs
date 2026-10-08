//! Viewport do editor: o runtime do Nexo rodando em WebAssembly.

use nexo_core::{Camera, SPRITE_FLOATS, SpriteInstance};
use nexo_render::Renderer;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

mod editor;
mod logic;
pub use editor::SceneEditor;
pub use logic::GraphEditor;

#[wasm_bindgen]
pub struct Viewport {
    renderer: Renderer,
    sprites: Vec<SpriteInstance>,
}

#[wasm_bindgen]
impl Viewport {
    /// Cria o viewport sobre um canvas. Usa WebGPU quando disponível e WebGL2 como reserva.
    pub async fn create(canvas: HtmlCanvasElement) -> Result<Viewport, JsError> {
        let (w, h) = (canvas.width(), canvas.height());
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
        .await;
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsError::new(&format!("Falha ao criar a superfície: {e}")))?;
        let renderer = Renderer::new(&instance, surface, w, h)
            .await
            .map_err(|e| JsError::new(&e))?;
        Ok(Viewport { renderer, sprites: Vec::new() })
    }

    pub fn info(&self) -> String {
        self.renderer.info().to_string()
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn set_camera(&mut self, x: f32, y: f32, zoom: f32, time: f32, screen_w: f32, screen_h: f32) {
        self.renderer.set_camera(Camera {
            center: [x, y],
            zoom,
            time,
            screen: [screen_w, screen_h],
            ..Default::default()
        });
    }

    /// Recebe as instâncias como floats planos, 12 por sprite.
    pub fn set_sprites(&mut self, data: &[f32]) {
        let n = data.len() / SPRITE_FLOATS;
        self.sprites.clear();
        self.sprites
            .extend_from_slice(bytemuck::cast_slice(&data[..n * SPRITE_FLOATS]));
        self.renderer.set_sprites(&self.sprites);
    }

    /// Envia uma textura RGBA (não pré-multiplicada, linha de cima primeiro) para o slot.
    /// Troca a textura se o slot já existe. O slot 0 é reservado.
    pub fn upload_texture(&mut self, slot: u32, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsError> {
        self.renderer
            .upload_texture(slot, width, height, rgba)
            .map_err(|e| JsError::new(&e))
    }

    /// Libera a textura do slot. Não faz nada se o slot não existe.
    pub fn remove_texture(&mut self, slot: u32) {
        self.renderer.remove_texture(slot);
    }

    pub fn render(&mut self) -> Result<(), JsError> {
        self.renderer.render().map_err(|e| JsError::new(&e))
    }
}
