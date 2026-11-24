//! engine : couche plateforme du jeu (fenetre, rendu, entrees, audio, assets).
//! Ne contient aucune regle de jeu.

pub mod app;
pub mod assets;
pub mod audio;
pub mod draw;
pub mod gfx;
pub mod input;
pub mod lofi;
pub mod vfs;
#[cfg(target_arch = "wasm32")]
mod webmusic;

pub use glam;
pub use glam::{vec2, Vec2};

/// Resolution logique de reference (hauteur fixe 360, largeur adaptee a l'ecran).
pub const LOGICAL_W: f32 = 640.0;
pub const LOGICAL_H: f32 = 360.0;
/// Frequence de la simulation (pas fixe).
pub const TICK_HZ: f32 = 60.0;
pub const TICK_DT: f32 = 1.0 / TICK_HZ;
