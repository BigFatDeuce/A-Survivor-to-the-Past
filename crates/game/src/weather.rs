//! Meteo des stages (rendu seul, sans effet sur la simulation) : pluie et eclairs, brume et
//! ombre du feuillage, neige, vent de sable, obscurite autour de Link. Et la porte de sortie.
use assetkit::meta::Frame;
use engine::assets::AnimId;
use engine::app::Ctx;
use engine::draw::{Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::defs::Weather;

pub struct WeatherVis {
    pub drop: AnimId,
    pub splash: AnimId,
    pub fog: AnimId,
    pub canopy: AnimId,
    pub spark: AnimId,
}

impl WeatherVis {
    pub fn new(atlas: &engine::assets::Atlas) -> WeatherVis {
        WeatherVis {
            drop: atlas.get("fx_rain_drop", "0"),
            splash: atlas.get("fx_rain_splash", "0"),
            fog: atlas.get("fx_fog", "0"),
            canopy: atlas.get("fx_canopy", "0"),
            spark: atlas.get("fx_spark", "0"),
        }
    }
}

#[inline]
fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9e37_79b1) ^ b.wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xc2b2_ae3d);
    h ^ (h >> 13)
}

/// Premier eclair a 6 s puis toutes les 8 a 16 s (temps de partie).
const STRIKE_FIRST: f32 = 6.0;

/// Eclair le plus recent avant `t` : (numero, instant).
pub fn strike_before(t: f32) -> Option<(u32, f32)> {
    if t < STRIKE_FIRST {
        return None;
    }
    let mut at = STRIKE_FIRST;
    let mut k = 0u32;
    loop {
        let next = at + 8.0 + (hash(k, 77) % 800) as f32 / 100.0;
        if next > t {
            return Some((k, at));
        }
        at = next;
        k += 1;
    }
}

/// Intensite de l'eclair (double eclat qui retombe).
fn flash(t: f32) -> f32 {
    let Some((_, at)) = strike_before(t) else { return 0.0 };
    let d = t - at;
    let a = if d < 0.08 { 1.0 } else if d < 0.16 { 0.25 } else if d < 0.24 { 0.8 } else { 0.0 };
    let tail = if d >= 0.24 { (1.0 - (d - 0.24) / 0.5).max(0.0) * 0.5 } else { 0.0 };
    a + tail
}

/// Dessine la meteo par-dessus le monde (espace interface). `cam` : centre de la camera dans le
/// monde ; `player` : position de Link a l'ecran (pour le halo de l'obscurite) ; `t` : temps.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    list: &mut DrawList,
    ctx: &Ctx,
    v: &WeatherVis,
    white: &Frame,
    weather: Weather,
    tint: Option<(f32, f32, f32, f32)>,
    cam: Vec2,
    player: Vec2,
    t: f32,
) {
    let view = ctx.view;
    list.set(Space::Ui, Blend::Alpha);
    if let Some((r, g, b, a)) = tint {
        list.rect(white, Vec2::ZERO, view, [r, g, b, a]);
    }
    match weather {
        Weather::Clear => {}
        Weather::Rain => {
            list.rect(white, Vec2::ZERO, view, [0.03, 0.05, 0.14, 0.22]);
            rain(list, ctx, v, view, t);
            let f = flash(t);
            if f > 0.0 {
                list.set(Space::Ui, Blend::Add);
                list.rect(white, Vec2::ZERO, view, [0.7, 0.75, 1.0, 0.25 * f]);
            }
        }
        Weather::Fog => {
            // ombre du feuillage accrochee au sol, puis deux couches de brume qui derivent
            layer(list, ctx, v.canopy, view, cam, Vec2::ZERO, 1.0, [1.0, 1.0, 1.0, 0.28]);
            layer(list, ctx, v.fog, view, cam * 0.8, Vec2::new(t * 7.0, t * 2.5), 1.0, [0.85, 0.9, 0.85, 0.09]);
            layer(list, ctx, v.fog, view, cam * 1.1, Vec2::new(-t * 4.0, t * 1.5 + 90.0), 2.0, [0.85, 0.9, 0.85, 0.06]);
        }
        Weather::Snow => {
            list.rect(white, Vec2::ZERO, view, [0.75, 0.85, 1.0, 0.08]);
            snow(list, white, view, cam, t);
        }
        Weather::Sand => {
            layer(list, ctx, v.fog, view, cam, Vec2::new(-t * 70.0, t * 6.0), 2.0, [0.85, 0.7, 0.45, 0.15]);
            sand(list, white, view, t);
        }
        Weather::Dark => darkness(list, white, view, player, t),
    }
    list.set(Space::Ui, Blend::Alpha);
}

/// Texture 256x256 repetee sur tout l'ecran (decalage = position monde + derive).
#[allow(clippy::too_many_arguments)]
fn layer(list: &mut DrawList, ctx: &Ctx, anim: AnimId, view: Vec2, cam: Vec2, drift: Vec2, scale: f32, color: [f32; 4]) {
    let f = ctx.atlas.frame(anim, 0.0);
    let size = 256.0 * scale;
    let off = cam - view * 0.5 + drift;
    let ox = -(off.x.rem_euclid(size));
    let oy = -(off.y.rem_euclid(size));
    let p = Params::default().scale(Vec2::splat(scale)).color(color);
    let mut y = oy;
    while y < view.y {
        let mut x = ox;
        while x < view.x {
            list.frame(f, Vec2::new(x, y).round(), &p);
            x += size;
        }
        y += size;
    }
}

fn rain(list: &mut DrawList, ctx: &Ctx, v: &WeatherVis, view: Vec2, t: f32) {
    const SPEED: f32 = 300.0;
    const SLOPE: f32 = 0.5;
    const SPLASH: f32 = 0.25;
    let n = (view.x * view.y / 260.0) as u32;
    let drop = ctx.atlas.frame(v.drop, 0.0);
    list.set(Space::Ui, Blend::Add);
    for i in 0..n {
        let cycle = 0.7 + (hash(i, 7) % 500) as f32 / 1000.0;
        let tt = t + (hash(i, 3) % 1000) as f32 / 1000.0 * cycle;
        let c = (tt / cycle).floor() as u32;
        let local = tt - c as f32 * cycle;
        let h = hash(c, i.wrapping_mul(31).wrapping_add(5));
        let fall = 0.22 + (h % 100) as f32 / 100.0 * 0.3;
        let reach = fall * SPEED;
        let gx = ((h >> 7) % 10_000) as f32 / 10_000.0 * (view.x + reach * SLOPE) - 4.0;
        let gy = ((h >> 14) % 10_000) as f32 / 10_000.0 * (view.y + 8.0);
        if local < fall {
            let d = (fall - local) * SPEED;
            list.frame(drop, Vec2::new(gx + d * SLOPE, gy - d).round(), &Params::default().color([0.85, 0.92, 1.0, 0.6]));
        } else if local < fall + SPLASH {
            let f = ctx.atlas.frame(v.splash, local - fall);
            list.frame(f, Vec2::new(gx, gy).round(), &Params::default().color([0.85, 0.92, 1.0, 0.5]));
        }
    }
    list.set(Space::Ui, Blend::Alpha);
}

fn snow(list: &mut DrawList, white: &Frame, view: Vec2, cam: Vec2, t: f32) {
    // flocons sur trois plans (les plus proches, plus gros et plus rapides)
    let n = (view.x * view.y / 900.0) as u32;
    for i in 0..n {
        let h = hash(i, 19);
        let depth = 1 + h % 3;
        let sp = 12.0 + depth as f32 * 10.0;
        let par = 0.3 * depth as f32;
        let w = view.x + 40.0;
        let hh = view.y + 40.0;
        let x0 = (h >> 3) % 10_000;
        let y0 = (h >> 13) % 10_000;
        let sway = (t * (0.6 + (h % 7) as f32 * 0.1) + i as f32).sin() * 6.0;
        let x = (x0 as f32 / 10_000.0 * w - cam.x * par + sway + t * 8.0).rem_euclid(w) - 20.0;
        let y = (y0 as f32 / 10_000.0 * hh - cam.y * par + t * sp).rem_euclid(hh) - 20.0;
        let s = if depth == 3 { 2.0 } else { 1.0 };
        let a = 0.45 + 0.18 * depth as f32;
        list.rect(white, Vec2::new(x, y).round(), Vec2::splat(s), [1.0, 1.0, 1.0, a]);
    }
}

fn sand(list: &mut DrawList, white: &Frame, view: Vec2, t: f32) {
    // trainees de sable poussees par le vent (de droite a gauche)
    let n = (view.x * view.y / 1400.0) as u32;
    for i in 0..n {
        let h = hash(i, 41);
        let sp = 160.0 + (h % 120) as f32;
        let len = 4.0 + (h % 9) as f32;
        let w = view.x + 60.0;
        let x = w - ((h >> 4) as f32 % w + t * sp).rem_euclid(w) - 30.0;
        let y = ((h >> 12) % 10_000) as f32 / 10_000.0 * view.y + (t * 2.0 + i as f32).sin() * 3.0;
        list.rect(white, Vec2::new(x, y).round(), Vec2::new(len, 1.0), [0.95, 0.82, 0.55, 0.55]);
    }
}

/// Tenebres : seul un halo autour de Link reste visible (bords adoucis par couches).
fn darkness(list: &mut DrawList, white: &Frame, view: Vec2, player: Vec2, t: f32) {
    let base = 92.0 + (t * 3.1).sin() * 2.0;
    let layers = [(1.0, 0.30), (1.18, 0.25), (1.38, 0.22), (1.6, 0.18)];
    let step = 2.0;
    for (k, a) in layers {
        let r = base * k;
        let col = [0.02, 0.0, 0.06, a];
        let mut y = 0.0;
        while y < view.y {
            let dy = y + step * 0.5 - player.y;
            if dy.abs() >= r {
                list.rect(white, Vec2::new(0.0, y), Vec2::new(view.x, step), col);
            } else {
                let half = (r * r - dy * dy).sqrt();
                let l = (player.x - half).max(0.0);
                let rr = (player.x + half).min(view.x);
                if l > 0.0 {
                    list.rect(white, Vec2::new(0.0, y), Vec2::new(l, step), col);
                }
                if rr < view.x {
                    list.rect(white, Vec2::new(rr, y), Vec2::new(view.x - rr, step), col);
                }
            }
            y += step;
        }
    }
}

/// Porte de sortie (monde) : colonne de lumiere, anneau au sol et etincelles qui montent.
pub fn draw_portal(list: &mut DrawList, ctx: &Ctx, v: &WeatherVis, white: &Frame, at: Vec2, age: f32, t: f32) {
    // ouverture douce : l'anneau s'ouvre au sol, puis la colonne monte (courbe en S)
    let u = (age / 1.6).clamp(0.0, 1.0);
    let open = u * u * (3.0 - 2.0 * u);
    let rise = ((age - 0.3) / 1.3).clamp(0.0, 1.0);
    let rise = rise * rise * (3.0 - 2.0 * rise);
    list.set(Space::World, Blend::Add);
    // colonne
    let w = 18.0 * open;
    let hgt = 96.0 * rise.max(0.01);
    let rows = 24;
    for k in 0..rows {
        let u = k as f32 / rows as f32;
        let a = (1.0 - u) * 0.35 * open * (0.85 + 0.15 * (t * 6.0 + u * 8.0).sin());
        let ww = w * (1.0 - u * 0.4);
        let y = at.y - u * hgt;
        list.rect(white, Vec2::new(at.x - ww * 0.5, y - hgt / rows as f32).round(), Vec2::new(ww, hgt / rows as f32 + 1.0), [0.55, 1.0, 0.6, a]);
        list.rect(white, Vec2::new(at.x - ww * 0.18, y - hgt / rows as f32).round(), Vec2::new(ww * 0.36, hgt / rows as f32 + 1.0), [1.0, 1.0, 0.9, a]);
    }
    // anneau au sol qui tourne
    let segs = 20;
    for k in 0..segs {
        let a = k as f32 / segs as f32 * std::f32::consts::TAU + t * 2.0;
        let p = at + Vec2::new(a.cos() * 16.0, a.sin() * 6.0) * open;
        list.rect(white, p.round(), Vec2::splat(2.0), [0.7, 1.0, 0.7, 0.8]);
    }
    // etincelles
    let f = ctx.atlas.frame(v.spark, 0.0);
    for k in 0..6u32 {
        let c = 1.4;
        let local = (t + k as f32 * c / 6.0).rem_euclid(c) / c;
        let h = hash(k, ((t + k as f32 * c / 6.0) / c) as u32);
        let x = at.x + ((h % 100) as f32 / 100.0 - 0.5) * 22.0;
        let y = at.y - local * 80.0;
        list.frame(f, Vec2::new(x, y).round(), &Params::default().color([0.8, 1.0, 0.8, (1.0 - local) * open]));
    }
    list.set(Space::World, Blend::Alpha);
    let _ = WHITE;
}
