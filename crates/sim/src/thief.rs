//! Pikit voleur de rubis : 2 a 3 fois par partie, un Pikit (A Link to the Past) arrive hors de l'ecran.
//! Une annonce et une fleche indiquent ou il est ; il fuit en sautant quand le joueur approche.
//! Vaincu, sa langue rapporte au joueur tous les rubis encore au sol ; ignore, il repart.
use crate::world::{Event, World, DT};
use glam::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Thief {
    /// Identifiant stable du Pikit dans la horde.
    pub uid: u32,
    /// Temps restant avant son depart (s).
    pub t: f32,
}

impl World {
    /// Position du Pikit en cours (pour la fleche).
    pub fn thief_pos(&self) -> Option<Vec2> {
        let th = self.thief.as_ref()?;
        let i = self.horde.uid.iter().position(|&u| u == th.uid)?;
        Some(self.horde.pos[i])
    }

    pub(crate) fn update_thief(&mut self) {
        let Some(kind) = self.thief_kind else { return };
        let r = self.content.rules.thief.clone();
        if let Some(mut th) = self.thief {
            let Some(i) = self.horde.uid.iter().position(|&u| u == th.uid) else {
                self.thief = None;
                return;
            };
            th.t -= DT;
            if th.t <= 0.0 {
                let pos = self.horde.pos[i];
                self.horde.remove(i);
                self.thief = None;
                self.thief_t = self.rng.range(r.every.0, r.every.1);
                self.events.push(Event::ThiefFled { pos });
                return;
            }
            self.thief = Some(th);
            return;
        }
        // pas pendant un boss ni a la toute fin du stage
        if self.boss_uid.is_some() || self.time > self.map.duration - 30.0 || !self.player.alive {
            return;
        }
        self.thief_t -= DT;
        if self.thief_t > 0.0 {
            return;
        }
        let edge = self.view_half.length();
        let at = self.player.pos + self.rng.dir() * (edge + self.rng.range(20.0, 90.0));
        let i = self.spawn_enemy(kind, at);
        self.horde.hp[i] *= r.hp;
        self.horde.max_hp[i] = self.horde.hp[i];
        self.thief = Some(Thief { uid: self.horde.uid[i], t: r.life });
        self.events.push(Event::ThiefSpawned { pos: at });
    }

    /// Pikit vaincu : tous les rubis au sol viennent au joueur.
    pub(crate) fn thief_defeated(&mut self, pos: Vec2) {
        let count = (0..self.pickups.len()).filter(|&i| self.pickups.kind[i].is_rupee()).count() as u32;
        self.thief = None;
        let r = &self.content.rules.thief;
        self.thief_t = self.rng.range(r.every.0, r.every.1);
        self.vacuum();
        self.events.push(Event::ThiefLooted { pos, count });
    }
}
