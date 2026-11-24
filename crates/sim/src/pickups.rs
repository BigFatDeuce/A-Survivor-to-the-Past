//! Objets au sol : rubis (experience) et coeurs.
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PickupKind {
    RupeeGreen = 0,
    RupeeBlue = 1,
    RupeeRed = 2,
    Heart = 3,
    /// Coffre (elite) : evolution ou ameliorations.
    Chest = 4,
    /// Grand coffre (boss).
    BigChest = 5,
    /// Filet a papillons : attire tous les rubis.
    Magnet = 6,
    /// Fee : soigne entierement.
    Fairy = 7,
    /// Medaillon d'Ether : gele les ennemis a l'ecran.
    Medallion = 8,
}

impl PickupKind {
    pub fn is_rupee(self) -> bool {
        matches!(self, PickupKind::RupeeGreen | PickupKind::RupeeBlue | PickupKind::RupeeRed)
    }
    pub fn is_chest(self) -> bool {
        matches!(self, PickupKind::Chest | PickupKind::BigChest)
    }
    /// Objet rare du decor (jamais oublie au loin, ni fusionne).
    pub fn is_special(self) -> bool {
        matches!(self, PickupKind::Magnet | PickupKind::Fairy | PickupKind::Medallion)
    }
}

impl PickupKind {
    /// Rubis correspondant a une valeur (1 vert, 5 bleu, 20 rouge).
    pub fn rupee_for(value: u32) -> PickupKind {
        if value >= 20 {
            PickupKind::RupeeRed
        } else if value >= 5 {
            PickupKind::RupeeBlue
        } else {
            PickupKind::RupeeGreen
        }
    }
}

#[derive(Default)]
pub struct Pickups {
    pub pos: Vec<Vec2>,
    pub prev: Vec<Vec2>,
    pub vel: Vec<Vec2>,
    pub kind: Vec<PickupKind>,
    pub value: Vec<u32>,
    /// Hauteur et vitesse verticale (petit rebond a l'apparition).
    pub z: Vec<f32>,
    pub vz: Vec<f32>,
    /// Attire par le joueur depuis (s), negatif = au repos.
    pub pull: Vec<f32>,
    pub age: Vec<f32>,
}

impl Pickups {
    #[inline]
    pub fn len(&self) -> usize {
        self.pos.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty()
    }

    pub fn spawn(&mut self, kind: PickupKind, value: u32, pos: Vec2, vel: Vec2, vz: f32) {
        self.pos.push(pos);
        self.prev.push(pos);
        self.vel.push(vel);
        self.kind.push(kind);
        self.value.push(value);
        self.z.push(0.0);
        self.vz.push(vz);
        self.pull.push(-1.0);
        self.age.push(0.0);
    }

    pub fn remove(&mut self, i: usize) {
        self.pos.swap_remove(i);
        self.prev.swap_remove(i);
        self.vel.swap_remove(i);
        self.kind.swap_remove(i);
        self.value.swap_remove(i);
        self.z.swap_remove(i);
        self.vz.swap_remove(i);
        self.pull.swap_remove(i);
        self.age.swap_remove(i);
    }
}
