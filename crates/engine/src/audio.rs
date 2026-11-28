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

/// Musique : fichier + boucle (secondes, au sample pres). Comme le pilote son de la SNES :
/// l'introduction est jouee une fois, puis la piste repart de `loop_start` en arrivant a
/// `loop_end`, sans coupure (fichiers prepares par research/tools/music_loopify.py).
#[derive(Clone, Debug)]
pub struct MusicDef {
    pub file: String,
    pub loop_start: f64,
    pub loop_end: Option<f64>,
    /// Faux = jingle joue une seule fois.
    pub looping: bool,
    pub volume: f32,
}

/// Enchainement de deux musiques, facon SNES : l'ancienne s'eteint en fondu, la nouvelle part du
/// debut quand l'extinction est presque finie (part de la duree du fondu), avec un fondu
/// d'entree tres court (pas de claquement).
const SEQUENCE_AT: f32 = 0.8;
const SEQUENCE_FADE_IN: f32 = 0.15;
/// Extrait du juke-box : extinction de la musique en cours, puis depart differe et long fondu d'entree.
const PREVIEW_OUT: f32 = 0.6;
const PREVIEW_IN: (f32, f32) = (0.5, 1.5);

/// Une piste en cours (musique ou ambiance).
#[cfg(not(target_arch = "wasm32"))]
struct Voice {
    id: String,
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
    /// Valeurs revues en 0.6.1 (retour : les filtres s'entendaient a peine) : la musique de la
    /// SNES n'a presque rien au-dessus de 6 kHz, les coupures sont donc bien plus basses.
    fn cutoff(self) -> Option<f64> {
        match self {
            AudioFilter::Off => None,
            // aigus adoucis, son plus rond
            AudioFilter::Soft => Some(4200.0),
            // televiseur cathodique : etouffe, chaleureux
            AudioFilter::Retro => Some(2600.0),
            // haut-parleur de console portable 32 bits : grain, peu de graves
            AudioFilter::Gba => Some(7500.0),
            // premiere console portable : tout petit haut-parleur, mono, tres granuleux
            AudioFilter::GameBoy => Some(3000.0),
        }
    }
    /// (frequence simulee, bits, mono, passe-haut) de l'effet lo-fi.
    fn lofi(self) -> (f32, f32, bool, f32) {
        match self {
            AudioFilter::Retro => (0.0, 0.0, false, 90.0),
            AudioFilter::Gba => (13379.0, 7.0, false, 220.0),
            AudioFilter::GameBoy => (8192.0, 5.0, true, 420.0),
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
    /// Enregistrement : effets joues (temps, fichier, volume, vitesse), pour remixer le son a part.
    pub log: Option<Vec<(f64, String, f32, f32)>>,
}

/// (delai, fondu d'entree) d'une nouvelle piste. Musique qui en remplace une autre : facon SNES,
/// elle part quand l'ancienne a presque fini de s'eteindre. Ambiance : fondu enchaine.
fn transition(music: bool, replacing: bool, fade: f32) -> (f32, f32) {
    if !music {
        (0.0, fade)
    } else if replacing && fade > 0.0 {
        (fade * SEQUENCE_AT, SEQUENCE_FADE_IN)
    } else {
        (0.0, fade.min(SEQUENCE_FADE_IN))
    }
}

fn amp_to_db(a: f32) -> Decibels {
    if a <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * a.log10())
    }
}

/// Ordre d'essai d'une sortie audio de secours (identifiant cpal, ex. « alsa:plughw:CARD=0,DEV=0 ») :
/// sorties systeme d'abord, puis la carte avec conversion de format ; jamais « null » (jette le son).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn fallback_rank(id: &str) -> Option<u8> {
    let id = id.to_ascii_lowercase();
    let pcm = id.split_once(':').map_or(id.as_str(), |(_, p)| p);
    if pcm.starts_with("null") {
        return None;
    }
    Some(match pcm {
        p if p.starts_with("default") || p.starts_with("sysdefault") => 0,
        p if p.starts_with("pipewire") || p.starts_with("pulse") => 1,
        p if p.starts_with("plughw") => 2,
        p if p.starts_with("dmix") => 3,
        p if p.starts_with("hw:") => 5,
        _ => 4,
    })
}

/// Sortie audio par defaut ; si elle refuse (consoles Linux : peripherique « default » d'ALSA absent ou
/// occupe), les autres sorties sont essayees dans l'ordre de `fallback_rank`. La sortie retenue est
/// notee dans le journal.
fn open_manager() -> Result<AudioManager, String> {
    let first = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default());
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(e) = &first {
        use kira::backend::cpal::{cpal, cpal::traits::DeviceTrait, cpal::traits::HostTrait, CpalBackendSettings};
        log::warn!("sortie audio par defaut : {e}");
        let mut devices: Vec<(u8, String, _)> = cpal::default_host()
            .output_devices()
            .into_iter()
            .flatten()
            .filter_map(|d| {
                let id = d.id().map(|i| i.to_string()).unwrap_or_else(|_| d.to_string());
                fallback_rank(&id).map(|r| (r, id, d))
            })
            .collect();
        devices.sort_by_key(|d| d.0);
        for (_, name, d) in devices {
            let settings = AudioManagerSettings {
                backend_settings: CpalBackendSettings { device: Some(d), config: None },
                ..AudioManagerSettings::default()
            };
            match AudioManager::<DefaultBackend>::new(settings) {
                Ok(m) => {
                    log::info!("sortie audio : {name}");
                    return Ok(m);
                }
                Err(e) => log::warn!("sortie audio {name} : {e}"),
            }
        }
    }
    first.map_err(|e| e.to_string())
}

impl Audio {
    /// Appli en arriere-plan (Android) : musique, ambiance et effets en pause, repris au retour. Le son
    /// tourne sur son propre fil : sans cela, il continue quand l'appli est reduite.
    pub fn set_paused(&mut self, paused: bool) {
        let tween = Tween { duration: Duration::from_millis(80), ..Default::default() };
        for t in [&mut self.music_track, &mut self.sfx_track].into_iter().flatten() {
            if paused {
                t.pause(tween);
            } else {
                t.resume(tween);
            }
        }
        log::info!("son : {}", if paused { "en pause (arriere-plan)" } else { "repris" });
    }

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
            log: None,
        };
        match open_manager() {
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

    /// Musique : enchainement facon SNES ; ambiance : fondu enchaine.
    #[cfg(target_arch = "wasm32")]
    fn web_start(&mut self, music: bool, id: &str, def: &MusicDef, fade: f32, timing: Option<(f32, f32)>) {
        let slot = if music { &mut self.web_music } else { &mut self.web_amb };
        if slot.as_ref().is_some_and(|v| v.id == id) {
            return;
        }
        let (wait, fade_in) = timing.unwrap_or_else(|| transition(music, slot.is_some(), fade));
        self.web_stop(music, fade);
        let slot = if music { &mut self.web_music } else { &mut self.web_amb };
        *slot = Some(crate::webmusic::Voice::start(id, def, crate::webmusic::url_of(&def.file), wait, fade_in));
    }

    #[cfg(target_arch = "wasm32")]
    fn web_stop(&mut self, music: bool, fade: f32) {
        let slot = if music { &mut self.web_music } else { &mut self.web_amb };
        if let Some(old) = slot.take() {
            self.web_fading.push((old.stop(fade), music));
        }
    }

    /// Prepare une piste a l'avance (navigateur : telechargement et decodage), pour qu'elle
    /// demarre a l'heure (musiques des boss).
    #[cfg(target_arch = "wasm32")]
    pub fn prefetch(&mut self, def: &MusicDef) {
        crate::webmusic::prefetch(&crate::webmusic::url_of(&def.file));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn prefetch(&mut self, _def: &MusicDef) {}

    #[cfg(target_arch = "wasm32")]
    pub fn play_ambience(&mut self, id: &str, def: &MusicDef, _root: &Path, fade: f32) {
        self.web_start(false, id, def, fade, None);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn stop_ambience(&mut self, fade: f32) {
        self.web_stop(false, fade);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn play_music(&mut self, id: &str, def: &MusicDef, _root: &Path, fade: f32) {
        self.web_start(true, id, def, fade, None);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn preview_music(&mut self, id: &str, def: &MusicDef, _root: &Path) {
        self.web_start(true, id, def, PREVIEW_OUT, Some(PREVIEW_IN));
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

    /// Avance l'horloge interne (temps reel, secondes) ; les boucles sont gerees par kira.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn update(&mut self, dt: f32) {
        self.clock += dt as f64;
    }

    /// Prepare une piste en streaming (volume, boucle au sample pres, depart differe, fondu d'entree).
    #[cfg(not(target_arch = "wasm32"))]
    fn open(path: &Path, def: &MusicDef, delay: f32, fade_in: f32) -> Result<StreamingSoundData<FromFileError>, FromFileError> {
        let mut data = StreamingSoundData::from_file(path)?.volume(amp_to_db(def.volume));
        if def.looping {
            data = match def.loop_end {
                Some(end) => data.loop_region(def.loop_start..end),
                None => data.loop_region(def.loop_start..),
            };
        }
        if delay > 0.0 {
            data = data.start_time(kira::StartTime::Delayed(Duration::from_secs_f32(delay)));
        }
        if fade_in > 0.0 {
            data = data.fade_in_tween(Tween { duration: Duration::from_secs_f32(fade_in), ..Default::default() });
        }
        Ok(data)
    }

    /// `music` : enchainement facon SNES (l'ancienne piste s'eteint, puis la nouvelle part) ;
    /// sinon fondu enchaine (ambiances). `timing` : depart differe et fondu d'entree imposes.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    fn start_voice(
        slot: &mut Option<Voice>,
        track: Option<&mut TrackHandle>,
        id: &str,
        def: &MusicDef,
        path: PathBuf,
        fade: f32,
        music: bool,
        timing: Option<(f32, f32)>,
    ) {
        if slot.as_ref().is_some_and(|v| v.id == id) {
            return;
        }
        let (delay, fade_in) = timing.unwrap_or_else(|| transition(music, slot.is_some(), fade));
        Self::stop_voice(slot, fade);
        let Some(track) = track else { return };
        match Self::open(&path, def, delay, fade_in) {
            Ok(data) => match track.play(data) {
                Ok(handle) => *slot = Some(Voice { id: id.to_string(), handle }),
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
        Self::start_voice(&mut self.ambience, self.sfx_track.as_mut(), id, def, path, fade, false, None);
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
        if let Some(log) = self.log.as_mut() {
            log.push((clock, name.to_string(), gain * s.rule.volume, rate as f32));
        }
        let data = s.data.volume(amp_to_db(gain * s.rule.volume)).playback_rate(rate);
        if let Ok(h) = track.play(data) {
            s.voices.push(h);
        }
    }

    /// Lance une musique : la precedente s'eteint en `fade` secondes, puis celle-ci part du debut
    /// (comme dans le jeu d'origine). Ne fait rien si elle joue deja.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn play_music(&mut self, id: &str, def: &MusicDef, root: &Path, fade: f32) {
        let path = root.join("assets/audio/music").join(&def.file);
        Self::start_voice(&mut self.music, self.music_track.as_mut(), id, def, path, fade, true, None);
    }

    /// Extrait d'une musique (juke-box) : la musique en cours s'eteint, puis celle-ci entre en long
    /// fondu. Pour la couper, relancer une musique (`play_music`) : elle s'eteint en fondu de sortie.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn preview_music(&mut self, id: &str, def: &MusicDef, root: &Path) {
        let path = root.join("assets/audio/music").join(&def.file);
        Self::start_voice(&mut self.music, self.music_track.as_mut(), id, def, path, PREVIEW_OUT, true, Some(PREVIEW_IN));
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
        // navigateur : les musiques passent par le lecteur de la page et son propre graphe
        #[cfg(target_arch = "wasm32")]
        crate::webmusic::set_filter(f.cutoff(), hp, bits, mono);
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

#[cfg(test)]
mod tests {
    #[test]
    fn fallback_order_skips_null() {
        use super::fallback_rank as r;
        assert_eq!(r("alsa:null"), None);
        assert!(r("alsa:sysdefault:CARD=0") < r("alsa:plughw:CARD=0,DEV=0"));
        assert!(r("alsa:plughw:CARD=0,DEV=0") < r("alsa:hw:CARD=0,DEV=0"));
    }
}
