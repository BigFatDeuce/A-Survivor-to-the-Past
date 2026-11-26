//! Rendu wgpu : un pipeline de sprites instancies, puis une passe de filtre video.
//!
//! Principe "pixels nets, mouvement fluide" : chaque sprite est dessine directement a la
//! resolution de l'ecran, ses pixels devenant des carres de S x S pixels (S entier).
//! L'origine du sprite est calee sur un pixel ecran (mouvement fluide a haut framerate),
//! et la rotation / mise a l'echelle sont calculees dans la grille du sprite (aspect SNES).
use crate::{LOGICAL_H, LOGICAL_W};
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use std::path::PathBuf;
use std::sync::Arc;
use winit::window::Window;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct SpriteInstance {
    /// Position du point d'ancrage (pixels logiques, monde ou interface).
    pub pos: [f32; 2],
    /// Rectangle dans l'atlas (x, y, l, h) en texels.
    pub uv: [f32; 4],
    /// Point d'ancrage dans la frame.
    pub pivot: [f32; 2],
    /// Echelle (negative = miroir).
    pub scale: [f32; 2],
    pub rot: f32,
    /// Teinte multiplicative.
    pub color: [f32; 4],
    /// Couleur de flash (rgb) et intensite (a).
    pub flash: [f32; 4],
}

/// Filtre video applique a l'image finale (passe de post-traitement).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum VideoFilter {
    /// Pixels nets, sans effet (par defaut).
    #[default]
    None,
    /// Sortie composite d'une console 16 bits sur televiseur : leger flou horizontal, lignes discretes.
    Snes,
    /// Televiseur cathodique : courbure, masque RGB, lignes marquees, vignettage.
    Crt,
    /// Pixels nets et lignes de balayage.
    Scanlines,
    /// Ecran LCD de console portable 32 bits : grille de pixels, couleurs ternies.
    Gba,
    /// Ecran de la premiere console portable : quatre tons de vert, trame ordonnee.
    GameBoy,
}

impl VideoFilter {
    pub const ALL: [VideoFilter; 6] = [
        VideoFilter::None,
        VideoFilter::Snes,
        VideoFilter::Crt,
        VideoFilter::Scanlines,
        VideoFilter::Gba,
        VideoFilter::GameBoy,
    ];
    fn code(self) -> u32 {
        match self {
            VideoFilter::None => 0,
            VideoFilter::Snes => 1,
            VideoFilter::Crt => 2,
            VideoFilter::Scanlines => 3,
            VideoFilter::Gba => 4,
            VideoFilter::GameBoy => 5,
        }
    }
}

/// Format de l'image de jeu dans la fenetre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Aspect {
    /// Toute la fenetre (16:9 et plus large).
    #[default]
    Wide,
    /// Image 4:3 centree, bandes noires sur les cotes (champ de vision reduit).
    Classic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blend {
    Alpha,
    Add,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Space {
    /// Coordonnees du monde (camera).
    World,
    /// Coordonnees de l'interface (0,0 = coin haut-gauche).
    Ui,
}

#[derive(Clone, Copy, Debug)]
pub struct Batch {
    pub space: Space,
    pub blend: Blend,
    pub start: u32,
    pub count: u32,
}

/// Element releve pour la verification des placements de l'interface.
#[derive(Clone, Debug)]
pub enum Probe {
    /// Texte dessine en coordonnees d'interface : rectangle et contenu.
    Text { pos: Vec2, size: Vec2, text: String },
    /// Cadre ou panneau qui doit contenir les textes qui y commencent.
    Panel { pos: Vec2, size: Vec2, name: &'static str },
    /// Caractere absent de la police.
    MissingGlyph { ch: char, text: String },
    /// Voile plein ecran : ce qui suit est un nouveau calque (menu par-dessus le jeu assombri).
    Layer,
}

/// Liste de dessin d'une image : instances + lots (changement d'espace ou de melange).
#[derive(Default)]
pub struct DrawList {
    pub instances: Vec<SpriteInstance>,
    pub batches: Vec<Batch>,
    state: Option<(Space, Blend)>,
    /// Releve des textes et cadres (verification de l'interface), desactive en jeu.
    pub probe: Option<Vec<Probe>>,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.instances.clear();
        self.batches.clear();
        self.state = None;
    }

    pub fn set(&mut self, space: Space, blend: Blend) {
        self.state = Some((space, blend));
    }

    pub fn space(&self) -> Space {
        self.state.map(|s| s.0).unwrap_or(Space::World)
    }

    /// Declare une zone pleine (icones, jauges) qu'aucun texte ne doit chevaucher.
    pub fn probe_block(&mut self, pos: Vec2, size: Vec2, name: &str) {
        if let Some(p) = self.probe.as_mut() {
            p.push(Probe::Text { pos, size, text: format!("<{name}>") });
        }
    }

    /// Nouveau calque (voile plein ecran sous un menu).
    pub fn probe_layer(&mut self) {
        if let Some(p) = self.probe.as_mut() {
            p.push(Probe::Layer);
        }
    }

    /// Declare un panneau (verification de l'interface uniquement).
    pub fn probe_panel(&mut self, pos: Vec2, size: Vec2, name: &'static str) {
        if let Some(p) = self.probe.as_mut() {
            p.push(Probe::Panel { pos, size, name });
        }
    }

    pub fn push(&mut self, inst: SpriteInstance) {
        let (space, blend) = self.state.unwrap_or((Space::World, Blend::Alpha));
        let idx = self.instances.len() as u32;
        match self.batches.last_mut() {
            Some(b) if b.space == space && b.blend == blend && b.start + b.count == idx => b.count += 1,
            _ => self.batches.push(Batch { space, blend, start: idx, count: 1 }),
        }
        self.instances.push(inst);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    origin: [f32; 2],
    offset: [f32; 2],
    screen: [f32; 2],
    scale: f32,
    _p: f32,
}

const SHADER: &str = r#"
struct View { origin: vec2<f32>, offset: vec2<f32>, screen: vec2<f32>, scale: f32, _p: f32 };
@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var tex: texture_2d<f32>;

struct VIn {
    @builtin(vertex_index) vi: u32,
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) pivot: vec2<f32>,
    @location(3) scale: vec2<f32>,
    @location(4) rot: f32,
    @location(5) color: vec4<f32>,
    @location(6) flash: vec4<f32>,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) q: vec2<f32>,
    @location(1) @interpolate(flat) uv: vec4<f32>,
    @location(2) @interpolate(flat) pivot: vec2<f32>,
    @location(3) @interpolate(flat) xf: vec3<f32>,
    @location(4) @interpolate(flat) color: vec4<f32>,
    @location(5) @interpolate(flat) flash: vec4<f32>,
};

fn xform(p: vec2<f32>, s: vec2<f32>, c: f32, sn: f32) -> vec2<f32> {
    let a = p * s;
    return vec2<f32>(a.x * c - a.y * sn, a.x * sn + a.y * c);
}

@vertex
fn vs(in: VIn) -> VOut {
    let size = in.uv.zw;
    let c = cos(in.rot);
    let sn = sin(in.rot);
    let p0 = xform(-in.pivot, in.scale, c, sn);
    let p1 = xform(vec2<f32>(size.x, 0.0) - in.pivot, in.scale, c, sn);
    let p2 = xform(vec2<f32>(0.0, size.y) - in.pivot, in.scale, c, sn);
    let p3 = xform(size - in.pivot, in.scale, c, sn);
    var mn = min(min(p0, p1), min(p2, p3));
    var mx = max(max(p0, p1), max(p2, p3));
    let simple = in.rot == 0.0 && abs(in.scale.x) == 1.0 && abs(in.scale.y) == 1.0;
    if (!simple) {
        mn = floor(mn) - vec2<f32>(1.0);
        mx = ceil(mx) + vec2<f32>(1.0);
    }
    let corner = vec2<f32>(f32(in.vi & 1u), f32(in.vi >> 1u));
    let q = mix(mn, mx, corner);
    let o = round((in.pos - view.origin) * view.scale + view.offset);
    let spx = o + q * view.scale;
    var out: VOut;
    out.clip = vec4<f32>(spx.x / view.screen.x * 2.0 - 1.0, 1.0 - spx.y / view.screen.y * 2.0, 0.0, 1.0);
    out.q = q;
    out.uv = in.uv;
    out.pivot = in.pivot;
    out.xf = vec3<f32>(in.scale, in.rot);
    out.color = in.color;
    out.flash = in.flash;
    return out;
}

@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    let c = cos(in.xf.z);
    let sn = sin(in.xf.z);
    // echantillonnage dans la grille logique du sprite : pixels carres meme tourne/agrandi
    let qs = floor(in.q) + vec2<f32>(0.5);
    let r = vec2<f32>(qs.x * c + qs.y * sn, -qs.x * sn + qs.y * c);
    let p = r / in.xf.xy + in.pivot;
    if (p.x < 0.0 || p.y < 0.0 || p.x >= in.uv.z || p.y >= in.uv.w) {
        discard;
    }
    let t = vec2<i32>(floor(in.uv.xy + p));
    var col = textureLoad(tex, t, 0) * in.color;
    if (col.a < 0.004) {
        discard;
    }
    col = vec4<f32>(mix(col.rgb, in.flash.rgb, in.flash.a), col.a);
    return col;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostUniform {
    /// Coin haut-gauche de l'image de jeu dans la fenetre, puis sa taille (pixels ecran).
    region: [f32; 4],
    scale: f32,
    mode: u32,
    _p: [f32; 2],
}

const POST_SHADER: &str = include_str!("post.wgsl");

/// Texture intermediaire ou le jeu est dessine avant le filtre video.
struct Scene {
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
    size: (u32, u32),
}

pub struct Gfx {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Format de rendu (toujours sRGB ; vue sRGB d'un canevas web qui ne l'est pas).
    target: wgpu::TextureFormat,
    pipe_alpha: wgpu::RenderPipeline,
    pipe_add: wgpu::RenderPipeline,
    view_bufs: [wgpu::Buffer; 2],
    view_groups: [wgpu::BindGroup; 2],
    tex_layout: wgpu::BindGroupLayout,
    tex_group: Option<wgpu::BindGroup>,
    inst_buf: wgpu::Buffer,
    inst_cap: usize,
    /// Facteur d'agrandissement entier (pixels ecran par pixel du jeu).
    pub scale: u32,
    /// Taille de la vue en pixels logiques.
    pub view: Vec2,
    pub vsync: bool,
    capture: Option<PathBuf>,
    pub adapter_name: String,
    post_pipe: wgpu::RenderPipeline,
    post_layout: wgpu::BindGroupLayout,
    post_buf: wgpu::Buffer,
    scene: Option<Scene>,
    pub video: VideoFilter,
    pub aspect: Aspect,
    /// Coin haut-gauche de l'image de jeu dans la fenetre (pixels ecran, bandes noires en 4:3).
    pub origin: Vec2,
    /// Taille de l'image de jeu (pixels ecran).
    region: (u32, u32),
}

impl Gfx {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(window: Arc<Window>, vsync: bool) -> Gfx {
        pollster::block_on(Self::new_async(window, vsync))
    }

    /// Initialisation (asynchrone dans le navigateur : WebGPU si disponible, sinon WebGL2).
    pub async fn new_async(window: Arc<Window>, vsync: bool) -> Gfx {
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor::new_without_display_handle()).await;
        #[cfg(not(target_arch = "wasm32"))]
        // WGPU_BACKEND=gl / vulkan... (consoles portables : OpenGL ES) ; l'affichage sert a EGL
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(window.clone())));
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("aucune carte graphique compatible");
        let adapter_name = format!("{} ({:?})", adapter.get_info().name, adapter.get_info().backend);
        log::info!("GPU : {adapter_name}");
        // limites modestes (WebGL2) : le jeu n'a besoin de rien de plus
        let desc = wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        };
        let (device, queue) = adapter.request_device(&desc).await.expect("device");
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let target = format.add_srgb_suffix();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync },
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: if target != format { vec![target] } else { vec![] },
        };
        surface.configure(&device, &config);
        let format = target;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprites"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&view_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let attrs = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x4, 2 => Float32x2, 3 => Float32x2,
            4 => Float32, 5 => Float32x4, 6 => Float32x4
        ];
        let make_pipe = |blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sprites"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SpriteInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &attrs,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipe_alpha = make_pipe(wgpu::BlendState::ALPHA_BLENDING);
        let pipe_add = make_pipe(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        });
        let mk_view = || {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: std::mem::size_of::<ViewUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let view_bufs = [mk_view(), mk_view()];
        let mk_group = |b: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &view_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: b.as_entire_binding() }],
            })
        };
        let view_groups = [mk_group(&view_bufs[0]), mk_group(&view_bufs[1])];

        // passe de filtre video
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(POST_SHADER.into()),
        });
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let post_pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&post_layout)],
            immediate_size: 0,
        });
        let post_pipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("post"),
            layout: Some(&post_pipe_layout),
            vertex: wgpu::VertexState {
                module: &post_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let post_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: std::mem::size_of::<PostUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let inst_cap = 16384;
        let inst_buf = Self::make_inst_buf(&device, inst_cap);
        let mut g = Gfx {
            window,
            surface,
            device,
            queue,
            config,
            target,
            pipe_alpha,
            pipe_add,
            view_bufs,
            view_groups,
            tex_layout,
            tex_group: None,
            inst_buf,
            inst_cap,
            scale: 1,
            view: Vec2::new(LOGICAL_W, LOGICAL_H),
            vsync,
            capture: None,
            adapter_name,
            post_pipe,
            post_layout,
            post_buf,
            scene: None,
            video: VideoFilter::default(),
            aspect: Aspect::default(),
            origin: Vec2::ZERO,
            region: (1, 1),
        };
        g.update_scale();
        g
    }

    /// Change le filtre video et le format d'image.
    pub fn set_video(&mut self, filter: VideoFilter, aspect: Aspect) {
        self.video = filter;
        if aspect != self.aspect {
            self.aspect = aspect;
            self.update_scale();
        }
    }

    /// (Re)cree la texture intermediaire a la taille de l'image de jeu.
    fn ensure_scene(&mut self) {
        let size = self.region;
        if self.scene.as_ref().is_some_and(|s| s.size == size) {
            return;
        }
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.target,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.post_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: self.post_buf.as_entire_binding() },
            ],
        });
        self.scene = Some(Scene { view, group, size });
    }

    fn make_inst_buf(device: &wgpu::Device, cap: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (cap * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Charge l'atlas (RGBA8, sRGB).
    pub fn set_atlas(&mut self, rgba: &[u8], w: u32, h: u32) {
        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            tex.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
            size,
        );
        let view = tex.create_view(&Default::default());
        self.tex_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }],
        }));
    }

    fn update_scale(&mut self) {
        let (sw, sh) = (self.config.width as f32, self.config.height as f32);
        let (w, h) = match self.aspect {
            Aspect::Wide => (sw, sh),
            Aspect::Classic => {
                let w = sw.min((sh * 4.0 / 3.0).round());
                (w, sh.min((w * 3.0 / 4.0).round()))
            }
        };
        // agrandissement entier le plus grand qui garde au moins 360 de haut et 480 (4:3) de large :
        // 1920x1080 -> x3 (640x360), 1024x768 -> x2 (512x384), 1280x1024 -> x2 (640x512)
        let s = (w / (LOGICAL_H * 4.0 / 3.0)).min(h / LOGICAL_H).floor().max(1.0);
        self.scale = s as u32;
        self.view = Vec2::new(w / s, h / s);
        self.region = (w as u32, h as u32);
        self.origin = Vec2::new(((sw - w) * 0.5).floor(), ((sh - h) * 0.5).floor());
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.update_scale();
    }

    pub fn set_vsync(&mut self, on: bool) {
        self.vsync = on;
        self.config.present_mode = if on { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        self.surface.configure(&self.device, &self.config);
    }

    pub fn screen_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Capture la prochaine image dans un fichier PNG.
    pub fn request_capture(&mut self, path: PathBuf) {
        self.capture = Some(path);
    }

    pub fn render(&mut self, list: &DrawList, camera: Vec2, clear: [f32; 3]) {
        self.ensure_scene();
        let (w, h) = (self.region.0 as f32, self.region.1 as f32);
        let post = PostUniform {
            region: [self.origin.x, self.origin.y, w, h],
            scale: self.scale as f32,
            mode: self.video.code(),
            _p: [0.0; 2],
        };
        self.queue.write_buffer(&self.post_buf, 0, bytemuck::bytes_of(&post));
        let s = self.scale as f32;
        let world = ViewUniform {
            origin: camera.to_array(),
            offset: [(w * 0.5).round(), (h * 0.5).round()],
            screen: [w, h],
            scale: s,
            _p: 0.0,
        };
        let ui = ViewUniform { origin: [0.0, 0.0], offset: [0.0, 0.0], screen: [w, h], scale: s, _p: 0.0 };
        self.queue.write_buffer(&self.view_bufs[0], 0, bytemuck::bytes_of(&world));
        self.queue.write_buffer(&self.view_bufs[1], 0, bytemuck::bytes_of(&ui));
        if list.instances.len() > self.inst_cap {
            while self.inst_cap < list.instances.len() {
                self.inst_cap *= 2;
            }
            self.inst_buf = Self::make_inst_buf(&self.device, self.inst_cap);
        }
        if !list.instances.is_empty() {
            self.queue.write_buffer(&self.inst_buf, 0, bytemuck::cast_slice(&list.instances));
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(self.target), ..Default::default() });
        let mut enc = self.device.create_command_encoder(&Default::default());
        let scene = self.scene.as_ref().expect("scene");
        self.encode(&mut enc, &scene.view, list, clear);
        self.post(&mut enc, &target);

        // capture eventuelle (meme rendu dans une texture lisible)
        let mut readback = None;
        if let Some(path) = self.capture.take() {
            let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("capture"),
                size: wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.target,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            self.post(&mut enc, &view);
            let bpr = (self.config.width * 4).div_ceil(256) * 256;
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (bpr * self.config.height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                tex.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buf,
                    layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: None },
                },
                wgpu::Extent3d { width: self.config.width, height: self.config.height, depth_or_array_layers: 1 },
            );
            readback = Some((buf, bpr, path));
        }
        self.queue.submit([enc.finish()]);
        self.queue.present(frame);
        if let Some((buf, bpr, path)) = readback {
            self.save_capture(buf, bpr, path);
        }
    }

    fn encode(&self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, list: &DrawList, clear: [f32; 3]) {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear[0] as f64,
                        g: clear[1] as f64,
                        b: clear[2] as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let Some(tex_group) = &self.tex_group else { return };
        rp.set_bind_group(1, tex_group, &[]);
        rp.set_vertex_buffer(0, self.inst_buf.slice(..));
        for b in &list.batches {
            rp.set_pipeline(match b.blend {
                Blend::Alpha => &self.pipe_alpha,
                Blend::Add => &self.pipe_add,
            });
            rp.set_bind_group(0, &self.view_groups[if b.space == Space::World { 0 } else { 1 }], &[]);
            rp.draw(0..4, b.start..b.start + b.count);
        }
    }

    /// Filtre video : de la texture intermediaire vers la cible, bandes noires autour.
    fn post(&self, enc: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let Some(scene) = &self.scene else { return };
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rp.set_viewport(self.origin.x, self.origin.y, self.region.0 as f32, self.region.1 as f32, 0.0, 1.0);
        rp.set_pipeline(&self.post_pipe);
        rp.set_bind_group(0, &scene.group, &[]);
        rp.draw(0..3, 0..1);
    }

    fn save_capture(&self, buf: wgpu::Buffer, bpr: u32, path: PathBuf) {
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let Ok(data) = slice.get_mapped_range() else {
            log::error!("capture : lecture du tampon impossible");
            return;
        };
        let (w, h) = (self.config.width, self.config.height);
        let bgra = matches!(self.target, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut img = image::RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let i = (y * bpr + x * 4) as usize;
                let (r, g, b) = if bgra { (data[i + 2], data[i + 1], data[i]) } else { (data[i], data[i + 1], data[i + 2]) };
                img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
            }
        }
        drop(data);
        buf.unmap();
        match img.save(&path) {
            Ok(_) => log::info!("capture enregistree : {}", path.display()),
            Err(e) => log::error!("capture impossible : {e}"),
        }
    }
}
