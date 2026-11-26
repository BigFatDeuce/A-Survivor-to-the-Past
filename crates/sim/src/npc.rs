//! PNJ de passage (0.6.3). La diseuse de bonne aventure remplace le grand coffre du boss final ; la
//! sorciere (achetee a la Grande Fee) passe une fois par partie, entre la 2e et la 5e minute, et
//! repart au bout de 30 s. Les toucher ouvre leur menu (le jeu se met en pause : le chrono aussi).
use crate::world::{Event, World, DT};
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcKind {
    Fortune,
    Witch,
}

#[derive(Clone, Copy, Debug)]
pub struct Npc {
    pub kind: NpcKind,
    pub pos: Vec2,
    /// Temps depuis l'arrivee, et duree de la visite (infinie pour la diseuse).
    pub t: f32,
    pub life: f32,
    /// Joueur au contact a l'image precedente : il faut s'eloigner pour la rouvrir.
    near: bool,
}

pub const WITCH_LIFE: f32 = 30.0;
/// Arrivee de la sorciere : entre 2 et 5 minutes de partie.
pub const WITCH_WINDOW: (f32, f32) = (120.0, 300.0);
const TOUCH: f32 = 16.0;

impl World {
    pub fn add_npc(&mut self, kind: NpcKind, at: Vec2) {
        let at = crate::props::resolve(&self.map.props, self.prop_seed, at, 12.0, Vec2::ZERO, 1.0);
        let life = if kind == NpcKind::Witch { WITCH_LIFE } else { f32::INFINITY };
        self.npcs.push(Npc { kind, pos: at, t: 0.0, life, near: false });
        self.events.push(Event::NpcArrived { kind, pos: at });
    }

    /// Le PNJ s'en va (fin de visite, achat fait, jeux termines).
    pub fn remove_npc(&mut self, kind: NpcKind) {
        if let Some(i) = self.npcs.iter().position(|n| n.kind == kind) {
            let pos = self.npcs.remove(i).pos;
            self.events.push(Event::NpcLeft { kind, pos });
        }
    }

    pub(crate) fn update_npcs(&mut self) {
        if self.witch_at.is_some_and(|t| self.time >= t) && self.player.alive {
            self.witch_at = None;
            let at = self.player.pos + self.rng.dir() * self.rng.range(70.0, 100.0);
            self.add_npc(NpcKind::Witch, at);
        }
        let me = self.player.pos;
        let mut gone = Vec::new();
        for n in &mut self.npcs {
            n.t += DT;
            if n.t >= n.life {
                gone.push(n.kind);
                continue;
            }
            let inside = (n.pos - me).length() < TOUCH;
            if inside && !n.near && n.t > 0.6 {
                self.events.push(Event::NpcMet { kind: n.kind });
            }
            n.near = inside;
        }
        for k in gone {
            self.remove_npc(k);
        }
    }
}
