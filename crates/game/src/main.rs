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
//!   --update-source dir     mises a jour lues dans un dossier (latest.json + archive) : essais
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
mod update;
mod visuals;

use std::path::{Path, PathBuf};

/// Boite de message systeme (erreurs graves : le jeu n'a pas de console).
#[cfg(windows)]
fn message_box(text: &str) {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    let wide = |s: &str| std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let (t, c) = (wide(text), wide("A Survivor to the Past"));
    // MB_ICONERROR
    unsafe { MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), 0x10) };
}

#[cfg(not(windows))]
fn message_box(text: &str) {
    eprintln!("{text}");
}

/// En cas de plantage : `crash.log` a la racine du jeu et message au joueur.
fn crash_report(root: &Path) {
    let root = root.to_path_buf();
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        let msg = format!("A Survivor to the Past v{}
{info}
", env!("CARGO_PKG_VERSION"));
        let _ = std::fs::write(root.join("crash.log"), &msg);
        message_box(&format!(
            "The game crashed. Details: crash.log
Le jeu a planté. Détails : crash.log

{info}

Discord: About / À propos"
        ));
    }));
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn,symphonia=warn"),
    )
    .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    // installateur de mise a jour (lance par l'ancienne version, depuis `.update/new/`)
    if args.first().map(String::as_str) == Some("--apply-update") {
        if let Some(root) = args.get(1) {
            update::apply(Path::new(root));
        }
        return;
    }
    let mut cfg = engine::app::Config::default();
    let root = engine::assets::find_root();
    if !root.join("data").is_dir() || !root.join("assets/gen/atlas.ron").is_file() {
        message_box(
            "Game files not found: extract the whole zip archive into a folder, then launch the game from that folder.

             Fichiers du jeu introuvables : extrayez toute l'archive zip dans un dossier, puis lancez le jeu depuis ce dossier.",
        );
        return;
    }
    crash_report(&root);
    // mise a jour telechargee lors d'une session precedente : installation avant tout
    if update::launch_pending(&root) {
        return;
    }
    update::cleanup(&root);
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
        update_source: None,
    };
    let mut i = 0;
    let num = |s: Option<&String>| -> f32 { s.and_then(|v| v.parse().ok()).unwrap_or(0.0) };
    while i < args.len() {
        match args[i].as_str() {
            "--fullscreen" => cfg.fullscreen = true,
            "--novsync" => cfg.vsync = false,
            "--demo" => opts.demo = true,
            "--update-source" => {
                opts.update_source = args.get(i + 1).map(PathBuf::from);
                i += 1;
            }
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
