//! sim : les regles du jeu (deplacements, combat, horde, armes, objets, progression).
//! Aucune dependance au rendu ni a la plateforme.

pub mod arsenal;
pub mod defs;
pub mod encounter;
pub mod enemies;
pub mod grid;
pub mod horde;
pub mod mods;
pub mod pickups;
pub mod props;
pub mod rng;
pub mod upgrades;
pub mod world;

pub use glam;
pub use world::{Event, Player, PlayerInput, Swing, World, DT};
