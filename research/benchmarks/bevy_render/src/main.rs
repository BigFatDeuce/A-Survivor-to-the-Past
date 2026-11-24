//! Benchmark de rendu : N ennemis animes (vrais sprites ALttP) pilotes par sim_core,
//! rendus par Bevy (batching de sprites). Tous visibles a l'ecran (pire cas).
//! Usage : bevy_render_bench <N>   -> affiche "RESULT N fps_moy frame_ms_moy frame_ms_p99"
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use std::time::Instant;

#[derive(Component)]
struct Enemy(usize, usize); // index sim, frame de base

#[derive(Resource)]
struct Bench {
    sim: sim_core::Sim,
    start: Instant,
    last: Instant,
    frames: Vec<f64>,
    n: usize,
    tick: u32,
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: format!("bench {n}"),
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(1920, 1080),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(Bench {
            sim: sim_core::Sim::new(n, n / 2),
            start: Instant::now(),
            last: Instant::now(),
            frames: Vec::new(),
            n,
            tick: 0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (simulate, measure))
        .run();
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    bench: Res<Bench>,
) {
    // camera : 1 unite monde = 1 pixel SNES ; on dezoome pour voir toute la horde (pire cas)
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.4,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(2048.0, 2048.0, 0.0),
    ));
    let image = assets.load("enemies.png");
    let layout = layouts.add(TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 8, None, None));
    for i in 0..bench.n {
        let base = (i * 4) % 60;
        commands.spawn((
            Sprite::from_atlas_image(image.clone(), TextureAtlas { layout: layout.clone(), index: base }),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Enemy(i, base),
        ));
    }
}

fn simulate(mut bench: ResMut<Bench>, mut q: Query<(&Enemy, &mut Transform, &mut Sprite)>) {
    bench.sim.tick(1.0 / 60.0);
    bench.tick += 1;
    let anim = ((bench.tick / 8) % 4) as usize;
    let (xs, ys) = bench.sim.positions();
    q.par_iter_mut().for_each(|(e, mut t, mut s)| {
        t.translation.x = xs[e.0];
        t.translation.y = ys[e.0];
        t.translation.z = -ys[e.0] * 0.0001; // tri en profondeur facon SNES
        if let Some(a) = s.texture_atlas.as_mut() {
            a.index = e.1 + anim;
        }
    });
}

fn measure(mut bench: ResMut<Bench>, mut exit: MessageWriter<AppExit>) {
    let now = Instant::now();
    let dt = now.duration_since(bench.last).as_secs_f64() * 1000.0;
    bench.last = now;
    let el = now.duration_since(bench.start).as_secs_f64();
    if el > 3.0 {
        bench.frames.push(dt);
    }
    if el > 9.0 {
        let mut v = bench.frames.clone();
        let avg = v.iter().sum::<f64>() / v.len() as f64;
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p99 = v[(v.len() * 99) / 100];
        println!("RESULT {} {:.0} {:.3} {:.3}", bench.n, 1000.0 / avg, avg, p99);
        exit.write(AppExit::Success);
    }
}
