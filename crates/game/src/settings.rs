//! Reglages du joueur, enregistres dans `settings.ron` a la racine du jeu.
use engine::audio::AudioFilter;
use engine::gfx::{Aspect, VideoFilter};
use engine::input::{Binding, Bindings};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

fn d_lang() -> String {
    "en".into()
}
fn d_master() -> f32 {
    0.8
}
fn d_music() -> f32 {
    0.7
}
fn d_sfx() -> f32 {
    0.8
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "d_lang")]
    pub lang: String,
    /// Filtre sonore old-school (actif par defaut).
    #[serde(default)]
    pub filter: AudioFilter,
    #[serde(default = "d_master")]
    pub master: f32,
    #[serde(default = "d_music")]
    pub music: f32,
    #[serde(default = "d_sfx")]
    pub sfx: f32,
    #[serde(default)]
    pub fullscreen: bool,
    /// Filtre video (SNES par defaut).
    #[serde(default)]
    pub video: VideoFilter,
    /// Format d'image : large ou 4:3.
    #[serde(default)]
    pub aspect: Aspect,
    /// Touches remplacees par le joueur (action -> touches). Vide = `data/input.ron`.
    #[serde(default)]
    pub bindings: BTreeMap<String, Vec<Binding>>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: d_lang(),
            filter: AudioFilter::default(),
            master: d_master(),
            music: d_music(),
            sfx: d_sfx(),
            fullscreen: false,
            video: VideoFilter::default(),
            aspect: Aspect::default(),
            bindings: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn load(root: &Path) -> Settings {
        let p = root.join("settings.ron");
        match std::fs::read_to_string(&p) {
            Ok(t) => ron::from_str(&t).unwrap_or_else(|e| {
                log::warn!("settings.ron illisible ({e}) : reglages par defaut");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, root: &Path) {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(3);
        match ron::ser::to_string_pretty(self, cfg) {
            Ok(t) => {
                let text = format!("// Reglages du joueur (modifiables depuis le menu Options).\n{t}\n");
                if let Err(e) = std::fs::write(root.join("settings.ron"), text) {
                    log::warn!("ecriture de settings.ron impossible : {e}");
                }
            }
            Err(e) => log::warn!("reglages : {e}"),
        }
    }

    /// Touches effectives : celles du jeu, remplacees action par action par celles du joueur.
    pub fn bindings(&self, base: &Bindings) -> Bindings {
        let mut b = base.clone();
        for (k, v) in &self.bindings {
            b.actions.insert(k.clone(), v.clone());
        }
        b
    }

    pub fn apply_audio(&self, audio: &mut engine::audio::Audio) {
        audio.volumes.master = self.master;
        audio.volumes.music = self.music;
        audio.volumes.sfx = self.sfx;
        audio.apply_volumes();
        audio.set_filter(self.filter);
    }
}
