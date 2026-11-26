//! Directeur des ralentis : decide quand un moment merite un tout petit ralenti.
//!
//! Le ralenti ne doit jamais devenir une gene. Trois garde-fous :
//! - un delai minimal entre deux ralentis (par type de moment) ;
//! - un budget : au plus `BUDGET` secondes de ralenti par fenetre glissante d'environ 30 s ;
//! - des ralentis tres courts (0,1 a 0,3 s) qui reviennent en douceur a la vitesse normale.
//!
//! Quand un ralenti est refuse, les effets visuels et sonores restent : seul le temps n'est pas touche.
use std::collections::VecDeque;

/// Fenetre de comptage des ennemis vaincus en rafale (s de jeu).
const KILL_WINDOW: f32 = 0.3;
/// Nombre d'ennemis vaincus dans la fenetre pour une « grosse vague ».
pub const MASS_KILL: usize = 24;
/// Secondes de ralenti autorisees, rechargees a raison de BUDGET / 30 par seconde.
const BUDGET: f32 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    /// Un coup tres puissant (explosion, seisme) qui touche beaucoup d'ennemis.
    Impact,
    /// Une grosse vague vient de mourir.
    MassKill,
    /// Boss vaincu : toujours accorde.
    Boss,
}

impl Moment {
    /// (facteur de vitesse, tenue en s, retour en s, delai minimal avant le prochain du meme type)
    fn shape(self) -> (f32, f32, f32, f32) {
        match self {
            Moment::Impact => (0.45, 0.06, 0.12, 9.0),
            Moment::MassKill => (0.3, 0.12, 0.22, 14.0),
            Moment::Boss => (0.15, 0.5, 0.8, 0.0),
        }
    }
}

struct Slow {
    t: f32,
    factor: f32,
    hold: f32,
    ramp: f32,
}

pub struct Director {
    kills: VecDeque<f32>,
    /// Temps reel restant avant de pouvoir accorder chaque type de moment.
    cool: [f32; 3],
    /// Ralenti deja consomme dans la fenetre (se vide avec le temps).
    spent: f32,
    slow: Option<Slow>,
    /// Pour le journal de reglage : (accordes, refuses).
    pub stats: (u32, u32),
}

impl Director {
    pub fn new() -> Director {
        Director { kills: VecDeque::with_capacity(256), cool: [0.0; 3], spent: 0.0, slow: None, stats: (0, 0) }
    }

    /// Note un ennemi vaincu ; renvoie vrai quand une grosse vague vient d'etre atteinte.
    pub fn kill(&mut self, now: f32) -> bool {
        self.kills.push_back(now);
        while self.kills.front().is_some_and(|&t| now - t > KILL_WINDOW) {
            self.kills.pop_front();
        }
        if self.kills.len() >= MASS_KILL {
            self.kills.clear();
            return true;
        }
        false
    }

    /// Demande un ralenti ; renvoie vrai s'il est accorde.
    pub fn request(&mut self, m: Moment) -> bool {
        let (factor, hold, ramp, gap) = m.shape();
        let idx = m as usize;
        let cost = hold + ramp * 0.5;
        let forced = m == Moment::Boss;
        if !forced && (self.cool[idx] > 0.0 || self.spent + cost > BUDGET || self.slow.is_some()) {
            self.stats.1 += 1;
            return false;
        }
        self.cool[idx] = gap;
        self.spent += cost;
        self.slow = Some(Slow { t: 0.0, factor, hold, ramp });
        self.stats.0 += 1;
        true
    }

    /// Avance en temps reel.
    pub fn update(&mut self, real: f32) {
        for c in &mut self.cool {
            *c = (*c - real).max(0.0);
        }
        self.spent = (self.spent - real * BUDGET / 30.0).max(0.0);
        if let Some(s) = &mut self.slow {
            s.t += real;
            if s.t >= s.hold + s.ramp {
                self.slow = None;
            }
        }
    }

    /// Facteur de vitesse du jeu (1 = normal).
    pub fn scale(&self) -> f32 {
        match &self.slow {
            None => 1.0,
            Some(s) if s.t < s.hold => s.factor,
            Some(s) => {
                let u = ((s.t - s.hold) / s.ramp).clamp(0.0, 1.0);
                // retour progressif (ease-out) : pas de « saut » de vitesse
                let e = 1.0 - (1.0 - u) * (1.0 - u);
                s.factor + (1.0 - s.factor) * e
            }
        }
    }

    pub fn clear(&mut self) {
        *self = Director::new();
    }
}
