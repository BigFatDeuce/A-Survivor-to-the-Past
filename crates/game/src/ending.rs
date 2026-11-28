//! Fin du jeu (1.0) : premiere victoire sur Ganon. Mort epique (plein ecran, partie figee), puis au
//! portail de sortie le generique (les personnages du jeu courent de droite a gauche en bas de
//! l'ecran), et la fee felicite le joueur au retour sur l'ecran titre (`fairy_tip::Talk::Victory`).
use crate::hub::SOFT;
use crate::hud::{text, wrap, GOLD};
use crate::lang::Lang;
use assetkit::meta::Frame;
use engine::app::Ctx;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::defs::Content;
use std::f32::consts::TAU;

/// Duree de la mort de Ganon (s), et debut du blanc total.
pub const DEATH: f32 = 6.0;
const WHITE_AT: f32 = 3.7;
/// Explosions par seconde autour de Ganon, jusqu'au blanc.
pub const BLASTS: f32 = 7.0;

/// Sons de la mort de Ganon : (instant, son, volume, hauteur).
pub const DEATH_SFX: [(f32, &str, f32, f32); 17] = [
    (0.0, "boss_die", 1.0, 0.8),
    (0.0, "quake", 1.0, 1.0),
    (0.05, "screen_rumble", 1.0, 0.9),
    (0.4, "explosion", 0.9, 0.9),
    (0.75, "explosion", 0.9, 1.1),
    (1.05, "bomb", 0.9, 0.8),
    (1.4, "explosion", 0.9, 1.0),
    (1.7, "explosion", 1.0, 0.85),
    (2.0, "bomb", 0.9, 1.2),
    (2.3, "bombos", 1.0, 1.0),
    (2.7, "explosion", 1.0, 0.75),
    (3.0, "explosion", 1.0, 1.0),
    (3.3, "ether", 0.9, 0.9),
    (3.6, "storm_thunder", 1.0, 1.0),
    (3.7, "screen_clear", 1.0, 0.8),
    (3.75, "boss_die", 1.0, 0.6),
    (4.6, "victory", 0.8, 1.0),
];

/// Nombre pseudo-aleatoire stable dans [0, 1).
fn hash(x: u32) -> f32 {
    let mut v = x.wrapping_mul(2_654_435_761) ^ 0x9e37_79b9;
    v ^= v >> 15;
    v = v.wrapping_mul(2_246_822_519);
    v ^= v >> 13;
    (v & 0xffff) as f32 / 65536.0
}

/// Decalage (monde) de la k-ieme explosion autour de Ganon : de plus en plus large.
pub fn blast_offset(k: u32) -> Vec2 {
    let a = hash(k * 2) * TAU;
    let r = 8.0 + hash(k * 2 + 1) * (18.0 + k as f32 * 2.5);
    Vec2::new(a.cos() * r, a.sin() * r * 0.7 - 16.0)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Plein ecran pendant la mort de Ganon (`at` : Ganon, en pixels d'interface) : voile rouge sombre,
/// rayons de lumiere qui tournent et grandissent, blanc total, puis « Ganon est vaincu ! ».
pub fn draw_death(list: &mut DrawList, ctx: &Ctx, lang: &Lang, white: &Frame, at: Vec2, t: f32) {
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    let veil = smooth(0.0, 1.0, t) * (1.0 - smooth(WHITE_AT, WHITE_AT + 0.4, t));
    list.rect(white, Vec2::ZERO, view, [0.22, 0.0, 0.06, 0.5 * veil]);
    // rayons de lumiere depuis Ganon (additifs), de plus en plus longs, qui tournent
    list.set(Space::Ui, Blend::Add);
    let grow = smooth(0.2, 3.2, t);
    let fade = 1.0 - smooth(WHITE_AT + 0.2, WHITE_AT + 0.6, t);
    let len = view.length() * (0.15 + 0.95 * grow);
    for k in 0..20 {
        let a = k as f32 / 20.0 * TAU + t * (0.5 + 0.4 * grow) + hash(k) * 0.2;
        let flick = 0.6 + 0.4 * (t * 23.0 + k as f32 * 1.7).sin().abs();
        let w = (2.0 + 6.0 * grow * hash(k + 40)).round();
        let c: Color = if k % 2 == 0 { GOLD } else { rgb(255, 250, 235) };
        list.frame(white, at, &Params::default().scale(Vec2::new(len, w)).rot(a).color(with_alpha(c, 0.32 * flick * fade * smooth(0.05, 0.4, t))));
    }
    list.set(Space::Ui, Blend::Alpha);
    // eclair blanc au coup final, blanc total, retour
    let first = 1.0 - smooth(0.0, 0.25, t);
    let out = smooth(WHITE_AT - 0.3, WHITE_AT, t) * (1.0 - smooth(WHITE_AT + 0.5, DEATH - 0.3, t));
    let a = first.max(out);
    if a > 0.0 {
        list.rect(white, Vec2::ZERO, view, [1.0, 1.0, 1.0, a]);
    }
    // titre
    let ta = smooth(WHITE_AT + 0.4, WHITE_AT + 0.8, t) * (1.0 - smooth(DEATH - 0.5, DEATH, t));
    if ta > 0.0 {
        let y = (view.y * 0.3).round();
        text(list, main, lang.get("ending.ganon_down"), Vec2::new((view.x * 0.5).round(), y), 3.0, with_alpha(GOLD, ta), Align::Center);
    }
}

/// Une ligne du generique (position dans le rouleau).
struct Line {
    y: f32,
    x: f32,
    text: String,
    scale: f32,
    color: Color,
}

/// Generique : `data/credits.ron` (titres traduits, noms), mis en page pour une largeur de vue.
pub struct Credits {
    lines: Vec<Line>,
    /// Defilement ou le message de fin est au centre de l'ecran (le rouleau s'y arrete).
    pub stop: f32,
    width: f32,
}

/// Sections du fichier : (cle du titre, noms, colonnes).
type Sections = Vec<(String, Vec<String>, usize)>;

/// Vitesse du rouleau (pixels d'interface par seconde).
pub const SPEED: f32 = 26.0;
/// Hauteur reservee en bas au defile des personnages.
fn parade_h(view: Vec2) -> f32 {
    (view.y * 0.3).round()
}

impl Credits {
    pub fn new(ctx: &Ctx, lang: &Lang) -> Credits {
        let view = ctx.view;
        let main = ctx.atlas.font("main");
        let sections: Sections = engine::vfs::read_string(&ctx.root.join("data/credits.ron"))
            .and_then(|t| ron::from_str(&t).map_err(|e| log::warn!("credits.ron : {e}")).ok())
            .unwrap_or_default();
        let mut lines = Vec::new();
        let cx = (view.x * 0.5).round();
        let mut y = 0.0;
        let mut push = |y: f32, x: f32, s: &str, scale: f32, color: Color| lines.push(Line { y, x, text: s.to_string(), scale, color });
        push(y, cx, "A Survivor to the Past", 3.0, GOLD);
        y += 54.0;
        push(y, cx, lang.get("credits.by"), 1.0, SOFT);
        y += 16.0;
        push(y, cx, "Deuce (Holy C)", 2.0, WHITE);
        y += 64.0;
        for (key, names, cols) in &sections {
            push(y, cx, lang.get(key), 1.0, GOLD);
            y += 22.0;
            let cols = (*cols).clamp(1, 3);
            let col_w = (view.x - 40.0).min(420.0) / cols as f32;
            let x0 = cx - col_w * cols as f32 * 0.5 + col_w * 0.5;
            for (i, n) in names.iter().enumerate() {
                let c = i % cols;
                push(y, (x0 + col_w * c as f32).round(), n, 1.0, WHITE);
                if c + 1 == cols || i + 1 == names.len() {
                    y += 15.0;
                }
            }
            y += 40.0;
        }
        // mot de l'auteur, centre a l'arret du rouleau
        y += 30.0;
        let top = y;
        for l in wrap(main, lang.get("credits.thanks"), (view.x - 48.0).min(400.0), 1.0) {
            push(y, cx, &l, 1.0, WHITE);
            y += 16.0;
        }
        y += 10.0;
        push(y, cx, lang.get("credits.sign"), 1.0, GOLD);
        y += 40.0;
        push(y, cx, lang.get("credits.end"), 2.0, GOLD);
        y += 24.0;
        let area = view.y - parade_h(view);
        // ligne a l'ecran = zone + 8 - defilement + y : le message de fin s'arrete au milieu de la zone
        let stop = ((top + y) * 0.5 + area * 0.5 + 8.0).max(0.0);
        Credits { lines, stop, width: view.x }
    }

    /// Mise en page faite pour une autre largeur (fenetre redimensionnee).
    pub fn stale(&self, ctx: &Ctx) -> bool {
        self.width != ctx.view.x
    }
}

/// Generique : ciel etoile, rouleau de noms (fondu en haut et en bas de sa zone), personnages qui
/// courent de droite a gauche en bas. `scroll` : defilement du rouleau ; `t` : temps ecoule.
#[allow(clippy::too_many_arguments)]
pub fn draw_credits(list: &mut DrawList, ctx: &Ctx, lang: &Lang, content: &Content, roster: &[String], white: &Frame, c: &Credits, t: f32, scroll: f32, done: bool) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 1.0]);
    for k in 0..70u32 {
        let p = Vec2::new((hash(k * 3) * view.x).round(), (hash(k * 3 + 1) * view.y * 0.8).round());
        let tw = 0.35 + 0.35 * (t * (0.8 + hash(k * 3 + 2) * 2.0) + k as f32).sin();
        list.rect(white, p, Vec2::ONE, with_alpha(WHITE, tw));
    }
    // rouleau
    let area = view.y - parade_h(view);
    let base = area + 8.0 - scroll;
    for l in &c.lines {
        let y = (base + l.y).round();
        let h = 12.0 * l.scale;
        if y + h < 0.0 || y > area {
            continue;
        }
        let a = smooth(-4.0, 26.0, y) * (1.0 - smooth(area - 30.0, area - 6.0, y + h));
        if a > 0.0 && !l.text.is_empty() {
            text(list, main, &l.text, Vec2::new(l.x, y), l.scale, with_alpha(l.color, l.color[3] * a), Align::Center);
        }
    }
    // sol et defile : 4 couloirs, les plus lointains plus sombres et plus lents (parallaxe)
    let ground = view.y - parade_h(view);
    list.rect(white, Vec2::new(0.0, ground), Vec2::new(view.x, view.y - ground), [0.03, 0.05, 0.1, 1.0]);
    list.rect(white, Vec2::new(0.0, ground), Vec2::new(view.x, 1.0), with_alpha(GOLD, 0.35));
    if !roster.is_empty() {
        let lanes = 4;
        for lane in 0..lanes {
            let depth = lane as f32 / (lanes - 1) as f32; // 0 = au fond
            let y = (ground + 26.0 + depth * (view.y - ground - 34.0)).round();
            let v = 55.0 + 45.0 * depth;
            let period = 1.25 - 0.3 * depth + hash(lane * 7) * 0.3;
            let travel = (view.x + 60.0) / v;
            let n1 = (t / period).floor() as i64;
            let n0 = ((t - travel) / period).floor().max(0.0) as i64;
            let shade = 0.45 + 0.55 * depth;
            for n in n0..=n1 {
                let born = n as f32 * period;
                let x = (view.x + 30.0 - (t - born) * v).round();
                let pick = (hash(lane * 1000 + n as u32) * roster.len() as f32) as usize % roster.len();
                let Some(def) = content.characters.get(&roster[pick]) else { continue };
                let Some(anim) = atlas.find(&def.sprite, "walk_side") else { continue };
                let f = atlas.frame(anim, t * 1.6 + hash(n as u32) * 2.0);
                let prm = Params::default().flip(!def.side_faces_left).color([shade, shade, shade, 1.0]);
                list.frame(f, Vec2::new(x, y), &prm);
            }
        }
    }
    // aide
    let hint = lang.get(if done { "credits.hint_end" } else { "credits.hint" });
    if (t * 1.2).fract() < 0.8 || !done {
        text(list, main, hint, Vec2::new(view.x - 8.0, view.y - 14.0), 1.0, with_alpha(SOFT, if done { 0.9 } else { 0.5 }), Align::Right);
    }
}
