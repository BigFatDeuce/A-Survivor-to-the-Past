//! Decor procedural d'une carte : arbres, rochers, buissons... places sans etat (fonction de la
//! case et de la graine), donc identiques pour la simulation et le rendu, sans limite de taille.
//! Les elements « solides » bloquent le joueur et la horde (glissement le long de l'obstacle).
use crate::defs::PropDef;
use glam::Vec2;

/// Taille d'une case de placement (pixels) : au plus un element par case.
pub const CELL: f32 = 48.0;
/// Aucun element pres du point de depart.
const SAFE: f32 = 110.0;

#[derive(Clone, Copy, Debug)]
pub struct Prop {
    /// Indice dans `MapDef::props`.
    pub kind: u16,
    pub pos: Vec2,
    /// Rayon de collision (0 = traversable).
    pub radius: f32,
    /// Variante (choix d'image pour le rendu).
    pub var: u32,
}

#[inline]
fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

/// Element de la case (cx, cy), s'il y en a un.
pub fn prop_at(defs: &[PropDef], seed: u32, cx: i32, cy: i32) -> Option<Prop> {
    if defs.is_empty() {
        return None;
    }
    let h = hash(cx, cy, seed);
    let u = (h & 0xffff) as f32 / 65_536.0;
    let mut acc = 0.0;
    let mut kind = None;
    for (k, d) in defs.iter().enumerate() {
        acc += d.density;
        if u < acc {
            kind = Some(k);
            break;
        }
    }
    let k = kind?;
    let d = &defs[k];
    let h2 = hash(cy, cx, seed ^ 0x51ed);
    // position dans la case, en gardant l'element entier dans sa case (pas de chevauchement)
    let m = d.radius.min(CELL * 0.4);
    let jx = (h2 & 0xff) as f32 / 255.0;
    let jy = ((h2 >> 8) & 0xff) as f32 / 255.0;
    let pos = Vec2::new(cx as f32 * CELL + m + jx * (CELL - 2.0 * m), cy as f32 * CELL + m + jy * (CELL - 2.0 * m));
    if pos.length() < SAFE + d.radius {
        return None;
    }
    Some(Prop { kind: k as u16, pos, radius: if d.solid { d.radius } else { 0.0 }, var: h2 >> 16 })
}

/// Appelle `f` pour chaque element dont la case touche le rectangle [lo, hi].
pub fn for_each(defs: &[PropDef], seed: u32, lo: Vec2, hi: Vec2, mut f: impl FnMut(Prop)) {
    if defs.is_empty() {
        return;
    }
    let (x0, y0) = ((lo.x / CELL).floor() as i32, (lo.y / CELL).floor() as i32);
    let (x1, y1) = ((hi.x / CELL).floor() as i32, (hi.y / CELL).floor() as i32);
    for cy in y0..=y1 {
        for cx in x0..=x1 {
            if let Some(p) = prop_at(defs, seed, cx, cy) {
                f(p);
            }
        }
    }
}

/// Repousse un cercle (centre `pos`, rayon `r`) hors des elements solides voisins.
/// `toward` : direction voulue ; `side` : sens de contournement quand on bute de face (+1 / -1).
pub fn resolve(defs: &[PropDef], seed: u32, pos: Vec2, r: f32, toward: Vec2, side: f32) -> Vec2 {
    if defs.is_empty() {
        return pos;
    }
    let (cx, cy) = ((pos.x / CELL).floor() as i32, (pos.y / CELL).floor() as i32);
    let mut p = pos;
    for oy in -1..=1 {
        for ox in -1..=1 {
            let Some(o) = prop_at(defs, seed, cx + ox, cy + oy) else { continue };
            if o.radius <= 0.0 {
                continue;
            }
            let d = p - o.pos;
            let min = o.radius + r;
            let l2 = d.length_squared();
            if l2 >= min * min {
                continue;
            }
            let l = l2.sqrt().max(0.001);
            let n = d / l;
            p = o.pos + n * min;
            // de face : petit pas de cote pour contourner au lieu de rester colle
            let t = n.perp();
            let along = toward.dot(t);
            let s = if along.abs() < 0.2 { side } else { along.signum() };
            p += t * s * 0.6;
        }
    }
    p
}

/// Vrai si le point est libre (hors des elements solides), avec une marge.
pub fn is_free(defs: &[PropDef], seed: u32, pos: Vec2, r: f32) -> bool {
    resolve(defs, seed, pos, r, Vec2::ZERO, 1.0).distance_squared(pos) < 0.01
}
