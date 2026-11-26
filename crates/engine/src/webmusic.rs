//! Musiques et ambiances dans le navigateur, par Web Audio : chaque piste est telechargee et
//! decodee une fois (AudioBuffer), puis jouee par une source qui boucle au sample pres
//! (loopStart / loopEnd), comme le pilote son de la SNES. Si le navigateur ne sait pas decoder le
//! fichier, repli sur le lecteur audio de la page (boucle moins precise).
use crate::audio::MusicDef;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::ArrayBuffer;
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioScheduledSourceNode, AudioContextState, BiquadFilterNode, BiquadFilterType,
    ChannelCountMode, ChannelInterpretation, GainNode, HtmlAudioElement, MediaElementAudioSourceNode, Response,
    WaveShaperNode,
};

/// Filtres « old-school » des musiques du navigateur : un petit graphe Web Audio (passe-haut,
/// passe-bas, reduction de resolution, mono), comme le moteur natif.
struct Graph {
    ctx: AudioContext,
    input: GainNode,
    hp: BiquadFilterNode,
    lp: BiquadFilterNode,
    crush: WaveShaperNode,
    out: GainNode,
}

/// Etat du telechargement / decodage d'une piste.
enum Load {
    Pending,
    Ready(AudioBuffer),
    Failed,
}

type Slot = Rc<RefCell<Load>>;

/// Pistes decodees gardees en memoire (les plus recentes) : musique du stage, des boss, du menu.
const CACHE: usize = 6;

thread_local! {
    static GRAPH: RefCell<Option<Graph>> = const { RefCell::new(None) };
    static WANTED: RefCell<(Option<f64>, f32, f32, bool)> = const { RefCell::new((None, 0.0, 0.0, false)) };
    static BUFFERS: RefCell<Vec<(String, Slot)>> = const { RefCell::new(Vec::new()) };
}

fn build_graph() -> Option<Graph> {
    let ctx = AudioContext::new().ok()?;
    let input = ctx.create_gain().ok()?;
    let hp = ctx.create_biquad_filter().ok()?;
    hp.set_type(BiquadFilterType::Highpass);
    let lp = ctx.create_biquad_filter().ok()?;
    lp.set_type(BiquadFilterType::Lowpass);
    lp.q().set_value(0.5);
    let crush = ctx.create_wave_shaper().ok()?;
    let out = ctx.create_gain().ok()?;
    input.connect_with_audio_node(&hp).ok()?;
    hp.connect_with_audio_node(&lp).ok()?;
    lp.connect_with_audio_node(&crush).ok()?;
    crush.connect_with_audio_node(&out).ok()?;
    out.connect_with_audio_node(&ctx.destination()).ok()?;
    let g = Graph { ctx, input, hp, lp, crush, out };
    WANTED.with(|w| apply(&g, *w.borrow()));
    Some(g)
}

/// Contexte audio et entree du graphe (crees au premier besoin).
fn graph() -> Option<(AudioContext, GainNode)> {
    GRAPH.with(|g| {
        let mut g = g.borrow_mut();
        if g.is_none() {
            *g = build_graph();
        }
        g.as_ref().map(|g| (g.ctx.clone(), g.input.clone()))
    })
}

fn apply(g: &Graph, (cutoff, highpass, bits, mono): (Option<f64>, f32, f32, bool)) {
    g.lp.frequency().set_value(cutoff.unwrap_or(20000.0) as f32);
    g.hp.frequency().set_value(highpass.max(10.0));
    if bits > 0.0 {
        // escalier de 2^bits niveaux : resolution reduite (grain des consoles portables)
        let q = 2f32.powf(bits - 1.0);
        let mut curve: Vec<f32> = (0..4096).map(|i| ((i as f32 / 2047.5 - 1.0) * q).round() / q).collect();
        g.crush.set_curve_opt_f32_slice(Some(&mut curve[..]));
    } else {
        g.crush.set_curve_opt_f32_slice(None);
    }
    if mono {
        g.out.set_channel_count(1);
        g.out.set_channel_count_mode(ChannelCountMode::Explicit);
        g.out.set_channel_interpretation(ChannelInterpretation::Speakers);
    } else {
        g.out.set_channel_count(2);
        g.out.set_channel_count_mode(ChannelCountMode::Max);
    }
}

/// Reglages du filtre (passe-bas en Hz ou None, passe-haut en Hz, bits ou 0, mono).
pub fn set_filter(cutoff: Option<f64>, highpass: f32, bits: f32, mono: bool) {
    WANTED.with(|w| *w.borrow_mut() = (cutoff, highpass, bits, mono));
    GRAPH.with(|g| {
        if let Some(g) = g.borrow().as_ref() {
            apply(g, (cutoff, highpass, bits, mono));
        }
    });
}

/// Le contexte audio demarre suspendu tant que le joueur n'a pas interagi : on le relance.
fn wake() {
    GRAPH.with(|g| {
        if let Some(g) = g.borrow().as_ref() {
            if g.ctx.state() == AudioContextState::Suspended {
                let _ = g.ctx.resume();
            }
        }
    });
}

/// Telecharge et decode une piste a l'avance (une seule fois).
pub fn prefetch(url: &str) {
    load(url);
}

/// Telecharge et decode une piste (une seule fois) ; les suivantes reutilisent le tampon.
fn load(url: &str) -> Slot {
    BUFFERS.with(|b| {
        let mut b = b.borrow_mut();
        if let Some(i) = b.iter().position(|(u, _)| u == url) {
            let e = b.remove(i);
            let s = e.1.clone();
            b.push(e);
            return s;
        }
        let slot: Slot = Rc::new(RefCell::new(Load::Pending));
        b.push((url.to_string(), slot.clone()));
        if b.len() > CACHE {
            b.remove(0);
        }
        let (u, s) = (url.to_string(), slot.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let res = decode(&u).await;
            if res.is_none() {
                log::warn!("{u} : decodage Web Audio impossible, lecteur de la page a la place");
            }
            *s.borrow_mut() = res.map(Load::Ready).unwrap_or(Load::Failed);
        });
        slot
    })
}

async fn decode(url: &str) -> Option<AudioBuffer> {
    let win = web_sys::window()?;
    let resp: Response = JsFuture::from(win.fetch_with_str(url)).await.ok()?.dyn_into().ok()?;
    if !resp.ok() {
        return None;
    }
    let data: ArrayBuffer = JsFuture::from(resp.array_buffer().ok()?).await.ok()?.dyn_into().ok()?;
    let (ctx, _) = graph()?;
    let buf = JsFuture::from(ctx.decode_audio_data(&data).ok()?).await.ok()?;
    buf.dyn_into().ok()
}

/// Repli : lecteur audio de la page.
struct Element {
    el: HtmlAudioElement,
    src: Option<MediaElementAudioSourceNode>,
}

impl Element {
    fn new(url: &str) -> Option<Element> {
        let el = HtmlAudioElement::new_with_src(url).ok()?;
        el.set_preload("auto");
        el.set_volume(0.0);
        let src = graph().and_then(|(ctx, input)| {
            let s = ctx.create_media_element_source(&el).ok()?;
            s.connect_with_audio_node(&input).ok()?;
            Some(s)
        });
        let _ = el.play();
        Some(Element { el, src })
    }
}

/// Sortie d'une piste en cours de lecture.
enum Out {
    Buf { src: AudioBufferSourceNode, gain: GainNode },
    El(Element),
}

/// Une piste (musique ou ambiance) : attente eventuelle (enchainement facon SNES : la piste
/// precedente s'eteint d'abord), puis lecture en boucle, avec son propre volume de fondu.
pub struct Voice {
    pub id: String,
    def: MusicDef,
    url: String,
    load: Slot,
    out: Option<Out>,
    /// Secondes avant le debut de la lecture.
    wait: f32,
    /// Duree du fondu d'entree.
    fade_in: f32,
    vol: f32,
    target: f32,
    speed: f32,
}

impl Voice {
    /// `wait` : delai avant de commencer ; `fade_in` : fondu d'entree (0 = plein volume).
    pub fn start(id: &str, def: &MusicDef, url: String, wait: f32, fade_in: f32) -> Voice {
        let load = load(&url);
        Voice { id: id.to_string(), def: def.clone(), url, load, out: None, wait, fade_in, vol: 0.0, target: 1.0, speed: 0.0 }
    }

    fn begin(&mut self) {
        let out = match &*self.load.borrow() {
            Load::Pending => return,
            Load::Ready(buf) => graph().and_then(|(ctx, input)| {
                let src = ctx.create_buffer_source().ok()?;
                src.set_buffer(Some(buf));
                if self.def.looping {
                    src.set_loop(true);
                    src.set_loop_start(self.def.loop_start);
                    src.set_loop_end(self.def.loop_end.unwrap_or(buf.duration()));
                }
                let gain = ctx.create_gain().ok()?;
                gain.gain().set_value(0.0);
                src.connect_with_audio_node(&gain).ok()?;
                gain.connect_with_audio_node(&input).ok()?;
                src.start_with_when_and_grain_offset(0.0, 0.0).ok()?;
                Some(Out::Buf { src, gain })
            }),
            Load::Failed => Element::new(&self.url).map(Out::El),
        };
        let Some(out) = out else {
            // aucune sortie possible : on n'insiste pas
            self.out = None;
            self.wait = f32::MAX;
            return;
        };
        self.out = Some(out);
        if self.fade_in > 0.0 {
            self.vol = 0.0;
            self.speed = 1.0 / self.fade_in;
        } else {
            self.vol = 1.0;
        }
    }

    fn fade_to(&mut self, target: f32, time: f32) {
        self.target = target;
        self.speed = (self.target - self.vol).abs() / time.max(0.01);
    }

    /// Lance l'extinction (fondu de sortie).
    pub fn stop(mut self, fade: f32) -> FadingOut {
        self.fade_to(0.0, fade);
        if self.out.is_none() {
            self.vol = 0.0;
        }
        FadingOut(self)
    }

    /// Attente, fondus, boucles du repli ; `gain` = volume du groupe (maitre x musique x attenuation).
    pub fn update(&mut self, dt: f32, gain: f32) {
        wake();
        if self.out.is_none() {
            self.wait -= dt;
            if self.wait <= 0.0 {
                self.begin();
            }
            return;
        }
        if self.vol < self.target {
            self.vol = (self.vol + self.speed * dt).min(self.target);
        } else if self.vol > self.target {
            self.vol = (self.vol - self.speed * dt).max(self.target);
        }
        let v = (self.vol * gain * self.def.volume).clamp(0.0, 1.0);
        match self.out.as_mut() {
            Some(Out::Buf { gain, .. }) => gain.gain().set_value(v),
            Some(Out::El(e)) => {
                let d = &self.def;
                if d.looping {
                    let t = e.el.current_time();
                    let end = d.loop_end.unwrap_or_else(|| {
                        let dur = e.el.duration();
                        if dur.is_finite() { dur - 0.03 } else { f64::MAX }
                    });
                    if t >= end || e.el.ended() {
                        e.el.set_current_time(d.loop_start);
                        let _ = e.el.play();
                    }
                }
                if e.el.paused() && !e.el.ended() {
                    let _ = e.el.play();
                }
                e.el.set_volume(v as f64);
            }
            None => {}
        }
    }

    fn release(&mut self) {
        match self.out.take() {
            Some(Out::Buf { src, gain }) => {
                let sched: &AudioScheduledSourceNode = &src;
                let _ = sched.stop_with_when(0.0);
                let _ = src.disconnect();
                let _ = gain.disconnect();
            }
            Some(Out::El(e)) => {
                let _ = e.el.pause();
                e.el.set_src("");
                if let Some(s) = &e.src {
                    let _ = s.disconnect();
                }
            }
            None => {}
        }
    }
}

impl Drop for Voice {
    fn drop(&mut self) {
        self.release();
    }
}

/// Piste arretee qui finit son fondu de sortie.
pub struct FadingOut(Voice);

impl FadingOut {
    /// Faux quand le fondu est fini (piste liberee).
    pub fn update(&mut self, dt: f32, gain: f32) -> bool {
        self.0.wait = f32::MAX;
        self.0.update(dt, gain);
        if self.0.vol <= 0.0 {
            self.0.release();
            false
        } else {
            true
        }
    }
}

/// Adresse d'un fichier de musique, relative a la page du jeu.
pub fn url_of(file: &str) -> String {
    format!("assets/audio/music/{file}")
}

/// Le lecteur audio de la page sait-il lire ce format (Ogg Vorbis) ?
pub fn can_play_ogg() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.create_element("audio").ok())
        .and_then(|e| e.dyn_into::<HtmlAudioElement>().ok())
        .map(|a| !a.can_play_type("audio/ogg; codecs=vorbis").is_empty())
        .unwrap_or(false)
}
