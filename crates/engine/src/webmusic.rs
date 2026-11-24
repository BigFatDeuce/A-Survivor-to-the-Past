//! Musiques et ambiances dans le navigateur : lecteur audio de la page (decodage et streaming
//! natifs, aucun thread). Le moteur gere les fondus, les boucles et les fondus enchaines.
use crate::audio::MusicDef;
use wasm_bindgen::JsCast;
use web_sys::HtmlAudioElement;

struct Player {
    el: HtmlAudioElement,
    /// Volume propre (fondu) 0..1, cible et vitesse (par seconde).
    vol: f32,
    target: f32,
    speed: f32,
}

impl Player {
    fn new(url: &str, start: f64) -> Option<Player> {
        let el = HtmlAudioElement::new_with_src(url).ok()?;
        el.set_preload("auto");
        if start > 0.0 {
            el.set_current_time(start);
        }
        el.set_volume(0.0);
        // refus possible (pas encore d'interaction) : on reessaie a chaque image
        let _ = el.play();
        Some(Player { el, vol: 0.0, target: 1.0, speed: 1.0 })
    }

    fn fade_to(&mut self, target: f32, time: f32) {
        self.target = target;
        self.speed = (self.target - self.vol).abs() / time.max(0.01);
    }

    fn stop(&self) {
        let _ = self.el.pause();
        self.el.set_src("");
    }
}

/// Une piste (musique ou ambiance) : lecteur courant et lecteurs qui s'eteignent.
pub struct Voice {
    pub id: String,
    def: MusicDef,
    url: String,
    cur: Player,
    fading: Vec<Player>,
}

impl Voice {
    pub fn start(id: &str, def: &MusicDef, url: String, fade: f32) -> Option<Voice> {
        let mut cur = Player::new(&url, 0.0)?;
        if fade > 0.0 {
            cur.fade_to(1.0, fade);
        } else {
            cur.vol = 1.0;
        }
        // boucle simple sur toute la piste : le lecteur s'en charge
        cur.el.set_loop(def.looping && def.crossfade.is_none() && def.loop_start <= 0.0 && def.loop_end.is_none());
        Some(Voice { id: id.to_string(), def: def.clone(), url, cur, fading: Vec::new() })
    }

    pub fn stop(mut self, fade: f32) -> Vec<FadingOut> {
        self.cur.fade_to(0.0, fade);
        self.fading.push(self.cur);
        let v = self.def.volume;
        self.fading.into_iter().map(|p| FadingOut(p, v)).collect()
    }

    /// Fondus, boucles, fondus enchaines ; `gain` = volume du groupe (maitre x musique x attenuation).
    pub fn update(&mut self, dt: f32, gain: f32) {
        let d = &self.def;
        let t = self.cur.el.current_time();
        if d.looping {
            if let Some((at, fade)) = d.crossfade {
                if t >= at {
                    if let Some(mut n) = Player::new(&self.url, d.loop_start) {
                        n.fade_to(1.0, fade as f32);
                        let mut old = std::mem::replace(&mut self.cur, n);
                        old.fade_to(0.0, fade as f32);
                        self.fading.push(old);
                    }
                }
            } else if d.loop_start > 0.0 || d.loop_end.is_some() {
                let dur = self.cur.el.duration();
                let end = d.loop_end.unwrap_or(if dur.is_finite() { dur - 0.03 } else { f64::MAX });
                if t >= end || self.cur.el.ended() {
                    self.cur.el.set_current_time(d.loop_start);
                    let _ = self.cur.el.play();
                }
            }
        }
        if self.cur.el.paused() && !self.cur.el.ended() {
            let _ = self.cur.el.play();
        }
        step(&mut self.cur, dt, gain * d.volume);
        let base = gain * d.volume;
        self.fading.retain_mut(|p| {
            step(p, dt, base);
            if p.vol <= 0.0 {
                p.stop();
                false
            } else {
                true
            }
        });
    }
}

fn step(p: &mut Player, dt: f32, gain: f32) {
    if p.vol < p.target {
        p.vol = (p.vol + p.speed * dt).min(p.target);
    } else if p.vol > p.target {
        p.vol = (p.vol - p.speed * dt).max(p.target);
    }
    p.el.set_volume((p.vol * gain).clamp(0.0, 1.0) as f64);
}

/// Lecteur arrete qui finit son fondu de sortie.
pub struct FadingOut(Player, f32);

impl FadingOut {
    /// Faux quand le fondu est fini (lecteur libere).
    pub fn update(&mut self, dt: f32, gain: f32) -> bool {
        step(&mut self.0, dt, gain * self.1);
        if self.0.vol <= 0.0 {
            self.0.stop();
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
