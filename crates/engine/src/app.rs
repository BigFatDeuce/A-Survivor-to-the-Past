//! Boucle principale : fenetre winit, pas fixe 60 Hz + interpolation, plein ecran, captures.
use crate::assets::{find_root, Atlas};
use crate::audio::Audio;
use crate::gfx::{Aspect, DrawList, Gfx, VideoFilter};
use crate::input::{Input, MouseButton};
use crate::TICK_DT;
use glam::Vec2;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use web_time::Duration;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Fullscreen, Icon, Window, WindowId};

/// Icone de fenetre depuis un PNG.
fn window_icon(png: &[u8]) -> Option<Icon> {
    let img = image::load_from_memory(png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).ok()
}

#[derive(Clone, Debug)]
pub struct Config {
    pub title: String,
    pub vsync: bool,
    pub fullscreen: bool,
    /// Taille de fenetre imposee (pixels), sinon un multiple de 640x360 adapte a l'ecran.
    pub window_size: Option<(u32, u32)>,
    /// Limite d'images par seconde quand la synchro verticale est coupee.
    pub fps_cap: Option<f32>,
    /// Captures d'ecran automatiques : (fichier, apres N secondes).
    pub captures: Vec<(PathBuf, f32)>,
    /// Quitte apres N secondes (mesures, captures).
    pub exit_after: Option<f32>,
    pub video: VideoFilter,
    pub aspect: Aspect,
    /// Icone de la fenetre (PNG).
    pub icon: Option<&'static [u8]>,
    /// Enregistrement video (bande-annonce) : pas fixe de 1/60 s quel que soit le temps reel, chaque
    /// image envoyee a ffmpeg dans ce fichier, effets sonores notes dans `<fichier>.sfx.tsv`, son coupe,
    /// clavier / souris / manette du joueur ignores (seul `script` agit).
    pub record: Option<PathBuf>,
    /// Touches jouees pendant l'enregistrement : (temps, touche, duree d'appui ; 0 = appui bref).
    pub script: Vec<(f32, KeyCode, f32)>,
    /// Enregistrement : les images d'avant ce temps (secondes) sont jouees mais pas gardees.
    pub record_from: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            title: "A Survivor to the Past".into(),
            vsync: true,
            fullscreen: false,
            window_size: None,
            fps_cap: None,
            captures: Vec::new(),
            exit_after: None,
            video: VideoFilter::default(),
            aspect: Aspect::default(),
            icon: None,
            record: None,
            script: Vec::new(),
            record_from: 0.0,
        }
    }
}

/// Mesures de performance (moyennes glissantes, millisecondes).
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub fps: f32,
    pub frame_ms: f32,
    pub tick_ms: f32,
    pub draw_ms: f32,
    pub render_ms: f32,
    pub instances: usize,
    pub ticks_total: u64,
    pub frames_total: u64,
}

/// Contexte partage avec le jeu.
pub struct Ctx {
    pub root: PathBuf,
    pub atlas: Atlas,
    pub audio: Audio,
    pub input: Input,
    /// Taille de la vue en pixels logiques (hauteur ~360, largeur selon l'ecran).
    pub view: Vec2,
    /// Facteur d'agrandissement entier courant.
    pub scale: u32,
    pub stats: Stats,
    /// Facteur de temps de la simulation (0 = figee, 1 = normale) : arrets sur image, ralentis.
    pub time_scale: f32,
    /// Temps reel ecoule depuis le lancement (s).
    pub real_time: f64,
    pub vsync: bool,
    pub fullscreen: bool,
    pub gpu: String,
    pub video: VideoFilter,
    pub aspect: Aspect,
    /// Mode performances (reglage du jeu) : 0 = non, 1 = rendu a la resolution du jeu, 2 = idem a 30 images/s.
    pub perf: u8,
    req_quit: bool,
    req_video: Option<(VideoFilter, Aspect)>,
    req_fullscreen: Option<bool>,
    req_vsync: Option<bool>,
    req_capture: Option<PathBuf>,
}

impl Ctx {
    pub fn quit(&mut self) {
        self.req_quit = true;
    }
    pub fn set_fullscreen(&mut self, on: bool) {
        self.req_fullscreen = Some(on);
    }
    pub fn set_vsync(&mut self, on: bool) {
        self.req_vsync = Some(on);
    }
    /// Filtre video et format d'image (appliques a l'image suivante).
    pub fn set_video(&mut self, filter: VideoFilter, aspect: Aspect) {
        self.req_video = Some((filter, aspect));
    }
    pub fn capture(&mut self, path: PathBuf) {
        self.req_capture = Some(path);
    }
    /// Position de la souris en coordonnees d'interface (pixels logiques).
    pub fn mouse_ui(&self) -> Vec2 {
        self.input.mouse_px / self.scale as f32
    }
}

/// Ce que le jeu demande au rendu pour cette image.
pub struct Frame {
    /// Centre de la camera dans le monde (pixels logiques, valeurs fractionnaires permises).
    pub camera: Vec2,
    /// Agrandissement du monde en plus de l'echelle entiere (1 = vue la plus large ; l'interface ne change pas).
    pub zoom: f32,
    pub clear: [f32; 3],
}

pub trait Game {
    /// Mise a jour par image (temps reel) : menus, effets visuels, minuteries d'arret sur image.
    fn frame(&mut self, ctx: &mut Ctx, dt: f32);
    /// Pas de simulation fixe (1/60 s, multiplie par `ctx.time_scale`).
    fn tick(&mut self, ctx: &mut Ctx);
    /// Construit la liste de dessin ; `alpha` = fraction entre les deux derniers pas.
    fn draw(&mut self, ctx: &Ctx, alpha: f32, list: &mut DrawList) -> Frame;
    /// Appele a la fermeture (sauvegarde, bilan).
    fn exit(&mut self, _ctx: &mut Ctx) {}
    /// Android : appli passee en arriere-plan (le systeme peut la fermer ensuite sans prevenir).
    fn suspend(&mut self, _ctx: &mut Ctx) {}
}

type Maker<G> = Box<dyn FnOnce(&mut Ctx) -> G>;

struct Runner<G: Game> {
    cfg: Config,
    make: Option<Maker<G>>,
    state: Option<Running<G>>,
    /// Navigateur : rendu en cours d'initialisation (asynchrone), recupere des qu'il est pret.
    #[cfg(target_arch = "wasm32")]
    pending: std::rc::Rc<std::cell::RefCell<Option<Gfx>>>,
    #[cfg(target_arch = "wasm32")]
    started: bool,
}

#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

/// Android : application donnee par `android_main`, avant `run`.
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();

struct Running<G> {
    /// Android : appli en arriere-plan (plus de surface d'affichage) ; rien n'est dessine.
    suspended: bool,
    gfx: Gfx,
    ctx: Ctx,
    game: G,
    list: DrawList,
    last: Instant,
    start: Instant,
    acc: f32,
    mods: ModifiersState,
    captured: usize,
    rec: Option<Recorder>,
}

/// Enregistrement en cours : ffmpeg (images brutes sur son entree), images ecrites, touches du script.
struct Recorder {
    child: std::process::Child,
    path: PathBuf,
    frames: u64,
    /// Debut de la video (secondes de jeu) : avant, rien n'est encode.
    from: f32,
    /// (temps, touche, appui) tries par temps, et le prochain a jouer.
    keys: Vec<(f32, KeyCode, bool)>,
    next_key: usize,
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
impl Recorder {
    fn start(path: &std::path::Path, script: &[(f32, KeyCode, f32)], from: f32, w: u32, h: u32, bgra: bool) -> Recorder {
        use std::process::{Command, Stdio};
        let size = format!("{w}x{h}");
        // sans perte (RGB, x264 qp 0) : le montage recompresse une seule fois a la fin
        let child = Command::new("ffmpeg")
            .args(["-loglevel", "error", "-y", "-f", "rawvideo", "-pix_fmt", if bgra { "bgra" } else { "rgba" }, "-s", &size, "-r", "60", "-i", "-"])
            .args(["-c:v", "libx264rgb", "-qp", "0", "-preset", "veryfast"])
            .arg(path)
            .stdin(Stdio::piped())
            .spawn()
            .expect("enregistrement : ffmpeg introuvable");
        let mut keys: Vec<(f32, KeyCode, bool)> = Vec::new();
        for &(t, k, hold) in script {
            keys.push((t, k, true));
            keys.push((t + if hold > 0.0 { hold } else { 0.07 }, k, false));
        }
        keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        log::info!("enregistrement : {} ({size}, 60 i/s), {} touches", path.display(), script.len());
        Recorder { child, path: path.to_path_buf(), frames: 0, from, keys, next_key: 0 }
    }

    /// L'image en cours fait partie de la video.
    fn keeps(&self) -> bool {
        self.frames as f32 * TICK_DT >= self.from - 1e-4
    }

    fn write(&mut self, rgba: &[u8]) {
        use std::io::Write;
        if !self.keeps() {
            self.frames += 1;
            return;
        }
        if let Some(stdin) = self.child.stdin.as_mut() {
            if stdin.write_all(rgba).is_err() {
                log::error!("enregistrement : ffmpeg a refuse une image");
            }
        }
        self.frames += 1;
    }

    fn finish(mut self, audio: &mut Audio) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
        let mut tsv = String::from("time\tname\tgain\trate\n");
        for (t, n, g, r) in audio.log.take().unwrap_or_default() {
            let t = t - self.from as f64;
            if t < 0.0 {
                continue;
            }
            tsv.push_str(&format!("{t:.4}\t{n}\t{g:.4}\t{r:.4}\n"));
        }
        let p = self.path.with_extension("sfx.tsv");
        if let Err(e) = std::fs::write(&p, tsv) {
            log::error!("enregistrement : {} : {e}", p.display());
        }
        log::info!("enregistrement termine : {} images -> {}", self.frames, self.path.display());
    }
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
impl Recorder {
    fn start(_: &std::path::Path, _: &[(f32, KeyCode, f32)], _: f32, _: u32, _: u32, _: bool) -> Recorder {
        unimplemented!("enregistrement : bureau seulement")
    }
    fn keeps(&self) -> bool {
        false
    }
    fn write(&mut self, _: &[u8]) {}
    fn finish(self, _: &mut Audio) {}
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<G: Game + 'static>(cfg: Config, make: impl FnOnce(&mut Ctx) -> G + 'static) {
    #[cfg(not(target_os = "android"))]
    let event_loop = EventLoop::new().expect("boucle d'evenements");
    #[cfg(target_os = "android")]
    let event_loop = {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        let app = ANDROID_APP.get().expect("android_main : ANDROID_APP").clone();
        EventLoop::builder().with_android_app(app).build().expect("boucle d'evenements")
    };
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut r = Runner { cfg, make: Some(Box::new(make)), state: None };
    event_loop.run_app(&mut r).expect("execution");
}

/// Navigateur : la boucle rend la main a la page ; le jeu demarre quand le rendu est pret.
#[cfg(target_arch = "wasm32")]
pub fn run<G: Game + 'static>(cfg: Config, make: impl FnOnce(&mut Ctx) -> G + 'static) {
    use winit::platform::web::EventLoopExtWebSys;
    let event_loop = EventLoop::new().expect("boucle d'evenements");
    event_loop.set_control_flow(ControlFlow::Poll);
    let r = Runner { cfg, make: Some(Box::new(make)), state: None, pending: Default::default(), started: false };
    event_loop.spawn_app(r);
}

/// Canevas de la page (`<canvas id="game">`).
#[cfg(target_arch = "wasm32")]
fn page_canvas() -> Option<web_sys::HtmlCanvasElement> {
    use wasm_bindgen::JsCast;
    web_sys::window()?.document()?.get_element_by_id("game")?.dyn_into().ok()
}

impl<G: Game> Runner<G> {
    /// Fin du lancement : atlas, audio, jeu.
    fn finish(&mut self, mut gfx: Gfx) {
        gfx.set_video(self.cfg.video, self.cfg.aspect);
        // curseur du systeme cache : le jeu dessine le sien (sprite)
        gfx.window.set_cursor_visible(false);
        let root = find_root();
        log::info!("racine du jeu : {}", root.display());
        let (atlas, img) = Atlas::load(&root);
        gfx.set_atlas(&img, img.width(), img.height());
        let audio = Audio::new(&root);
        let mut ctx = Ctx {
            root,
            atlas,
            audio,
            input: Input::new(),
            view: gfx.view,
            scale: gfx.scale,
            stats: Stats::default(),
            time_scale: 1.0,
            real_time: 0.0,
            vsync: self.cfg.vsync,
            fullscreen: self.cfg.fullscreen,
            gpu: gfx.adapter_name.clone(),
            video: self.cfg.video,
            aspect: self.cfg.aspect,
            perf: 0,
            req_quit: false,
            req_video: None,
            req_fullscreen: None,
            req_vsync: None,
            req_capture: None,
        };
        let game = (self.make.take().unwrap())(&mut ctx);
        let now = Instant::now();
        let window = gfx.window.clone();
        self.state = Some(Running {
            suspended: false,
            gfx,
            ctx,
            game,
            list: DrawList::default(),
            last: now,
            start: now,
            acc: 0.0,
            mods: ModifiersState::empty(),
            captured: 0,
            rec: None,
        });
        window.request_redraw();
    }
}

impl<G: Game> ApplicationHandler for Runner<G> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        // Android : retour au premier plan, nouvelle surface d'affichage
        if let Some(s) = self.state.as_mut() {
            if s.suspended {
                s.suspended = false;
                s.ctx.audio.set_paused(false);
                s.gfx.recreate_surface();
                s.ctx.view = s.gfx.view;
                s.ctx.scale = s.gfx.scale;
                s.last = Instant::now();
                s.gfx.window.request_redraw();
            }
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            if self.started {
                return;
            }
            self.started = true;
            use winit::platform::web::WindowAttributesExtWebSys;
            // le canevas suit la taille que lui donne la page (CSS)
            let attrs = Window::default_attributes()
                .with_title(self.cfg.title.clone())
                .with_canvas(page_canvas())
                .with_prevent_default(true)
                .with_focusable(true);
            let window = Arc::new(el.create_window(attrs).expect("fenetre"));
            let pending = self.pending.clone();
            let vsync = self.cfg.vsync;
            wasm_bindgen_futures::spawn_local(async move {
                let gfx = Gfx::new_async(window.clone(), vsync).await;
                *pending.borrow_mut() = Some(gfx);
                window.request_redraw();
            });
            return;
        }
        #[allow(unreachable_code)]
        let size = self.cfg.window_size.unwrap_or_else(|| {
            let mh = el.primary_monitor().map(|m| m.size().height).unwrap_or(1080);
            let k = ((mh as f32 * 0.8) / 360.0).floor().max(1.0) as u32;
            (640 * k, 360 * k)
        });
        let mut attrs = Window::default_attributes()
            .with_title(self.cfg.title.clone())
            .with_inner_size(PhysicalSize::new(size.0, size.1))
            .with_min_inner_size(PhysicalSize::new(640, 360))
            .with_window_icon(self.cfg.icon.and_then(window_icon));
        if self.cfg.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = Arc::new(el.create_window(attrs).expect("fenetre"));
        #[cfg(not(target_arch = "wasm32"))]
        self.finish(Gfx::new(window, self.cfg.vsync));
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        #[cfg(target_arch = "wasm32")]
        if self.state.is_none() {
            let ready = self.pending.borrow_mut().take();
            if let Some(gfx) = ready {
                self.finish(gfx);
            }
        }
        let Some(s) = self.state.as_mut() else { return };
        if self.cfg.record.is_some()
            && matches!(
                event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::CursorMoved { .. }
                    | WindowEvent::CursorEntered { .. }
                    | WindowEvent::CursorLeft { .. }
                    | WindowEvent::Touch(_)
                    | WindowEvent::Focused(_)
            )
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                s.game.exit(&mut s.ctx);
                el.exit();
            }
            WindowEvent::Resized(sz) => {
                s.gfx.resize(sz.width, sz.height);
                s.ctx.view = s.gfx.view;
                s.ctx.scale = s.gfx.scale;
            }
            WindowEvent::Focused(f) => {
                if !f {
                    s.ctx.input.focus_lost();
                } else {
                    s.ctx.input.focused = true;
                }
            }
            WindowEvent::ModifiersChanged(m) => s.mods = m.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    // Android : touche Retour = Echap (pause, retour des menus)
                    let code = if code == KeyCode::BrowserBack { KeyCode::Escape } else { code };
                    let down = event.state == ElementState::Pressed;
                    // raccourcis globaux : F11 ou Alt+Entree = plein ecran
                    if down
                        && !event.repeat
                        && (code == KeyCode::F11 || (code == KeyCode::Enter && s.mods.alt_key()))
                    {
                        let on = !s.ctx.fullscreen;
                        s.ctx.req_fullscreen = Some(on);
                        return;
                    }
                    if let (true, winit::keyboard::Key::Character(c)) = (down, &event.logical_key) {
                        s.ctx.input.learn_label(code, c);
                    }
                    if down {
                        if let Some(t) = &event.text {
                            s.ctx.input.type_text(t);
                        }
                    }
                    s.ctx.input.key_event(code, down, event.repeat);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    _ => return,
                };
                s.ctx.input.mouse_event(b, state == ElementState::Pressed);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // un cran de molette = 1 ; pave tactile : environ 40 pixels par cran
                let n = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                s.ctx.input.wheel_event(n);
            }
            WindowEvent::Touch(t) => {
                use crate::input::TouchPhase as P;
                let phase = match t.phase {
                    winit::event::TouchPhase::Started => P::Start,
                    winit::event::TouchPhase::Moved => P::Move,
                    winit::event::TouchPhase::Ended => P::End,
                    winit::event::TouchPhase::Cancelled => P::Cancel,
                };
                let p = Vec2::new(t.location.x as f32, t.location.y as f32) - s.gfx.origin;
                // une ligne de liste des menus fait environ 22 pixels d'interface
                s.ctx.input.touch_event(t.id, phase, p, 22.0 * s.gfx.scale as f32);
            }
            WindowEvent::CursorLeft { .. } => s.ctx.input.mouse_inside = false,
            WindowEvent::CursorEntered { .. } => s.ctx.input.mouse_inside = true,
            WindowEvent::CursorMoved { position, .. } => {
                s.ctx.input.mouse_inside = true;
                // coordonnees relatives a l'image de jeu (bandes noires du 4:3 exclues)
                s.ctx.input.mouse_moved(Vec2::new(position.x as f32, position.y as f32) - s.gfx.origin);
            }
            // Android en arriere-plan : plus de surface, rien a dessiner
            WindowEvent::RedrawRequested if s.suspended => {}
            // une image entiere est calculee ici : pas dans une garde de `match` (lisibilite)
            #[allow(clippy::collapsible_match)]
            WindowEvent::RedrawRequested if self.cfg.record.is_some() => {}
            WindowEvent::RedrawRequested => {
                if s.frame(&self.cfg) {
                    s.game.exit(&mut s.ctx);
                    el.exit();
                }
            }
            _ => {}
        }
    }

    fn suspended(&mut self, _el: &ActiveEventLoop) {
        if let Some(s) = self.state.as_mut() {
            s.suspended = true;
            s.ctx.input.focus_lost();
            s.game.suspend(&mut s.ctx);
            s.ctx.audio.set_paused(true);
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        if self.state.is_none() {
            let ready = self.pending.borrow_mut().take();
            if let Some(gfx) = ready {
                self.finish(gfx);
            }
        }
        if let Some(s) = self.state.as_ref() {
            // en arriere-plan (Android) : attendre le retour au lieu de tourner a vide (batterie)
            el.set_control_flow(if s.suspended { ControlFlow::Wait } else { ControlFlow::Poll });
            if !s.suspended {
                s.gfx.window.request_redraw();
            }
        }
        // tournage : les images ne dependent pas des messages de la fenetre (ecran eteint, verrouille)
        if self.cfg.record.is_some() && !el.exiting() {
            if let Some(s) = self.state.as_mut() {
                if s.frame(&self.cfg) {
                    s.game.exit(&mut s.ctx);
                    el.exit();
                }
            }
        }
    }
}

/// Moyenne glissante exponentielle.
fn ema(old: f32, new: f32) -> f32 {
    if old == 0.0 {
        new
    } else {
        old + (new - old) * 0.05
    }
}

impl<G: Game> Running<G> {
    /// Une image complete. Renvoie `true` s'il faut quitter.
    fn frame(&mut self, cfg: &Config) -> bool {
        let now = Instant::now();
        let dt_real = (now - self.last).as_secs_f32();
        self.last = now;
        let mut dt = dt_real.min(0.1);
        if self.rec.is_none() {
            if let Some(path) = &cfg.record {
                let bgra = self.gfx.target_is_bgra();
                let (w, h) = self.gfx.size();
                self.rec = Some(Recorder::start(path, &cfg.script, cfg.record_from, w, h, bgra));
                self.ctx.audio.log = Some(Vec::new());
                self.ctx.audio.volumes.master = 0.0;
                self.ctx.audio.apply_volumes();
                self.ctx.input.focused = true;
                self.gfx.offscreen = true;
            }
        }
        let ctx = &mut self.ctx;
        if let Some(rec) = self.rec.as_mut() {
            // temps virtuel : une image = 1/60 s, meme si l'encodage est plus lent
            dt = TICK_DT;
            ctx.real_time = rec.frames as f64 * TICK_DT as f64;
            while let Some(&(t, k, down)) = rec.keys.get(rec.next_key) {
                if t > ctx.real_time as f32 {
                    break;
                }
                ctx.input.key_event(k, down, false);
                rec.next_key += 1;
            }
            // le jeu peut remettre le volume (reglages) : on le recoupe
            if ctx.audio.volumes.master > 0.0 {
                ctx.audio.volumes.master = 0.0;
                ctx.audio.apply_volumes();
            }
        } else {
            ctx.real_time = (now - self.start).as_secs_f64();
        }
        ctx.audio.update(dt);
        if self.rec.is_none() {
            ctx.input.poll_gamepad();
        }

        // 1) mise a jour par image (menus, effets)
        ctx.input.set_phase_tick(false);
        self.game.frame(ctx, dt);

        // 2) pas fixes
        let t0 = Instant::now();
        self.acc += dt * ctx.time_scale.max(0.0);
        let mut n = 0;
        ctx.input.set_phase_tick(true);
        while self.acc >= TICK_DT {
            self.game.tick(ctx);
            ctx.input.consume_tick();
            self.acc -= TICK_DT;
            n += 1;
            ctx.stats.ticks_total += 1;
            if n >= 6 {
                // spirale de la mort evitee : on lache le retard
                self.acc = self.acc.min(TICK_DT);
                break;
            }
        }
        ctx.input.set_phase_tick(false);
        let t1 = Instant::now();
        if n > 0 {
            ctx.stats.tick_ms = ema(ctx.stats.tick_ms, (t1 - t0).as_secs_f32() * 1000.0 / n as f32);
        }

        // 3) dessin
        let alpha = (self.acc / TICK_DT).clamp(0.0, 1.0);
        self.list.clear();
        let fr = self.game.draw(ctx, alpha, &mut self.list);
        let t2 = Instant::now();
        if let Some(p) = ctx.req_capture.take() {
            self.gfx.request_capture(p);
        }
        if let Some((path, after)) = cfg.captures.get(self.captured) {
            if ctx.real_time as f32 >= *after {
                self.gfx.request_capture(path.clone());
                self.captured += 1;
            }
        }
        if self.rec.as_ref().is_some_and(Recorder::keeps) {
            self.gfx.request_raw();
        }
        self.gfx.lowres = ctx.perf > 0;
        self.gfx.render(&self.list, fr.camera, fr.zoom, fr.clear);
        if let Some(rec) = self.rec.as_mut() {
            let img = self.gfx.raw_frame.take().unwrap_or_default();
            rec.write(&img);
        }
        let t3 = Instant::now();
        ctx.input.end_frame();

        let st = &mut ctx.stats;
        st.draw_ms = ema(st.draw_ms, (t2 - t1).as_secs_f32() * 1000.0);
        st.render_ms = ema(st.render_ms, (t3 - t2).as_secs_f32() * 1000.0);
        st.frame_ms = ema(st.frame_ms, dt_real * 1000.0);
        st.fps = if st.frame_ms > 0.0 { 1000.0 / st.frame_ms } else { 0.0 };
        st.instances = self.list.instances.len();
        st.frames_total += 1;

        // 4) requetes du jeu
        if let Some(on) = ctx.req_fullscreen.take() {
            ctx.fullscreen = on;
            self.gfx.window.set_fullscreen(if on { Some(Fullscreen::Borderless(None)) } else { None });
        }
        if let Some((f, a)) = ctx.req_video.take() {
            ctx.video = f;
            ctx.aspect = a;
            self.gfx.set_video(f, a);
            ctx.view = self.gfx.view;
            ctx.scale = self.gfx.scale;
        }
        if let Some(on) = ctx.req_vsync.take() {
            ctx.vsync = on;
            self.gfx.set_vsync(on);
        }

        // 5) limiteur d'images (sans synchro verticale)
        // mode performances : 30 images/s (synchro verticale : on se reveille avant la 2e trame).
        // shortcut: navigateur sans limiteur (requestAnimationFrame), le mode n'y reduit que la resolution
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(cap) = match (ctx.perf == 2 && self.rec.is_none(), ctx.vsync) {
            (true, true) => Some(1.0 / 0.0293),
            (true, false) => Some(30.0),
            (false, false) => cfg.fps_cap,
            (false, true) => None,
        } {
            let target = Duration::from_secs_f32(1.0 / cap.max(10.0));
            let spent = now.elapsed();
            if spent < target {
                let rest = target - spent;
                if rest > Duration::from_millis(2) {
                    std::thread::sleep(rest - Duration::from_millis(1));
                }
                while now.elapsed() < target {
                    std::hint::spin_loop();
                }
            }
        }

        let timeout = cfg.exit_after.is_some_and(|t| ctx.real_time as f32 >= t);
        let quit = ctx.req_quit || timeout;
        if quit {
            if let Some(rec) = self.rec.take() {
                rec.finish(&mut self.ctx.audio);
            }
        }
        quit
    }
}
