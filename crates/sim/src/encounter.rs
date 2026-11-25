//! Rencontres : de temps en temps, un champion errant (une elite renforcee) attend loin du joueur.
//! Une annonce et une fleche indiquent ou il se trouve ; il reste sur place jusqu'a ce que le joueur
//! approche, puis se bat. Vaincu, il laisse une recompense a choisir (benediction, amelioration speciale
//! d'une arme ou relique) ; ignore trop longtemps, il repart.
use crate::world::{Event, World, DT};
use glam::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Encounter {
    /// Identifiant stable de l'ennemi dans la horde.
    pub uid: u32,
    /// Point ou il attend.
    pub anchor: Vec2,
    /// Temps restant avant son depart (tant qu'il n'est pas engage).
    pub t: f32,
    /// Le joueur l'a rejoint : il se bat jusqu'au bout.
    pub engaged: bool,
}

impl World {
    /// L'ennemi `i` est le champion de la rencontre en cours.
    pub fn is_champion(&self, i: usize) -> bool {
        self.encounter.as_ref().is_some_and(|e| self.horde.uid.get(i) == Some(&e.uid))
    }

    /// Position du champion en cours (pour la fleche).
    pub fn champion_pos(&self) -> Option<Vec2> {
        let e = self.encounter.as_ref()?;
        let i = self.horde.uid.iter().position(|&u| u == e.uid)?;
        Some(self.horde.pos[i])
    }

    pub(crate) fn update_encounter(&mut self) {
        let r = self.content.rules.encounter.clone();
        if let Some(mut e) = self.encounter {
            let Some(i) = self.horde.uid.iter().position(|&u| u == e.uid) else {
                self.encounter = None;
                return;
            };
            if !e.engaged {
                let d = (self.horde.pos[i] - self.player.pos).length();
                if d < r.engage {
                    e.engaged = true;
                } else {
                    // attend sur place
                    self.horde.pos[i] = e.anchor;
                    self.horde.prev[i] = e.anchor;
                    self.horde.knock[i] = Vec2::ZERO;
                    e.t -= DT;
                    if e.t <= 0.0 {
                        self.horde.remove(i);
                        self.encounter = None;
                        self.encounter_t = self.rng.range(r.every.0, r.every.1);
                        self.events.push(Event::EncounterFled { pos: e.anchor });
                        return;
                    }
                }
            }
            self.encounter = Some(e);
            return;
        }
        if self.boss_uid.is_some() || self.time > self.map.duration - r.quiet_end || !self.player.alive {
            return;
        }
        self.encounter_t -= DT;
        if self.encounter_t > 0.0 {
            return;
        }
        // un champion parmi les elites du stage, sinon parmi les vagues
        let pool: Vec<u16> = if self.map.elites.is_empty() {
            self.map.waves.iter().filter_map(|w| self.content.enemy_index(&w.enemy)).collect()
        } else {
            self.map.elites.iter().filter_map(|w| self.content.enemy_index(&w.enemy)).collect()
        };
        if pool.is_empty() {
            self.encounter_t = 1e9;
            return;
        }
        let k = pool[(self.rng.f() * pool.len() as f32) as usize % pool.len()];
        let edge = self.view_half.length();
        let at = self.player.pos + self.rng.dir() * (edge + self.rng.range(r.dist.0, r.dist.1));
        let i = self.spawn_enemy(k, at);
        let rules = &self.content.rules;
        self.horde.elite[i] = true;
        self.horde.hp[i] *= rules.elite_hp * r.hp;
        self.horde.max_hp[i] = self.horde.hp[i];
        self.horde.power[i] = rules.elite_damage * r.damage * self.mods.enemy_damage;
        self.encounter = Some(Encounter { uid: self.horde.uid[i], anchor: at, t: r.life, engaged: false });
        self.events.push(Event::EncounterSpawned { pos: at, kind: k });
    }
}
