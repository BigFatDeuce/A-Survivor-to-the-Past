//! Audio (kira) : effets en memoire avec limitation de voix, musiques en streaming bouclees.
use kira::effect::compressor::CompressorBuilder;
use kira::effect::filter::{FilterBuilder, FilterHandle, FilterMode};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::FromFileError;
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Mix, Tween};
use crate::lofi::{LoFiBuilder, LoFiParams};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use std::time::Duration;

/// Reglages d'un effet sonore (limitation des sons de horde).
#[derive(Clone, Copy, Debug)]
pub struct SfxRule {
    /// Nombre maximal d'instances simultanees.
    pub max_voices: usize,
    /// Intervalle minimal entre deux declenchements (secondes).
    pub min_interval: f32,
    /// Volume de base (0..1).
    pub volume: f32,
    /// Variation aleatoire de hauteur (+/- ratio).
    pub pitch_jitter: f32,
}

impl Default for SfxRule {
    fn default() -> Self {
        SfxRule { max_voices: 4, min_interval: 0.03, volume: 1.0, pitch_jitter: 0.0 }
    }
}

struct Sfx {
    data: StaticSoundData,
    rule: SfxRule,
    voices: Vec<StaticSoundHandle>,
    last: f64,
}

/// Musique : fichier + boucle (secondes).
#[derive(Clone, Debug)]
pub struct MusicDef {
    pub file: String,
    pub loop_start: f64,
    pub loop_end: Option<f64>,
    /// Faux = jingle joue une seule fois.
    pub looping: bool,
    pub volume: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes { master: 0.8, music: 0.7, sfx: 0.8 }
    }
}

/// Filtre « old-school » : couleur sonore d'une console.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum AudioFilter {
    Off,
    /// Aigus legerement adoucis.
    Soft,
    /// Console 16 bits (SNES) : aigus feutres.
    #[default]
    Retro,
    /// Console portable 32 bits : echantillonnage reduit, un peu de grain.
    Gba,
    /// Premiere console portable : mono, grain marque, petit haut-parleur.
    GameBoy,
}

impl AudioFilter {
    pub const ALL: [AudioFilter; 5] =
        [AudioFilter::Off, AudioFilter::Soft, AudioFilter::Retro, AudioFilter::Gba, AudioFilter::GameBoy];
    /// Frequence de coupure du passe-bas (Hz), None = desactive.
    fn cutoff(self) -> Option<f64> {
        match self {
            AudioFilter::Off => None,
            AudioFilter::Soft => Some(9500.0),
            AudioFilter::Retro => Some(6200.0),
            AudioFilter::Gba => Some(7000.0),
            AudioFilter::GameBoy => Some(4200.0),
        }
    }
    /// (frequence simulee, bits, mono, passe-haut) de l'effet lo-fi.
    fn lofi(self) -> (f32, f32, bool, f32) {
        match self {
            AudioFilter::Gba => (18157.0, 9.0, false, 0.0),
            AudioFilter::GameBoy => (11025.0, 6.0, true, 160.0),
            _ => (0.0, 0.0, false, 0.0),
        }
    }
}

pub struct Audio {
    manager: Option<AudioManager>,
    filters: Vec<FilterHandle>,
    lofi: Arc<LoFiParams>,
    pub filter: AudioFilter,
    music_track: Option<TrackHandle>,
    sfx_track: Option<TrackHandle>,
    sfx: HashMap<String, Sfx>,
    music: Option<(String, StreamingSoundHandle<FromFileError>)>,
    clock: f64,
    rng: u32,
    pub volumes: Volumes,
}

fn amp_to_db(a: f32) -> Decibels {
    if a <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * a.log10())
    }
}

impl Audio {
    /// Initialise la sortie audio ; en cas d'echec le jeu continue en silence.
    pub fn new(root: &Path) -> Audio {
        let mut a = Audio {
            manager: None,
            filters: Vec::new(),
            lofi: Arc::new(LoFiParams::default()),
            filter: AudioFilter::default(),
            music_track: None,
            sfx_track: None,
            sfx: HashMap::new(),
            music: None,
            clock: 0.0,
            rng: 0x9e37_79b9,
            volumes: Volumes::default(),
        };
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(mut m) => {
                let lp = || FilterBuilder::new().mode(FilterMode::LowPass).cutoff(20000.0).resonance(0.05);
                let mut music = TrackBuilder::new();
                music.add_effect(LoFiBuilder(a.lofi.clone()));
                a.filters.push(music.add_effect(lp()));
                a.music_track = m.add_sub_track(music).ok();
                // effets : compresseur (les hordes empilent les sons) puis passe-bas
                let mut sfx = TrackBuilder::new().sound_capacity(256);
                sfx.add_effect(
                    CompressorBuilder::new()
                        .threshold(-16.0)
                        .ratio(3.5)
                        .attack_duration(Duration::from_millis(4))
                        .release_duration(Duration::from_millis(90)),
                );
                sfx.add_effect(LoFiBuilder(a.lofi.clone()));
                a.filters.push(sfx.add_effect(lp()));
                a.sfx_track = m.add_sub_track(sfx).ok();
                a.manager = Some(m);
            }
            Err(e) => log::error!("audio indisponible : {e}"),
        }
        a.load_sfx_dir(&root.join("assets/audio/sfx"));
        a.apply_volumes();
        a.set_filter(a.filter);
        a
    }

    fn load_sfx_dir(&mut self, dir: &Path) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            log::warn!("pas d'effets sonores dans {}", dir.display());
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "ogg" || x == "wav") {
                let name = p.file_stem().unwrap().to_string_lossy().to_string();
                match StaticSoundData::from_file(&p) {
                    Ok(data) => {
                        self.sfx.insert(name, Sfx { data, rule: SfxRule::default(), voices: Vec::new(), last: -1.0 });
                    }
                    Err(e) => log::warn!("{}: {e}", p.display()),
                }
            }
        }
        log::info!("{} effets sonores charges", self.sfx.len());
    }

    pub fn set_rule(&mut self, name: &str, rule: SfxRule) {
        if let Some(s) = self.sfx.get_mut(name) {
            s.rule = rule;
        } else {
            log::warn!("effet sonore inconnu : {name}");
        }
    }

    pub fn has_sfx(&self, name: &str) -> bool {
        self.sfx.contains_key(name)
    }

    /// Avance l'horloge interne (temps reel, secondes).
    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64;
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    /// Joue un effet. `pitch` multiplie la vitesse de lecture, `gain` le volume.
    pub fn play(&mut self, name: &str, gain: f32, pitch: f32) {
        let r = self.rand() * 2.0 - 1.0;
        let clock = self.clock;
        let (Some(track), Some(s)) = (self.sfx_track.as_mut(), self.sfx.get_mut(name)) else { return };
        if clock - s.last < s.rule.min_interval as f64 {
            return;
        }
        s.voices.retain(|h| h.state() != kira::sound::PlaybackState::Stopped);
        if s.voices.len() >= s.rule.max_voices {
            // vole la voix la plus ancienne
            let mut old = s.voices.remove(0);
            old.stop(Tween { duration: Duration::from_millis(15), ..Default::default() });
        }
        s.last = clock;
        let rate = (pitch * (1.0 + r * s.rule.pitch_jitter)).max(0.05) as f64;
        let data = s.data.volume(amp_to_db(gain * s.rule.volume)).playback_rate(rate);
        if let Ok(h) = track.play(data) {
            s.voices.push(h);
        }
    }

    /// Lance une musique (fondu enchaine). Ne fait rien si elle joue deja.
    pub fn play_music(&mut self, id: &str, def: &MusicDef, root: &Path, fade: f32) {
        if self.music.as_ref().is_some_and(|(cur, _)| cur == id) {
            return;
        }
        self.stop_music(fade);
        let Some(track) = self.music_track.as_mut() else { return };
        let path = root.join("assets/audio/music").join(&def.file);
        match StreamingSoundData::from_file(&path) {
            Ok(data) => {
                let mut data = data.volume(amp_to_db(def.volume));
                if def.looping {
                    data = match def.loop_end {
                        Some(end) => data.loop_region(def.loop_start..end),
                        None => data.loop_region(def.loop_start..),
                    };
                }
                if fade > 0.0 {
                    data = data.fade_in_tween(Tween { duration: Duration::from_secs_f32(fade), ..Default::default() });
                }
                match track.play(data) {
                    Ok(h) => self.music = Some((id.to_string(), h)),
                    Err(e) => log::error!("musique {id} : {e}"),
                }
            }
            Err(e) => log::error!("musique {} : {e}", path.display()),
        }
    }

    pub fn stop_music(&mut self, fade: f32) {
        if let Some((_, mut h)) = self.music.take() {
            h.stop(Tween { duration: Duration::from_secs_f32(fade.max(0.01)), ..Default::default() });
        }
    }

    /// Volume temporaire de la musique (ex. assourdie en pause), 0..1.
    pub fn duck_music(&mut self, level: f32, time: f32) {
        let v = self.volumes.master * self.volumes.music * level;
        if let Some(t) = self.music_track.as_mut() {
            t.set_volume(amp_to_db(v), Tween { duration: Duration::from_secs_f32(time.max(0.01)), ..Default::default() });
        }
    }

    /// Change le filtre old-school (transition douce).
    pub fn set_filter(&mut self, f: AudioFilter) {
        self.filter = f;
        let tw = Tween { duration: Duration::from_millis(120), ..Default::default() };
        let (hold, bits, mono, hp) = f.lofi();
        self.lofi.set(hold, bits, mono, hp);
        for h in &mut self.filters {
            match f.cutoff() {
                Some(c) => {
                    h.set_cutoff(c, tw);
                    h.set_mix(Mix::WET, tw);
                }
                None => {
                    h.set_cutoff(20000.0, tw);
                    h.set_mix(Mix::DRY, tw);
                }
            }
        }
    }

    pub fn apply_volumes(&mut self) {
        let v = self.volumes;
        if let Some(t) = self.music_track.as_mut() {
            t.set_volume(amp_to_db(v.master * v.music), Tween::default());
        }
        if let Some(t) = self.sfx_track.as_mut() {
            t.set_volume(amp_to_db(v.master * v.sfx), Tween::default());
        }
    }
}
