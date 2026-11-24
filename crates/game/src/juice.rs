//! « Juice » : tout ce qui rend l'action satisfaisante sans changer les regles.
//! Particules, chiffres de degats, tremblement, arrets sur image, flashs, anneaux, images remanentes.
use assetkit::meta::Frame;
use engine::assets::{AnimId, Atlas};
use engine::draw::{with_alpha, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::rng::Rng;

pub enum PKind {
    Anim(AnimId),
    /// Carre de couleur (debris, etincelles).
    Pixel(f32),
}

pub struct Particle {
    pub rot: f32,
    pub scale: f32,
    pub pos: Vec2,
    pub vel: Vec2,
    pub z: f32,
    pub vz: f32,
    pub gravity: f32,
    pub drag: f32,
    pub t: f32,
    pub life: f32,
    pub kind: PKind,
    pub color: Color,
    pub additive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NumKind {
    Normal,
    Crit,
    /// Brulure, flammes au sol : petits chiffres orange, discrets.
    Dot,
    /// Degats subis par le boss.
    Boss,
}

pub struct Number {
    pub pos: Vec2,
    pub vy: f32,
    pub t: f32,
    pub value: u32,
    pub kind: NumKind,
}

pub struct Ring {
    pub pos: Vec2,
    pub t: f32,
    pub life: f32,
    pub r0: f32,
    pub r1: f32,
    pub color: Color,
}

pub struct Ghost {
    pub frame: Frame,
    pub pos: Vec2,
    pub flip: bool,
    pub t: f32,
    pub life: f32,
    pub color: Color,
}

pub struct Juice {
    pub particles: Vec<Particle>,
    pub numbers: Vec<Number>,
    pub rings: Vec<Ring>,
    pub ghosts: Vec<Ghost>,
    /// Intensite du tremblement (0..1), decroit avec le temps.
    pub trauma: f32,
    /// Arret sur image restant (s, temps reel).
    pub hitstop: f32,
    /// Flash plein ecran : couleur, temps restant, duree.
    pub flash: (Color, f32, f32),
    pub shake: Vec2,
    rng: Rng,
    time: f32,
    /// Animations en cours (nuages de mort, etincelles) : plafonnees a part.
    n_anims: usize,
}

const NUM_LIFE: f32 = 0.55;
/// Plafonds : au-dela, l'ecran devient illisible (et rien n'est gagne en satisfaction).
const MAX_PARTICLES: usize = 900;
const MAX_NUMBERS: usize = 90;
const MAX_ANIMS: usize = 70;

impl Juice {
    pub fn new() -> Juice {
        Juice {
            particles: Vec::with_capacity(4096),
            numbers: Vec::with_capacity(512),
            rings: Vec::new(),
            ghosts: Vec::new(),
            trauma: 0.0,
            hitstop: 0.0,
            flash: (WHITE, 0.0, 1.0),
            shake: Vec2::ZERO,
            rng: Rng::new(7),
            time: 0.0,
            n_anims: 0,
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.n_anims = 0;
        self.numbers.clear();
        self.rings.clear();
        self.ghosts.clear();
        self.trauma = 0.0;
        self.hitstop = 0.0;
        self.flash.1 = 0.0;
    }

    pub fn add_trauma(&mut self, t: f32, max: f32) {
        if self.trauma < max {
            self.trauma = (self.trauma + t).min(max);
        }
    }

    pub fn hitstop(&mut self, secs: f32) {
        self.hitstop = self.hitstop.max(secs);
    }

    pub fn flash(&mut self, c: Color, dur: f32) {
        self.flash = (c, dur, dur);
    }

    pub fn anim(&mut self, a: AnimId, pos: Vec2, life: f32) {
        self.anim_rot(a, pos, life, 0.0);
    }

    pub fn anim_rot(&mut self, a: AnimId, pos: Vec2, life: f32, rot: f32) {
        self.anim_ex(a, pos, life, rot, 1.0, WHITE, false);
    }

    /// Animation ponctuelle avec rotation, echelle, teinte et mode additif.
    #[allow(clippy::too_many_arguments)]
    pub fn anim_ex(&mut self, a: AnimId, pos: Vec2, life: f32, rot: f32, scale: f32, color: Color, additive: bool) {
        if self.particles.len() >= MAX_PARTICLES || self.n_anims >= MAX_ANIMS {
            return;
        }
        self.n_anims += 1;
        self.particles.push(Particle {
            rot,
            scale,
            pos,
            vel: Vec2::ZERO,
            z: 0.0,
            vz: 0.0,
            gravity: 0.0,
            drag: 0.0,
            t: 0.0,
            life,
            kind: PKind::Anim(a),
            color,
            additive,
        });
    }

    /// Gerbe de debris colores (mort, impact).
    pub fn debris(&mut self, pos: Vec2, dir: Vec2, n: u32, color: Color, speed: f32, additive: bool) {
        if self.particles.len() >= MAX_PARTICLES * 3 / 4 {
            return;
        }
        for _ in 0..n {
            let d = (dir * 0.8 + self.rng.dir()).normalize_or_zero();
            let sp = speed * self.rng.range(0.4, 1.0);
            self.particles.push(Particle {
                rot: 0.0,
                scale: 1.0,
                pos,
                vel: d * sp,
                z: self.rng.range(2.0, 8.0),
                vz: self.rng.range(40.0, 110.0),
                gravity: 380.0,
                drag: 3.0,
                t: 0.0,
                life: self.rng.range(0.25, 0.5),
                kind: PKind::Pixel(if self.rng.chance(0.3) { 2.0 } else { 1.0 }),
                color,
                additive,
            });
        }
    }

    pub fn number(&mut self, pos: Vec2, value: f32, kind: NumKind) {
        if self.numbers.len() >= MAX_NUMBERS {
            // les petits chiffres de brulure cedent leur place en premier
            if kind == NumKind::Dot {
                return;
            }
            let k = self.numbers.iter().position(|n| n.kind == NumKind::Dot).unwrap_or(0);
            self.numbers.remove(k);
        }
        let jitter = Vec2::new(self.rng.range(-4.0, 4.0), self.rng.range(-3.0, 1.0));
        let vy = if kind == NumKind::Dot { -30.0 } else { -50.0 };
        self.numbers.push(Number { pos: pos + jitter, vy, t: 0.0, value: value.round().max(1.0) as u32, kind });
    }

    pub fn ring(&mut self, pos: Vec2, r0: f32, r1: f32, life: f32, color: Color) {
        self.rings.push(Ring { pos, t: 0.0, life, r0, r1, color });
    }

    pub fn ghost(&mut self, frame: Frame, pos: Vec2, flip: bool, life: f32, color: Color) {
        self.ghosts.push(Ghost { frame, pos, flip, t: 0.0, life, color });
    }

    /// Avance les effets. `dt` = temps de jeu (fige pendant les arrets sur image), `real` = temps reel.
    pub fn update(&mut self, dt: f32, real: f32) {
        self.time += real;
        self.trauma = (self.trauma - real * 1.6).max(0.0);
        let s = self.trauma * self.trauma;
        let t = self.time * 38.0;
        self.shake = Vec2::new((t * 1.13).sin() + (t * 2.71).sin() * 0.5, (t * 1.37).cos() + (t * 3.07).sin() * 0.5) * s * 6.0;
        self.flash.1 = (self.flash.1 - real).max(0.0);
        if dt <= 0.0 {
            return;
        }
        let mut n_anims = 0;
        self.particles.retain_mut(|p| {
            p.t += dt;
            p.vel *= (-p.drag * dt).exp();
            p.pos += p.vel * dt;
            if p.gravity > 0.0 {
                p.vz -= p.gravity * dt;
                p.z += p.vz * dt;
                if p.z < 0.0 {
                    p.z = 0.0;
                    p.vz = -p.vz * 0.3;
                    p.vel *= 0.5;
                }
            }
            let keep = p.t < p.life;
            if keep && matches!(p.kind, PKind::Anim(_)) {
                n_anims += 1;
            }
            keep
        });
        self.n_anims = n_anims;
        self.numbers.retain_mut(|n| {
            n.t += dt;
            n.pos.y += n.vy * dt;
            n.vy *= (-6.0 * dt).exp();
            n.t < NUM_LIFE
        });
        self.rings.retain_mut(|r| {
            r.t += dt;
            r.t < r.life
        });
        self.ghosts.retain_mut(|g| {
            g.t += dt;
            g.t < g.life
        });
    }

    /// Effets dessines sous les personnages (images remanentes).
    pub fn draw_under(&self, list: &mut DrawList) {
        list.set(Space::World, Blend::Add);
        for g in &self.ghosts {
            let a = 1.0 - g.t / g.life;
            let p = Params::default().flip(g.flip).color(with_alpha(g.color, a * 0.7));
            list.frame(&g.frame, g.pos, &p);
        }
    }

    /// Effets dessines au-dessus du monde.
    pub fn draw_over(&self, list: &mut DrawList, atlas: &Atlas, white: &Frame) {
        // particules normales
        list.set(Space::World, Blend::Alpha);
        for p in self.particles.iter().filter(|p| !p.additive) {
            self.draw_particle(list, atlas, white, p);
        }
        list.set(Space::World, Blend::Add);
        for p in self.particles.iter().filter(|p| p.additive) {
            self.draw_particle(list, atlas, white, p);
        }
        // anneaux (ondes de choc)
        for r in &self.rings {
            let u = r.t / r.life;
            let e = 1.0 - (1.0 - u) * (1.0 - u) * (1.0 - u);
            let rad = r.r0 + (r.r1 - r.r0) * e;
            let a = (1.0 - u) * r.color[3];
            let segs = ((rad * 0.55) as usize).clamp(12, 96);
            let thick = 3.0 * (1.0 - u) + 1.0;
            for k in 0..segs {
                let ang = k as f32 / segs as f32 * std::f32::consts::TAU;
                let d = Vec2::from_angle(ang);
                let pos = r.pos + d * rad - Vec2::splat(thick * 0.5);
                list.rect(white, pos, Vec2::splat(thick), with_alpha(r.color, a));
            }
        }
    }

    fn draw_particle(&self, list: &mut DrawList, atlas: &Atlas, white: &Frame, p: &Particle) {
        let pos = p.pos - Vec2::new(0.0, p.z);
        match p.kind {
            PKind::Anim(a) => {
                let f = atlas.frame(a, p.t);
                let prm = Params::default().color(p.color).rot(p.rot).scale(Vec2::splat(p.scale));
                list.frame(f, pos, &prm);
            }
            PKind::Pixel(sz) => {
                let a = (1.0 - p.t / p.life).clamp(0.0, 1.0);
                list.rect(white, pos.round(), Vec2::splat(sz), with_alpha(p.color, a));
            }
        }
    }

    /// Chiffres de degats (police HUD), au-dessus de tout le monde.
    pub fn draw_numbers(&self, list: &mut DrawList, atlas: &Atlas) {
        let font = atlas.font("hud");
        list.set(Space::World, Blend::Alpha);
        for n in &self.numbers {
            let u = n.t / NUM_LIFE;
            // les critiques sautent aux yeux (x3 un instant puis x2), les autres restent discrets
            let sc: f32 = match n.kind {
                NumKind::Crit | NumKind::Boss if n.t < 0.06 => 3.0,
                NumKind::Crit | NumKind::Boss => 2.0,
                _ => 1.0,
            };
            let a = if u > 0.7 { 1.0 - (u - 0.7) / 0.3 } else { 1.0 };
            let col = match n.kind {
                NumKind::Crit => [1.0, 0.85, 0.2, a],
                NumKind::Dot => [1.0, 0.6, 0.25, a * 0.85],
                NumKind::Boss => [1.0, 0.55, 0.5, a],
                NumKind::Normal => [1.0, 1.0, 1.0, a],
            };
            let txt = n.value.to_string();
            let w = font.measure(&txt) as f32 * sc;
            let mut x = n.pos.x - w * 0.5;
            let y = n.pos.y - 7.0 * sc;
            for ch in txt.chars() {
                if let Some(g) = font.glyph(ch) {
                    let f = Frame { x: g.x, y: g.y, w: g.w, h: g.h, px: 0, py: 0 };
                    let gy = y + g.oy as f32 * sc;
                    // contour sombre puis chiffre
                    let shadow = Params::default().scale(Vec2::splat(sc)).color([0.1, 0.05, 0.1, a * 0.9]);
                    list.frame(&f, Vec2::new(x + sc, gy + sc), &shadow);
                    list.frame(&f, Vec2::new(x, gy), &Params::default().scale(Vec2::splat(sc)).color(col));
                    x += (g.w as f32 + font.spacing as f32 + 1.0) * sc;
                }
            }
        }
    }
}
