//! La horde : tous les ennemis en tableaux paralleles (SoA), compacts et parcourus lineairement.
use glam::Vec2;

/// Orientation d'affichage.
pub const FACE_DOWN: u8 = 0;
pub const FACE_RIGHT: u8 = 1;
pub const FACE_UP: u8 = 2;
pub const FACE_LEFT: u8 = 3;

/// Sources de degats suivies separement (une par emplacement d'arme + zones + divers).
pub const HIT_SLOTS: usize = 8;
/// Joueurs d'une partie en coop, au plus.
pub const MAX_HEROES: usize = 4;
/// Delais de touche d'un ennemi : un jeu d'emplacements d'armes par joueur.
pub const HIT_CD: usize = HIT_SLOTS * MAX_HEROES;
/// Emplacement des zones au sol et brulures.
pub const SLOT_ZONE: usize = 6;
/// Emplacement des degats divers (faisceaux, onde).
pub const SLOT_MISC: usize = 7;

/// Etat d'un comportement (charge, tir, boss).
pub const ST_MOVE: u8 = 0;
pub const ST_WINDUP: u8 = 1;
pub const ST_ACT: u8 = 2;
pub const ST_REST: u8 = 3;

#[derive(Default)]
pub struct Horde {
    pub pos: Vec<Vec2>,
    /// Position au pas precedent (interpolation de rendu).
    pub prev: Vec<Vec2>,
    /// Vitesse de recul (decroit vite).
    pub knock: Vec<Vec2>,
    /// Vitesse propre (sauts, charges, flanerie).
    pub vel: Vec<Vec2>,
    pub kind: Vec<u16>,
    pub hp: Vec<f32>,
    pub max_hp: Vec<f32>,
    pub radius: Vec<f32>,
    /// Identifiant unique (stable malgre les retraits).
    pub uid: Vec<u32>,
    /// Pas restants avant qu'une source puisse toucher de nouveau.
    pub hit_cd: Vec<[u8; HIT_CD]>,
    /// Temps depuis le dernier degat recu (s).
    pub hurt: Vec<f32>,
    pub face: Vec<u8>,
    /// Graine individuelle (dephasage des animations et comportements).
    pub seed: Vec<u32>,
    /// Minuterie et etat de comportement.
    pub timer: Vec<f32>,
    pub state: Vec<u8>,
    /// Action en cours (boss).
    pub step: Vec<u8>,
    /// Hauteur au-dessus du sol (sauts, vol).
    pub z: Vec<f32>,
    /// Effets d'etat (s restantes) et degats de brulure.
    pub burn: Vec<f32>,
    pub burn_dps: Vec<f32>,
    pub freeze: Vec<f32>,
    pub stun: Vec<f32>,
    /// Elite : contour dore, coffre a la mort.
    pub elite: Vec<bool>,
    /// Multiplicateur de degats au contact.
    pub power: Vec<f32>,
    /// Nombre d'ennemis vivants par type.
    pub alive: Vec<u32>,
    next_uid: u32,
}

impl Horde {
    pub fn with_kinds(n: usize) -> Horde {
        Horde { alive: vec![0; n], ..Default::default() }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.pos.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty()
    }

    pub fn spawn(&mut self, kind: u16, pos: Vec2, hp: f32, radius: f32, seed: u32) -> usize {
        self.pos.push(pos);
        self.prev.push(pos);
        self.knock.push(Vec2::ZERO);
        self.vel.push(Vec2::ZERO);
        self.kind.push(kind);
        self.hp.push(hp);
        self.max_hp.push(hp);
        self.radius.push(radius);
        self.uid.push(self.next_uid);
        self.next_uid = self.next_uid.wrapping_add(1);
        self.hit_cd.push([0; HIT_CD]);
        self.hurt.push(99.0);
        self.face.push(FACE_DOWN);
        self.seed.push(seed);
        self.timer.push((seed % 1000) as f32 / 1000.0);
        self.state.push(ST_MOVE);
        self.step.push(0);
        self.z.push(0.0);
        self.burn.push(0.0);
        self.burn_dps.push(0.0);
        self.freeze.push(0.0);
        self.stun.push(0.0);
        self.elite.push(false);
        self.power.push(1.0);
        self.alive[kind as usize] += 1;
        self.pos.len() - 1
    }

    /// Retire l'ennemi `i` (le dernier prend sa place).
    pub fn remove(&mut self, i: usize) {
        self.alive[self.kind[i] as usize] -= 1;
        self.pos.swap_remove(i);
        self.prev.swap_remove(i);
        self.knock.swap_remove(i);
        self.vel.swap_remove(i);
        self.kind.swap_remove(i);
        self.hp.swap_remove(i);
        self.max_hp.swap_remove(i);
        self.radius.swap_remove(i);
        self.uid.swap_remove(i);
        self.hit_cd.swap_remove(i);
        self.hurt.swap_remove(i);
        self.face.swap_remove(i);
        self.seed.swap_remove(i);
        self.timer.swap_remove(i);
        self.state.swap_remove(i);
        self.step.swap_remove(i);
        self.z.swap_remove(i);
        self.burn.swap_remove(i);
        self.burn_dps.swap_remove(i);
        self.freeze.swap_remove(i);
        self.stun.swap_remove(i);
        self.elite.swap_remove(i);
        self.power.swap_remove(i);
    }

    /// Immobilise (gel ou etourdissement).
    #[inline]
    pub fn held(&self, i: usize) -> bool {
        self.freeze[i] > 0.0 || self.stun[i] > 0.0
    }
}

/// Orientation a partir d'une direction, avec hysteresis pour eviter le clignotement.
#[inline]
pub fn face_from(d: Vec2, current: u8) -> u8 {
    if d.length_squared() < 1e-6 {
        return current;
    }
    let horizontal_now = current == FACE_LEFT || current == FACE_RIGHT;
    let bias = 1.25;
    let horiz = if horizontal_now { d.x.abs() * bias > d.y.abs() } else { d.x.abs() > d.y.abs() * bias };
    if horiz {
        if d.x > 0.0 {
            FACE_RIGHT
        } else {
            FACE_LEFT
        }
    } else if d.y > 0.0 {
        FACE_DOWN
    } else {
        FACE_UP
    }
}
