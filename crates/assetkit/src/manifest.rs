//! Manifestes d'assets (ecrits a la main dans `data/sprites/*.ron`) : ils decrivent
//! ou trouver chaque frame dans les planches d'origine. Lus par l'outil `forge`.
use crate::meta::AnimMode;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub sheets: Vec<Sheet>,
    #[serde(default)]
    pub fonts: Vec<FontSpec>,
    #[serde(default)]
    pub tilesets: Vec<TilesetSpec>,
}

/// Une planche source et les sprites qu'on y decoupe.
#[derive(Debug, Deserialize)]
pub struct Sheet {
    /// Nom du fichier dans `assets/source/` (le prefixe commun des planches ALttP
    /// peut etre omis : forge le retrouve).
    pub source: String,
    /// Couleurs rendues transparentes ("#ff00ff"). "auto" = couleur la plus frequente.
    #[serde(default)]
    pub transparent: Vec<String>,
    pub sprites: BTreeMap<String, BTreeMap<String, AnimSpec>>,
}

/// Alignement vertical commun aux frames d'une animation : les frames d'une meme
/// rangee de la planche partagent le haut (Top) ou le bas (Bottom).
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Top,
    Bottom,
    /// Chaque frame est centree sur son propre milieu (effets, particules).
    Center,
}

#[derive(Debug, Deserialize)]
pub struct AnimSpec {
    /// Rectangles (x, y, largeur, hauteur) dans la planche.
    pub frames: Vec<(u32, u32, u32, u32)>,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default)]
    pub mode: AnimMode,
    /// Point d'ancrage. x : depuis le centre de la frame si `center_x`, sinon depuis le bord gauche.
    /// y : depuis le haut commun (Align::Top) ou le bas commun (Align::Bottom) de l'animation.
    pub pivot: (i32, i32),
    #[serde(default = "default_true")]
    pub center_x: bool,
    #[serde(default)]
    pub align: Align,
}

fn default_fps() -> f32 {
    8.0
}
fn default_true() -> bool {
    true
}

/// Police bitmap : un ou plusieurs blocs de cases (planches differentes), plus des
/// glyphes composes (lettres accentuees, signes retournes, dessins).
#[derive(Debug, Deserialize)]
pub struct FontSpec {
    pub name: String,
    /// Rangee de la case qui correspond au haut de la ligne de texte.
    #[serde(default)]
    pub top: u32,
    /// Hauteur d'une ligne de texte.
    pub line_h: u16,
    /// Largeur d'une espace.
    pub space: u16,
    /// Espacement entre deux glyphes (negatif : contours partages).
    #[serde(default)]
    pub spacing: i16,
    /// Decalage vertical du texte a l'affichage (negatif : remonte), pour centrer les lettres dans les
    /// cadres quand la case garde de la place au-dessus pour les accents.
    #[serde(default)]
    pub nudge: i16,
    /// Rangees de la case reservees aux accents des minuscules (au-dessus du corps).
    #[serde(default)]
    pub accent_rows: u32,
    /// Couleurs (contour, remplissage) des glyphes dessines.
    #[serde(default = "default_ink")]
    pub ink: (String, String),
    pub blocks: Vec<FontBlock>,
    #[serde(default)]
    pub compose: Vec<Compose>,
}

fn default_ink() -> (String, String) {
    ("#000070".into(), "#f8f8f8".into())
}

/// Grille de cases dans une planche. Les blocs suivants remplacent les precedents.
#[derive(Debug, Deserialize)]
pub struct FontBlock {
    pub source: String,
    #[serde(default)]
    pub transparent: Vec<String>,
    /// Coin haut-gauche de la premiere case.
    pub origin: (u32, u32),
    /// Taille d'une case (largeur, hauteur).
    pub cell: (u32, u32),
    /// Une chaine par rangee ; '¤' = case ignoree.
    pub rows: Vec<String>,
    /// Chasse fixe : la case n'est pas rognee horizontalement (kana).
    #[serde(default)]
    pub mono: bool,
}

/// Glyphe fabrique a partir d'autres glyphes.
#[derive(Debug, Deserialize)]
pub enum Compose {
    /// (resultat, base, donneur) : corps de `base`, accent pris au-dessus du corps du donneur.
    Accent(char, char, char),
    /// (resultat, majuscule, donneur) : accent du donneur pose au-dessus de la majuscule
    /// (la case grandit vers le haut si besoin, comme Ä Ö Ü).
    AccentCap(char, char, char),
    /// (resultat, base, masque) : accent dessine ('o' = remplissage, contour automatique).
    AccentArt(char, char, Vec<String>),
    /// (resultat, source) : demi-tour (¿ ¡).
    Rotate(char, char),
    /// (resultat, rangee de la case, masque) : glyphe dessine, contour automatique.
    Art(char, u32, Vec<String>),
}

/// Tuiles prelevees dans une carte (coins haut-gauche, taille fixe).
#[derive(Debug, Deserialize)]
pub struct TilesetSpec {
    pub name: String,
    pub source: String,
    pub size: u32,
    pub tiles: Vec<(u32, u32)>,
}
