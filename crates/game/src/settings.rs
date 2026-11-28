//! Reglages du joueur, enregistres dans `settings.ron` a la racine du jeu.
use engine::audio::AudioFilter;
use engine::gfx::{Aspect, VideoFilter};
use crate::update::UpdateMode;
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

/// Version du format des reglages (migrations a la lecture).
const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// 0 = reglages ecrits avant la v0.3 (filtre SNES impose par defaut).
    #[serde(default)]
    pub version: u32,
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
    /// Filtre video (aucun par defaut ; l'ecran titre n'en a jamais).
    #[serde(default)]
    pub video: VideoFilter,
    /// Format d'image : large ou 4:3.
    #[serde(default)]
    pub aspect: Aspect,
    /// Touches remplacees par le joueur (action -> touches). Vide = `data/input.ron`.
    #[serde(default)]
    pub bindings: BTreeMap<String, Vec<Binding>>,
    /// Mises a jour : automatiques (defaut), sur demande, ou desactivees.
    #[serde(default)]
    pub updates: UpdateMode,
    /// Zoom en partie : 0 = vue la plus large, 1 et 2 = plus pres (`game::ZOOMS`).
    #[serde(default)]
    pub zoom: u8,
    /// Mode performances : 0 = non, 1 = rendu a la resolution du jeu (60 images/s), 2 = idem a 30 images/s.
    #[serde(default)]
    pub perf: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: VERSION,
            lang: d_lang(),
            filter: AudioFilter::default(),
            master: d_master(),
            music: d_music(),
            sfx: d_sfx(),
            fullscreen: false,
            video: VideoFilter::default(),
            aspect: Aspect::default(),
            bindings: BTreeMap::new(),
            updates: UpdateMode::default(),
            zoom: 0,
            perf: 0,
        }
    }
}

impl Settings {
    pub fn load(root: &Path) -> Settings {
        let mut s = match crate::storage::read(root, "settings.ron") {
            Some(t) => ron::from_str(&t).unwrap_or_else(|e| {
                // copie de secours (touches, options) : le prochain enregistrement l'ecrasera
                log::warn!("settings.ron illisible ({e}) : reglages par defaut, copie dans settings.broken.ron");
                let _ = crate::storage::write(root, "settings.broken.ron", &t);
                Settings::default()
            }),
            None => Settings::default(),
        };
        if s.version < 1 {
            // le filtre SNES etait impose par defaut : on passe au rendu net, sans filtre
            if s.video == VideoFilter::Snes {
                s.video = VideoFilter::None;
            }
            s.version = VERSION;
        }
        s
    }

    pub fn save(&self, root: &Path) {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(3);
        match ron::ser::to_string_pretty(self, cfg) {
            Ok(t) => {
                let text = format!("// Reglages du joueur (modifiables depuis le menu Options).\n{t}\n");
                if let Err(e) = crate::storage::write(root, "settings.ron", &text) {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::Settings;

    /// Reglages illisibles : reglages par defaut, et le fichier abime est garde a cote.
    #[test]
    fn broken_settings_are_kept_aside() {
        let dir = std::env::temp_dir().join(format!("astp_settings_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.ron"), "(lang: \"fr\", oups").unwrap();
        let s = Settings::load(&dir);
        assert_eq!(s.lang, Settings::default().lang);
        let kept = std::fs::read_to_string(dir.join("settings.broken.ron")).unwrap();
        assert!(kept.contains("oups"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
