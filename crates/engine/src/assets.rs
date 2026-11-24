//! Chargement de l'atlas genere par `forge` et acces rapide aux animations par poignee.
use assetkit::meta::{Anim, AtlasMeta, Font, Frame};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Poignee d'animation (indice dans l'atlas), resolue une fois au chargement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct AnimId(pub u32);

pub struct Atlas {
    pub meta: AtlasMeta,
    anims: Vec<Anim>,
    by_name: HashMap<(String, String), AnimId>,
    pub white: Frame,
}

impl Atlas {
    pub fn load(root: &Path) -> (Atlas, image::RgbaImage) {
        let dir = root.join("assets/gen");
        let text = crate::vfs::read_string(&dir.join("atlas.ron"))
            .unwrap_or_else(|| panic!("assets/gen/atlas.ron introuvable : lancer `cargo run -p forge`"));
        let meta: AtlasMeta = ron::from_str(&text).expect("atlas.ron invalide");
        let png = crate::vfs::read(&dir.join(&meta.image)).expect("atlas.png");
        let img = image::load_from_memory(&png).expect("atlas.png").to_rgba8();
        let mut anims = Vec::new();
        let mut by_name = HashMap::new();
        for (s, set) in &meta.sprites {
            for (a, anim) in set {
                by_name.insert((s.clone(), a.clone()), AnimId(anims.len() as u32));
                anims.push(anim.clone());
            }
        }
        let white = meta.sprites["white"]["0"].frames[0];
        let white = Frame { x: white.x + 1, y: white.y + 1, w: 1, h: 1, px: 0, py: 0 };
        (Atlas { meta, anims, by_name, white }, img)
    }

    /// Recherche par nom ; `None` si absente.
    pub fn find(&self, sprite: &str, anim: &str) -> Option<AnimId> {
        self.by_name.get(&(sprite.to_string(), anim.to_string())).copied()
    }

    /// Recherche par nom ; journalise et renvoie le pixel blanc si absente.
    pub fn get(&self, sprite: &str, anim: &str) -> AnimId {
        self.find(sprite, anim).unwrap_or_else(|| {
            log::warn!("animation absente : {sprite}/{anim}");
            self.find("white", "0").unwrap()
        })
    }

    pub fn has_sprite(&self, sprite: &str) -> bool {
        self.meta.sprites.contains_key(sprite)
    }

    #[inline]
    pub fn anim(&self, id: AnimId) -> &Anim {
        &self.anims[id.0 as usize]
    }

    #[inline]
    pub fn frame(&self, id: AnimId, t: f32) -> &Frame {
        self.anims[id.0 as usize].frame_at(t)
    }

    pub fn font(&self, name: &str) -> &Font {
        self.meta.fonts.get(name).unwrap_or_else(|| panic!("police absente : {name}"))
    }

    pub fn tiles(&self, set: &str) -> &[Frame] {
        self.meta.tilesets.get(set).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Trouve la racine du jeu (dossier contenant `assets/` et `data/`) :
/// a cote de l'executable, puis en remontant depuis le dossier courant.
#[cfg(target_arch = "wasm32")]
pub fn find_root() -> PathBuf {
    PathBuf::new()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn find_root() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        candidates.extend(exe.ancestors().skip(1).map(Path::to_path_buf));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.extend(cwd.ancestors().map(Path::to_path_buf));
    }
    candidates
        .into_iter()
        .find(|p| p.join("assets/gen/atlas.ron").exists() && p.join("data").is_dir())
        .unwrap_or_else(|| PathBuf::from("."))
}
