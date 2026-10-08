//! Renderizador 2D do Nexo sobre wgpu. Funciona em nativo e em WebGPU/WebGL2.

use std::collections::HashMap;

use nexo_core::{Camera, SpriteInstance};
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("shader.wgsl");

/// Bytes de cada instância de sprite no buffer (12 floats).
const SPRITE_STRIDE: u64 = std::mem::size_of::<SpriteInstance>() as u64;

/// Textura de um slot. O `BindGroup` aponta para a view e para o sampler compartilhado.
struct TextureEntry {
    _texture: wgpu::Texture,
    bind: wgpu::BindGroup,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    grid_pipeline: wgpu::RenderPipeline,
    sprite_pipeline: wgpu::RenderPipeline,
    texture_pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u32, TextureEntry>,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    instances: wgpu::Buffer,
    instance_capacity: usize,
    /// Slot de textura de cada sprite, na ordem de desenho (0 = procedural).
    sprite_slots: Vec<u32>,
    info: String,
}

impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Nenhum adaptador gráfico compatível: {e}"))?;

        let adapter_info = adapter.get_info();
        let backend = match adapter_info.backend {
            wgpu::Backend::BrowserWebGpu => "WebGPU".to_string(),
            wgpu::Backend::Gl if cfg!(target_arch = "wasm32") => "WebGL2".to_string(),
            wgpu::Backend::Gl => "OpenGL".to_string(),
            wgpu::Backend::Dx12 => "DirectX 12".to_string(),
            other => format!("{other:?}"),
        };
        let info = if adapter_info.name.trim().is_empty() {
            backend
        } else {
            format!("{backend} · {}", adapter_info.name)
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("nexo-device"),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("Falha ao criar o dispositivo gráfico: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        // Formato não-sRGB: as cores da engine são as mesmas do CSS do editor.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes[0]
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nexo-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let camera = Camera::default();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera"),
            contents: bytemuck::bytes_of(&camera),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bind"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nexo-layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            ..Default::default()
        });

        // Texturas: grupo 1 com textura filtrável e sampler linear, com borda travada.
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let texture_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("nexo-texture-layout"),
                bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
                ..Default::default()
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let target = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_grid"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_grid"),
                compilation_options: Default::default(),
                targets: &target,
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sprite_pipeline =
            make_sprite_pipeline(&device, &shader, &layout, "fs_sprite", &target);
        let texture_pipeline =
            make_sprite_pipeline(&device, &shader, &texture_pipeline_layout, "fs_tex", &target);

        let instance_capacity = 256;
        let instances = Self::make_instance_buffer(&device, instance_capacity);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            grid_pipeline,
            sprite_pipeline,
            texture_pipeline,
            texture_layout,
            sampler,
            textures: HashMap::new(),
            camera,
            camera_buffer,
            camera_bind,
            instances,
            instance_capacity,
            sprite_slots: Vec::new(),
            info,
        })
    }

    fn make_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite-instances"),
            size: (capacity * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Backend e nome da GPU, para a barra de status.
    pub fn info(&self) -> &str {
        &self.info
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let max = self.device.limits().max_texture_dimension_2d;
        self.config.width = width.clamp(1, max);
        self.config.height = height.clamp(1, max);
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_camera(&mut self, mut camera: Camera) {
        camera.viewport = [self.config.width as f32, self.config.height as f32];
        self.camera = camera;
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&self.camera));
    }

    pub fn set_sprites(&mut self, sprites: &[SpriteInstance]) {
        if sprites.len() > self.instance_capacity {
            self.instance_capacity = sprites.len().next_power_of_two();
            self.instances = Self::make_instance_buffer(&self.device, self.instance_capacity);
        }
        if !sprites.is_empty() {
            self.queue
                .write_buffer(&self.instances, 0, bytemuck::cast_slice(sprites));
        }
        self.sprite_slots.clear();
        self.sprite_slots
            .extend(sprites.iter().map(|s| slot_from_pad(s._pad)));
    }

    /// Cria ou troca a textura do slot. `rgba` tem `width * height * 4` bytes (RGBA
    /// não pré-multiplicado), com a linha de cima primeiro. Slot 0 é reservado.
    pub fn upload_texture(
        &mut self,
        slot: u32,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        if slot == 0 {
            return Err("O slot 0 é reservado para sprites sem textura".to_string());
        }
        let max = self.device.limits().max_texture_dimension_2d;
        if width == 0 || height == 0 || width > max || height > max {
            return Err(format!("Textura {width}x{height} fora do limite de {max} px"));
        }
        let expected = width as u64 * height as u64 * 4;
        if rgba.len() as u64 != expected {
            return Err(format!(
                "Textura {width}x{height} espera {expected} bytes, recebeu {}",
                rgba.len()
            ));
        }

        // Pré-multiplica o alfa: a filtragem linear não escurece as bordas transparentes.
        let mut pixels = rgba.to_vec();
        for px in pixels.chunks_exact_mut(4) {
            let a = px[3] as u32;
            for c in &mut px[..3] {
                *c = ((*c as u32 * a + 127) / 255) as u8;
            }
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite-texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite-texture-bind"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        // Trocar um slot já usado solta a textura antiga.
        self.textures.insert(
            slot,
            TextureEntry {
                _texture: texture,
                bind,
            },
        );
        Ok(())
    }

    /// Libera a textura do slot. Não faz nada se o slot não existe.
    pub fn remove_texture(&mut self, slot: u32) {
        self.textures.remove(&slot);
    }

    pub fn render(&mut self) -> Result<(), String> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => return Err(format!("Falha ao obter o frame: {other:?}")),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.055, g: 0.059, b: 0.075, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_pipeline(&self.grid_pipeline);
            pass.draw(0..3, 0..1);

            // Um draw por trecho, na ordem de desenho (z-order já vem do chamador).
            let has_texture = |slot: u32| self.textures.contains_key(&slot);
            for run in plan_runs(&self.sprite_slots, has_texture) {
                match run.kind {
                    RunKind::Procedural => pass.set_pipeline(&self.sprite_pipeline),
                    RunKind::Textured(slot) => {
                        pass.set_pipeline(&self.texture_pipeline);
                        pass.set_bind_group(1, &self.textures[&slot].bind, &[]);
                    }
                }
                // Recorta o buffer no trecho: o draw começa no instance 0 de cada um.
                let start = run.start as u64 * SPRITE_STRIDE;
                let end = start + run.count as u64 * SPRITE_STRIDE;
                pass.set_vertex_buffer(0, self.instances.slice(start..end));
                pass.draw(0..6, 0..run.count);
            }
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(())
    }
}

/// Pipeline das instâncias de sprite. Procedurais e texturizados usam o mesmo vértice
/// e o mesmo layout de instância; só muda o fragment.
fn make_sprite_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fs_entry: &str,
    target: &[Option<wgpu::ColorTargetState>],
) -> wgpu::RenderPipeline {
    let attrs = [
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 16, shader_location: 2 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 20, shader_location: 3 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 24, shader_location: 4 },
        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 5 },
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprites"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_sprite"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: SPRITE_STRIDE,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &attrs,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            compilation_options: Default::default(),
            targets: target,
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Slot de textura guardado em `_pad`, arredondado. Valores inválidos (negativos, NaN)
/// viram 0, que é procedural.
fn slot_from_pad(pad: f32) -> u32 {
    pad.round() as u32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunKind {
    /// Formas procedurais. A forma vem de cada instância, então todas cabem num trecho.
    Procedural,
    /// Sprites que usam a textura deste slot.
    Textured(u32),
}

/// Trecho contíguo de sprites desenhado por uma única chamada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    kind: RunKind,
    /// Índice do primeiro sprite do trecho.
    start: u32,
    count: u32,
}

/// Divide os sprites, na ordem de desenho, em trechos. Uma troca de tipo ou de slot
/// começa um trecho novo. Slot 0, ou slot sem textura enviada, é procedural.
fn plan_runs(slots: &[u32], has_texture: impl Fn(u32) -> bool) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (i, &slot) in slots.iter().enumerate() {
        let kind = if slot != 0 && has_texture(slot) {
            RunKind::Textured(slot)
        } else {
            RunKind::Procedural
        };
        match runs.last_mut() {
            Some(run) if run.kind == kind => run.count += 1,
            _ => runs.push(Run {
                kind,
                start: i as u32,
                count: 1,
            }),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(kind: RunKind, start: u32, count: u32) -> Run {
        Run { kind, start, count }
    }

    #[test]
    fn trechos_agrupam_slots_consecutivos() {
        // Slot 3 não tem textura enviada: vira procedural e se junta ao trecho vizinho.
        let slots = [0, 0, 1, 1, 2, 0, 3, 3];
        let has = |s: u32| s == 1 || s == 2;
        assert_eq!(
            plan_runs(&slots, has),
            vec![
                run(RunKind::Procedural, 0, 2),
                run(RunKind::Textured(1), 2, 2),
                run(RunKind::Textured(2), 4, 1),
                run(RunKind::Procedural, 5, 3),
            ]
        );
    }

    #[test]
    fn lista_vazia_nao_gera_trechos() {
        assert!(plan_runs(&[], |_| true).is_empty());
    }

    #[test]
    fn slot_do_pad_e_arredondado_e_invalido_vira_zero() {
        assert_eq!(slot_from_pad(2.0), 2);
        assert_eq!(slot_from_pad(2.9999), 3);
        assert_eq!(slot_from_pad(0.0), 0);
        assert_eq!(slot_from_pad(-1.0), 0);
        assert_eq!(slot_from_pad(f32::NAN), 0);
    }
}
