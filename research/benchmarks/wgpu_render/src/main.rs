//! Benchmark de rendu "moteur sur mesure" : sim_core (SoA) + 1 seul draw call instancie wgpu.
//! Rendu hors-ecran 1920x1080, chaque frame attend la fin du GPU (meme methode que le test WebGL "sync").
use std::time::Instant;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Inst {
    pos: [f32; 2],
    frame: f32,
    depth: f32,
}

const SHADER: &str = r#"
struct Cam { center: vec2<f32>, half: vec2<f32> };
@group(0) @binding(0) var<uniform> cam: Cam;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs(@builtin(vertex_index) vi: u32, @location(0) p: vec2<f32>, @location(1) frame: f32, @location(2) depth: f32) -> VOut {
    var corners = array<vec2<f32>, 4>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(1.0, 1.0));
    let c = corners[vi];
    let cell = vec2<f32>(f32(u32(frame) % 8u), f32(u32(frame) / 8u));
    var o: VOut;
    o.uv = (cell + vec2(c.x, 1.0 - c.y)) / 8.0;
    let w = p + (c - 0.5) * 32.0;
    o.pos = vec4<f32>((w - cam.center) / cam.half, depth, 1.0);
    return o;
}
@fragment
fn fs(i: VOut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, smp, i.uv);
    if (c.a < 0.5) { discard; }
    return c;
}
"#;

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .expect("adapter");
    println!("GPU: {} ({:?})", adapter.get_info().name, adapter.get_info().backend);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("device");

    // atlas
    let img = image::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../bevy_render/assets/enemies.png"))
        .unwrap()
        .to_rgba8();
    let (tw, th) = img.dimensions();
    let size = wgpu::Extent3d { width: tw, height: th, depth_or_array_layers: 1 };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        tex.as_image_copy(),
        &img,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * tw), rows_per_image: Some(th) },
        size,
    );
    let tex_view = tex.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default()); // nearest par defaut

    let (w, h) = (1920u32, 1080u32);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());

    let cam: [f32; 4] = [2048.0, 2048.0, 960.0 * 1.4, 540.0 * 1.4];
    let cam_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&cam_buf, 0, bytemuck::cast_slice(&cam));

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Inst>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32, 2 => Float32],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
        }),
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: cam_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
    });

    println!("{:>8} {:>10} {:>10} {:>8}", "ennemis", "ms/frame", "p99 ms", "fps");
    for &n in &[5_000usize, 10_000, 25_000, 50_000, 100_000, 200_000] {
        let mut sim = sim_core::Sim::new(n, n / 2);
        let inst_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (n * std::mem::size_of::<Inst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut data = vec![Inst { pos: [0.0; 2], frame: 0.0, depth: 0.0 }; n];
        let mut ms = Vec::new();
        for tick in 0..420u32 {
            let t0 = Instant::now();
            sim.tick(1.0 / 60.0);
            let anim = ((tick / 8) % 4) as usize;
            let (xs, ys) = sim.positions();
            for i in 0..n {
                data[i] = Inst {
                    pos: [xs[i], ys[i]],
                    frame: (((i * 4) % 60) + anim) as f32,
                    depth: 1.0 - ys[i] / 8192.0,
                };
            }
            queue.write_buffer(&inst_buf, 0, bytemuck::cast_slice(&data));
            let mut enc = device.create_command_encoder(&Default::default());
            {
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.1, b: 0.05, a: 1.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rp.set_pipeline(&pipeline);
                rp.set_bind_group(0, &bind, &[]);
                rp.set_vertex_buffer(0, inst_buf.slice(..));
                rp.draw(0..4, 0..n as u32);
            }
            queue.submit([enc.finish()]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            if tick >= 120 {
                ms.push(t0.elapsed().as_secs_f64() * 1000.0);
            }
        }
        let avg = ms.iter().sum::<f64>() / ms.len() as f64;
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("{:>8} {:>10.3} {:>10.3} {:>8.0}", n, avg, ms[ms.len() * 99 / 100], 1000.0 / avg);
    }
}
