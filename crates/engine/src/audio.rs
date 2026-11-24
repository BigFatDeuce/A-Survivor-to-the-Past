//! Audio (kira) : effets en memoire avec limitation de voix, musiques en streaming bouclees.
use kira::effect::compressor::CompressorBuilder;
use kira::effect::filter::{FilterBuilder, FilterHandle, FilterMode};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
#[cfg(not(target_arch = "wasm32"))]
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
#[cfg(not(target_arch = "wasm32"))]
use kira::sound::FromFileError;
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Mix, Tween};
use crate::lofi::{LoFiBuilder, LoFiParams};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
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
    /// Boucle en fondu enchaine (a, f) : arrivee a `a` secondes, la piste repart de `loop_start`
    /// en fondu de `f` secondes pendant que l'ancienne s'eteint. Pour les longues pistes dont la
    /// fin est un fondu : aucune coupure audible, meme sans point de boucle exact.
    pub crossfade: Option<(f64, f64)>,
}

/// Une piste en cours (musique ou ambiance) et de quoi la relancer.
#[cfg(not(target_arch = "wasm32"))]
struct Voice {
    id: String,
    def: MusicDef,
    path: PathBuf,
    handle: StreamingSoundHandle<FromFileError>,
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
    #[cfg(not(target_arch = "wasm32"))]
    music: Option<Voice>,
    /// Ambiance sonore (pluie, vent), sur la piste des effets.
    #[cfg(not(target_arch = "wasm32"))]
    ambience: Option<Voice>,
    /// Navigateur : musiques et ambiances par le lecteur de la page, fondus de sortie en cours
    /// (vrai = musique), attenuation de la musique (actuelle, cible, vitesse).
    #[cfg(target_arch = "wasm32")]
    web_music: Option<crate::webmusic::Voice>,
    #[cfg(target_arch = "wasm32")]
    web_amb: Option<crate::webmusic::Voice>,
    #[cfg(target_arch = "wasm32")]
    web_fading: Vec<(crate::webmusic::FadingOut, bool)>,
    #[cfg(target_arch = "wasm32")]
    duck: (f32, f32, f32),
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
            #[cfg(not(target_arch = "wasm32"))]
            music: None,
            #[cfg(not(target_arch = "wasm32"))]
            ambience: None,
            #[cfg(target_arch = "wasm32")]
            web_music: None,
            #[cfg(target_arch = "wasm32")]
            web_amb: None,
            #[cfg(target_arch = "wasm32")]
            web_fading: Vec::new(),
            #[cfg(target_arch = "wasm32")]
            duck: (1.0, 1.0, 1.0),
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
        #[cfg(target_arch = "wasm32")]
        if !crate::webmusic::can_play_ogg() {
            log::warn!("ce navigateur ne lit pas l'Ogg Vorbis : musiques muettes");
        }
        a.load_sfx_dir(&root.join("assets/audio/sfx"));
        a.apply_volumes();
        a.set_filter(a.filter);
        a
    }

    fn load_sfx_dir(&mut self, dir: &Path) {
        let names = crate::vfs::list(dir);
        if names.is_empty() {
            log::warn!("pas d'effets sonores dans {}", dir.display());
            return;
        }
        for f in names {
            let p = dir.join(&f);
            if p.extension().is_some_and(|x| x == "ogg" || x == "wav") {
                let name = p.file_stem().unwrap().to_string_lossy().to_string();
                let Some(bytes) = crate::vfs::read(&p) else { continue };
                match StaticSoundData::from_cursor(std::io::Cursor::new(bytes)) {
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

    /// Avance l'horloge interne (temps reel, secondes), fondus et boucles des musiques.
    #[cfg(target_arch = "wasm32")]
    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64;
        let (cur, target, speed) = &mut self.duck;
        if *cur < *target {
            *cur = (*cur + *speed * dt).min(*target);
        } else if *cur > *target {
            *cur = (*cur - *speed * dt).max(*target);
        }
        let v = self.volumes;
        let gm = v.master * v.music * self.duck.0;
        let ga = v.master * v.sfx;
        if let Some(m) = self.web_music.as_mut() {
            m.update(dt, gm);
        }
        if let Some(a) = self.web_amb.as_mut() {
            a.update(dt, ga);
        }
        self.web_fading.retain_mut(|(f, music)| f.update(dt, if *music { gm } else { ga }));
    }

    #[cfg(target_arch = "wasm32")]
    fn web_start(&mut self, music: bool, id: &str, def: &MusicDef, fade: f32) {
        let slot = if music { &mut self.web_music } else { &mut self.web_amb };
        if slot.as_ref().is_some_and(|v| v.id == id) {
            return;
        }
        self.web_stop(music, fade);
        let slot = if music { &mut self.web_music } else { &mut self.web_amb };
        *slot = crate::webmusic::Voice::start(id, def, crate::webmusic::url_of(&def.file), fade);
    }

    #[cfg(target_arch = "wasm32")]
    fn web_stop(&mut self, music: bool, fade: f32) {
        let slot = if music { &mut self.web_music } else { &mut self.web_amb };
        if let Some(old) = slot.take() {
            self.web_fading.extend(old.stop(fade).into_iter().map(|f| (f, music)));
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn play_ambience(&mut self, id: &str, def: &MusicDef, _root: &Path, fade: f32) {
        self.web_start(false, id, def, fade);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn stop_ambience(&mut self, fade: f32) {
        self.web_stop(false, fade);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn play_music(&mut self, id: &str, def: &MusicDef, _root: &Path, fade: f32) {
        self.web_start(true, id, def, fade);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn stop_music(&mut self, fade: f32) {
        self.web_stop(true, fade);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn duck_music(&mut self, level: f32, time: f32) {
        self.duck.1 = level;
        self.duck.2 = (self.duck.0 - level).abs() / time.max(0.01);
    }

    /// Avance l'horloge interne (temps reel, secondes) et relance les boucles en fondu enchaine.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64;
        let (music, ambience) = (&mut self.music_track, &mut self.sfx_track);
        Self::crossfade(&mut self.music, music.as_mut());
        Self::crossfade(&mut self.ambience, ambience.as_mut());
    }

#[cfg(not(target_arch = "wasm32"))]
    fn crossfade(voice: &mut Option<Voice>, track: Option<&mut TrackHandle>) {
        let (Some(v), Some(track)) = (voice.as_mut(), track) else { return };
        let Some((at, fade)) = v.def.crossfade else { return };
        if v.handle.position() < at {
            return;
        }
        let tw = Tween { duration: Duration::from_secs_f64(fade.max(0.01)), ..Default::default() };
        match Self::open(&v.path, &v.def, fade as f32, v.def.loop_start) {
            Ok(data) => match track.play(data) {
                Ok(h) => {
                    let mut old = std::mem::replace(&mut v.handle, h);
                    old.stop(tw);
                }
                Err(e) => log::error!("{} : {e}", v.id),
            },
            Err(e) => log::error!("{} : {e}", v.id),
        }
    }

    /// Prepare une piste en streaming (volume, boucle, fondu d'entree, position de depart).
    #[cfg(not(target_arch = "wasm32"))]
    fn open(
        path: &Path,
        def: &MusicDef,
        fade: f32,
        start: f64,
    ) -> Result<StreamingSoundData<FromFileError>, FromFileError> {
        let mut data = StreamingSoundData::from_file(path)?.volume(amp_to_db(def.volume));
        if def.looping && def.crossfade.is_none() {
            data = match def.loop_end {
                Some(end) => data.loop_region(def.loop_start..end),
                None => data.loop_region(def.loop_start..),
            };
        }
        if start > 0.0 {
            data = data.start_position(start);
        }
        if fade > 0.0 {
            data = data.fade_in_tween(Tween { duration: Duration::from_secs_f32(fade), ..Default::default() });
        }
        Ok(data)
    }

#[cfg(not(target_arch = "wasm32"))]
    fn start_voice(
        slot: &mut Option<Voice>,
        track: Option<&mut TrackHandle>,
        id: &str,
        def: &MusicDef,
        path: PathBuf,
        fade: f32,
    ) {
        if slot.as_ref().is_some_and(|v| v.id == id) {
            return;
        }
        Self::stop_voice(slot, fade);
        let Some(track) = track else { return };
        match Self::open(&path, def, fade, 0.0) {
            Ok(data) => match track.play(data) {
                Ok(handle) => *slot = Some(Voice { id: id.to_string(), def: def.clone(), path, handle }),
                Err(e) => log::error!("{id} : {e}"),
            },
            Err(e) => log::error!("{} : {e}", path.display()),
        }
    }

#[cfg(not(target_arch = "wasm32"))]
    fn stop_voice(slot: &mut Option<Voice>, fade: f32) {
        if let Some(mut v) = slot.take() {
            v.handle.stop(Tween { duration: Duration::from_secs_f32(fade.max(0.01)), ..Default::default() });
        }
    }

    /// Lance une ambiance en boucle (pluie...) sur la piste des effets. Ne fait rien si elle joue deja.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn play_ambience(&mut self, id: &str, def: &MusicDef, root: &Path, fade: f32) {
        let path = root.join("assets/audio/music").join(&def.file);
        Self::start_voice(&mut self.ambience, self.sfx_track.as_mut(), id, def, path, fade);
    }

#[cfg(not(target_arch = "wasm32"))]
    pub fn stop_ambience(&mut self, fade: f32) {
        Self::stop_voice(&mut self.ambience, fade);
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn play_music(&mut self, id: &str, def: &MusicDef, root: &Path, fade: f32) {
        let path = root.join("assets/audio/music").join(&def.file);
        Self::start_voice(&mut self.music, self.music_track.as_mut(), id, def, path, fade);
    }

#[cfg(not(target_arch = "wasm32"))]
    pub fn stop_music(&mut self, fade: f32) {
        Self::stop_voice(&mut self.music, fade);
    }

    /// Volume temporaire de la musique (ex. assourdie en pause), 0..1.
    #[cfg(not(target_arch = "wasm32"))]
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
