//! Modificateurs d'une partie, venus de la progression du joueur : achats de la boutique,
//! reliques et benedictions (bonus), difficulte et Sceaux de Ganon (malus).
use crate::defs::Stat;

#[derive(Clone, Debug)]
pub struct RunMods {
    /// Bonus permanents ajoutes aux statistiques de depart.
    pub stats: Vec<(Stat, f32)>,
    /// Multiplicateurs appliques aux ennemis.
    pub enemy_hp: f32,
    pub enemy_damage: f32,
    pub enemy_speed: f32,
    /// Multiplicateur des boss (en plus de `enemy_hp`).
    pub boss_hp: f32,
    /// Multiplicateur du flux d'apparition et des vagues.
    pub spawn_rate: f32,
    /// Soin des coeurs ramasses (0 = aucun soin).
    pub heal: f32,
    /// Gain d'experience.
    pub xp: f32,
    /// Fees en bouteille : Link se releve avec la moitie de sa vie.
    pub revives: u32,
    /// Montee de niveau : relances (nouveaux choix) et passes (niveau sans recompense) restantes.
    pub rerolls: u32,
    pub skips: u32,
    /// Sorciere achetee a la Grande Fee : elle passe une fois par partie.
    pub witch: bool,
}

impl Default for RunMods {
    fn default() -> Self {
        RunMods {
            stats: Vec::new(),
            enemy_hp: 1.0,
            enemy_damage: 1.0,
            enemy_speed: 1.0,
            boss_hp: 1.0,
            spawn_rate: 1.0,
            heal: 1.0,
            xp: 1.0,
            revives: 0,
            // 2 de chaque par partie, plus la boutique et les reliques
            rerolls: 2,
            skips: 2,
            witch: false,
        }
    }
}
