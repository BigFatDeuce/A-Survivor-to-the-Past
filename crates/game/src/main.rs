//! A Survivor to the Past — point d'entree.
//!
//! Options de ligne de commande :
//!   --fullscreen            plein ecran
//!   --window 1920x1080      taille de fenetre
//!   --novsync               sans synchro verticale (images non limitees)
//!   --fps-cap 240           limite d'images sans synchro
//!   --bench 3000            test de charge : 3000 ennemis maintenus, Link invincible
//!   --seconds 20            quitte apres 20 s
//!   --shot fichier.png 5    capture d'ecran apres 5 s (option repetable)
//!   --demo                  Link se deplace et choisit seul (captures, essais)
//!   --time 290              commence la partie a 4:50 (essais du boss, de fin de partie)
//!   --give bow,quiver,bow   armes et objets donnes au depart (repetes = niveaux ; chest, big_chest = coffre)
//!   --lang fr               langue pour cette session (sinon celle des reglages, anglais par defaut)
//!   --menu options          ouvre un ecran au depart (title, about, pause, options, title_options, gameover)
//!   --play                  partie directe, sans ecran titre
//!   --uicheck               verifie les placements de l'interface (toutes langues et vues) puis quitte
//!   --video crt --aspect 4:3  filtre video et format pour cette session (none, snes, crt, scanlines, gba, gb)
//!   --map arena_test  --char link  --seed 42
// pas de console derriere la fenetre en version publiee
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod content;
mod director;
mod game;
mod hud;
mod juice;
mod lang;
mod menus;
mod settings;
mod title;
mod uicheck;
mod visuals;

use std::path::PathBuf;

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn,symphonia=warn"),
    )
    .init();
    let mut cfg = engine::app::Config::default();
    let root = engine::assets::find_root();
    let set = settings::Settings::load(&root);
    cfg.fullscreen = set.fullscreen;
    cfg.video = set.video;
    cfg.icon = Some(include_bytes!("../../../assets/icons/icon_64.png"));
    cfg.aspect = set.aspect;
    let mut opts = game::Options {
        bench: None,
        demo: false,
        seed: 0x5eed,
        map: "arena_test".into(),
        character: "link".into(),
        start_time: 0.0,
        give: Vec::new(),
        menu: None,
        lang: None,
        play: false,
        uicheck: false,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let num = |s: Option<&String>| -> f32 { s.and_then(|v| v.parse().ok()).unwrap_or(0.0) };
    while i < args.len() {
        match args[i].as_str() {
            "--fullscreen" => cfg.fullscreen = true,
            "--novsync" => cfg.vsync = false,
            "--demo" => opts.demo = true,
            "--window" => {
                if let Some((w, h)) = args.get(i + 1).and_then(|s| s.split_once('x')) {
                    cfg.window_size = Some((w.parse().unwrap_or(1280), h.parse().unwrap_or(720)));
                }
                i += 1;
            }
            "--fps-cap" => {
                cfg.fps_cap = Some(num(args.get(i + 1)));
                i += 1;
            }
            "--bench" => {
                opts.bench = Some(num(args.get(i + 1)) as u32);
                i += 1;
            }
            "--seconds" => {
                cfg.exit_after = Some(num(args.get(i + 1)));
                i += 1;
            }
            "--shot" => {
                let path = PathBuf::from(args.get(i + 1).cloned().unwrap_or_else(|| "capture.png".into()));
                let at = num(args.get(i + 2));
                cfg.captures.push((path, at));
                i += 2;
            }
            "--map" => {
                opts.map = args.get(i + 1).cloned().unwrap_or(opts.map);
                i += 1;
            }
            "--char" => {
                opts.character = args.get(i + 1).cloned().unwrap_or(opts.character);
                i += 1;
            }
            "--seed" => {
                opts.seed = num(args.get(i + 1)) as u64;
                i += 1;
            }
            "--time" => {
                opts.start_time = num(args.get(i + 1));
                i += 1;
            }
            "--give" => {
                if let Some(list) = args.get(i + 1) {
                    opts.give.extend(list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
                }
                i += 1;
            }
            "--menu" => {
                opts.menu = args.get(i + 1).cloned();
                i += 1;
            }
            "--lang" => {
                opts.lang = args.get(i + 1).cloned();
                i += 1;
            }
            "--play" => opts.play = true,
            "--uicheck" => opts.uicheck = true,
            "--video" => {
                use engine::gfx::VideoFilter as V;
                cfg.video = match args.get(i + 1).map(|s| s.as_str()) {
                    Some("none") => V::None,
                    Some("crt") => V::Crt,
                    Some("scanlines") => V::Scanlines,
                    Some("gba") => V::Gba,
                    Some("gb") => V::GameBoy,
                    _ => V::Snes,
                };
                i += 1;
            }
            "--aspect" => {
                cfg.aspect = if args.get(i + 1).is_some_and(|s| s == "4:3") {
                    engine::gfx::Aspect::Classic
                } else {
                    engine::gfx::Aspect::Wide
                };
                i += 1;
            }
            other => log::warn!("option inconnue : {other}"),
        }
        i += 1;
    }
    engine::app::run(cfg, move |ctx| game::Game::new(ctx, opts));
}
