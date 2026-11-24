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

struct Running<G> {
    gfx: Gfx,
    ctx: Ctx,
    game: G,
    list: DrawList,
    last: Instant,
    start: Instant,
    acc: f32,
    mods: ModifiersState,
    captured: usize,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run<G: Game + 'static>(cfg: Config, make: impl FnOnce(&mut Ctx) -> G + 'static) {
    let event_loop = EventLoop::new().expect("boucle d'evenements");
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
            gfx,
            ctx,
            game,
            list: DrawList::default(),
            last: now,
            start: now,
            acc: 0.0,
            mods: ModifiersState::empty(),
            captured: 0,
        });
        window.request_redraw();
    }
}

impl<G: Game> ApplicationHandler for Runner<G> {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.state.is_some() {
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
            WindowEvent::CursorMoved { position, .. } => {
                // coordonnees relatives a l'image de jeu (bandes noires du 4:3 exclues)
                s.ctx.input.mouse_moved(Vec2::new(position.x as f32, position.y as f32) - s.gfx.origin);
            }
            WindowEvent::RedrawRequested => {
                if s.frame(&self.cfg) {
                    s.game.exit(&mut s.ctx);
                    el.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        if self.state.is_none() {
            let ready = self.pending.borrow_mut().take();
            if let Some(gfx) = ready {
                self.finish(gfx);
            }
        }
        if let Some(s) = self.state.as_ref() {
            s.gfx.window.request_redraw();
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
        let dt = dt_real.min(0.1);
        let ctx = &mut self.ctx;
        ctx.real_time = (now - self.start).as_secs_f64();
        ctx.audio.update(dt);
        ctx.input.poll_gamepad();

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
        self.gfx.render(&self.list, fr.camera, fr.clear);
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
        #[cfg(not(target_arch = "wasm32"))]
        if let (false, Some(cap)) = (ctx.vsync, cfg.fps_cap) {
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
        ctx.req_quit || timeout
    }
}
