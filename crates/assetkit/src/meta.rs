//! Description d'un atlas genere (lue par le jeu).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Mode de lecture d'une animation.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AnimMode {
    #[default]
    Loop,
    /// 1-2-3-2-1-2-3...
    PingPong,
    /// Joue une fois et reste sur la derniere frame.
    Once,
}

/// Une frame dans l'atlas : rectangle en texels + point d'ancrage (pieds du personnage)
/// exprime dans le repere de la frame.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Frame {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub px: i16,
    pub py: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Anim {
    pub frames: Vec<Frame>,
    pub fps: f32,
    pub mode: AnimMode,
}

impl Anim {
    /// Frame a afficher apres `t` secondes de lecture.
    pub fn frame_at(&self, t: f32) -> &Frame {
        let n = self.frames.len();
        if n <= 1 {
            return &self.frames[0];
        }
        let i = (t.max(0.0) * self.fps) as usize;
        let idx = match self.mode {
            AnimMode::Loop => i % n,
            AnimMode::Once => i.min(n - 1),
            AnimMode::PingPong => {
                let period = 2 * n - 2;
                let k = i % period;
                if k < n {
                    k
                } else {
                    period - k
                }
            }
        };
        &self.frames[idx]
    }

    /// Duree d'un cycle complet (pour les animations `Once`).
    pub fn duration(&self) -> f32 {
        self.frames.len() as f32 / self.fps.max(0.001)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Glyph {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    /// Decalage vertical depuis le haut de la ligne (accents des majuscules : negatif).
    #[serde(default)]
    pub oy: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Font {
    pub glyphs: BTreeMap<char, Glyph>,
    pub line_height: u16,
    /// Largeur d'une espace.
    pub space: u16,
    /// Espacement entre deux glyphes (peut etre negatif : contours partages).
    pub spacing: i16,
}

impl Font {
    /// Largeur en pixels d'un texte sur une ligne.
    pub fn measure(&self, text: &str) -> i32 {
        let mut w = 0i32;
        let mut first = true;
        for c in text.chars() {
            if !first {
                w += self.spacing as i32;
            }
            first = false;
            w += match self.glyph(c) {
                Some(g) => g.w as i32,
                None => self.space as i32,
            };
        }
        w
    }

    /// Glyphe d'un caractere ; a defaut, la lettre sans accent (puis l'autre casse).
    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        if let Some(g) = self.glyphs.get(&c) {
            return Some(g);
        }
        let b = base_letter(c);
        self.glyphs
            .get(&b)
            .or_else(|| b.to_uppercase().next().and_then(|u| self.glyphs.get(&u)))
            .or_else(|| b.to_lowercase().next().and_then(|l| self.glyphs.get(&l)))
    }
}

/// Lettre de base d'une lettre accentuee (repli quand la police n'a pas le glyphe).
pub fn base_letter(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'À' | 'Á' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ÿ' => 'y',
        '’' | '‘' => '\'',
        '«' | '»' | '“' | '”' => '"',
        '？' => '?',
        '！' => '!',
        '（' => '(',
        '）' => ')',
        '／' => '/',
        '：' => ':',
        _ => c,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AtlasMeta {
    /// Image de l'atlas (relative au fichier .ron).
    pub image: String,
    pub width: u32,
    pub height: u32,
    /// sprite -> animation -> frames
    pub sprites: BTreeMap<String, BTreeMap<String, Anim>>,
    pub fonts: BTreeMap<String, Font>,
    pub tilesets: BTreeMap<String, Vec<Frame>>,
}
