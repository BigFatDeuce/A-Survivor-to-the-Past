//! Diseuse de bonne aventure (0.6.3) : apparait a la place du grand coffre du boss final. Deux jeux,
//! une fois chacun, pendant que la partie (et son chrono) est en pause :
//! - jeu des coffres : une mise prise dans la banque, trois coffres (x3, x1, rien) ;
//! - quitte ou double : les rubis que la partie mettra en banque sont doubles... ou divises par deux.
use crate::hud::{frame_rect, text, GOLD};
use crate::lang::Lang;
use crate::menus::{NavIn, OfferInfo, DIM};
use crate::visuals::Vis;
use engine::app::Ctx;
use engine::assets::AnimId;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;

/// Mises possibles du jeu des coffres (rubis de la banque).
pub const STAKES: [u64; 3] = [100, 500, 2000];
/// Contenu des trois coffres, melange a chaque partie : esperance x1,33.
pub const CHEST_MULTS: [u32; 3] = [3, 1, 0];
/// Quitte ou double : une chance sur deux (x2 ou x0,5, esperance x1,25).
pub const DOUBLE_WIN: f32 = 0.5;
/// Delai avant d'accepter une touche apres un resultat.
pub const RESULT_LOCK: f32 = 1.0;
pub const PURPLE: Color = rgb(200, 150, 255);

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Choix du jeu.
    Menu,
    /// Jeu des coffres : choix de la mise.
    Stake,
    /// Trois coffres fermes.
    Chests { stake: u64 },
    /// Coffres ouverts : contenu, coffre choisi.
    Opened { stake: u64, mults: [u32; 3], pick: usize },
    /// Quitte ou double : la boule de cristal s'eclaire, puis le resultat.
    Double { win: bool },
}

fn card(icon: AnimId, name: String, tag: String, ok: bool, desc: String) -> OfferInfo {
    OfferInfo { icon, name, tag, is_new: ok, desc, level: 0, max: 0, evo: Vec::new(), evo_label: String::new(), keep: false, fusion: 0 }
}

/// Cartes du menu : jeu des coffres, quitte ou double, partir. `played` : jeux deja faits.
pub fn menu_infos(vis: &Vis, lang: &Lang, played: [bool; 2], run_bank: u64) -> Vec<OfferInfo> {
    let tag = |done: bool| if done { lang.get("ui.fortune.played").to_string() } else { String::new() };
    vec![
        card(vis.fortune_chest[0], lang.get("ui.fortune.chests").to_string(), tag(played[0]), false, lang.get("ui.fortune.chests_desc").to_string()),
        card(
            vis.icon_rupees,
            lang.get("ui.fortune.double").to_string(),
            tag(played[1]),
            false,
            lang.get("ui.fortune.double_desc").replace("{n}", &run_bank.to_string()),
        ),
        card(vis.ui_arrow, lang.get("ui.fortune.leave").to_string(), String::new(), false, lang.get("ui.fortune.leave_desc").to_string()),
    ]
}

/// Cartes des mises (grisees si la banque ne suffit pas), puis Retour.
pub fn stake_infos(vis: &Vis, lang: &Lang, bank: u64) -> Vec<OfferInfo> {
    let mut out: Vec<OfferInfo> = STAKES
        .iter()
        .map(|&s| {
            let ok = bank >= s;
            let tag = if ok { lang.get("ui.fortune.bank").replace("{n}", &bank.to_string()) } else { lang.get("ui.fortune.too_poor").to_string() };
            card(
                vis.rupees[2],
                lang.get("ui.fortune.stake").replace("{n}", &s.to_string()),
                tag,
                ok,
                lang.get("ui.fortune.stake_desc").replace("{n}", &(s * CHEST_MULTS[0] as u64).to_string()),
            )
        })
        .collect();
    out.push(card(vis.ui_arrow, lang.get("ui.fortune.back").to_string(), String::new(), false, lang.get("ui.fortune.back_desc").to_string()));
    out
}

/// Gain net du coffre choisi (negatif : mise perdue).
pub fn chest_gain(stake: u64, mult: u32) -> i64 {
    stake as i64 * mult as i64 - stake as i64
}

const CHEST_SCALE: f32 = 4.0;

/// Coffres : centre de chacun, sur une rangee.
fn chest_centers(view: Vec2) -> [Vec2; 3] {
    let gap = (view.x / 4.0).min(130.0).round();
    let y = (view.y * 0.55).round();
    [-1.0, 0.0, 1.0].map(|k| Vec2::new((view.x * 0.5 + k * gap).round(), y))
}

fn chest_box(c: Vec2) -> (Vec2, Vec2) {
    let s = Vec2::splat(16.0 * CHEST_SCALE + 12.0);
    (c - s * 0.5, s)
}

/// Choix d'un coffre : gauche/droite, souris, chiffres 1-3.
pub fn input_chests(nav: &NavIn, view: Vec2, sel: &mut usize, t: f32) -> (Option<usize>, bool) {
    let mut moved = false;
    if nav.left && *sel > 0 {
        *sel -= 1;
        moved = true;
    }
    if nav.right && *sel < 2 {
        *sel += 1;
        moved = true;
    }
    let boxes = chest_centers(view).map(chest_box);
    if let Some(m) = nav.hover {
        for (k, (p, s)) in boxes.iter().enumerate() {
            if m.x >= p.x && m.y >= p.y && m.x < p.x + s.x && m.y < p.y + s.y && *sel != k {
                *sel = k;
                moved = true;
            }
        }
    }
    if t < crate::menus::LEVELUP_LOCK {
        return (None, moved);
    }
    if let Some(d) = nav.digit.filter(|&d| d < 3) {
        return (Some(d), moved);
    }
    if nav.click {
        for (k, (p, s)) in boxes.iter().enumerate() {
            if nav.mouse.x >= p.x && nav.mouse.y >= p.y && nav.mouse.x < p.x + s.x && nav.mouse.y < p.y + s.y {
                return (Some(k), moved);
            }
        }
    }
    (nav.confirm.then_some(*sel), moved)
}

/// Fond violet, la diseuse en grand et le titre.
fn backdrop(list: &mut DrawList, ctx: &Ctx, vis: &Vis, title: &str, t: f32) {
    let view = ctx.view;
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(&vis.white, Vec2::ZERO, view, with_alpha(DIM, (t / 0.15).min(1.0)));
    list.rect(&vis.white, Vec2::ZERO, view, [0.25, 0.05, 0.35, 0.25]);
    let main = ctx.atlas.font("main");
    text(list, main, title, Vec2::new(view.x * 0.5, 14.0), crate::hud::title_scale(main, title, view.x, 2.0), PURPLE, Align::Center);
    let f = ctx.atlas.frame(vis.npc_fortune, ctx.real_time as f32);
    list.frame(f, Vec2::new(view.x * 0.5, TELLER_Y), &Params::default().scale(Vec2::splat(3.0)));
    draw_table(list, vis, Vec2::new(view.x * 0.5, TELLER_Y + 3.0), 3.0, PURPLE);
}

/// Centre de la boule de cristal posee sur la table (`draw_table`).
fn table_ball(base: Vec2, s: f32) -> Vec2 {
    (base - Vec2::new(0.0, 7.5 * s)).round()
}

/// Pied de la diseuse dans les menus (sprite x3).
const TELLER_Y: f32 = 104.0;

/// Table drapee et boule de cristal devant la diseuse (elle est assise derriere, comme dans le jeu
/// d'origine) ; `base` : milieu du bas de la nappe.
pub fn draw_table(list: &mut DrawList, vis: &Vis, base: Vec2, s: f32, glow: Color) {
    let w = &vis.white;
    let (tw, th) = (24.0 * s, 6.0 * s);
    let top = (base - Vec2::new(tw * 0.5, th)).round();
    list.rect(w, top, Vec2::new(tw, th), [0.24, 0.1, 0.36, 1.0]);
    list.rect(w, top, Vec2::new(tw, s), rgb(232, 184, 72));
    list.rect(w, top + Vec2::new(0.0, th - s), Vec2::new(tw, s), [0.14, 0.05, 0.22, 1.0]);
    let ball = table_ball(base, s);
    disc(list, vis, ball, 2.5 * s, with_alpha(glow, 0.95));
    disc(list, vis, ball, 1.8 * s, [0.85, 0.8, 1.0, 1.0]);
    list.rect(w, (ball - Vec2::splat(1.0 * s)).round(), Vec2::splat(s), WHITE);
}

/// Les trois coffres (fermes, ou ouverts avec leur contenu) et le resultat.
#[allow(clippy::too_many_arguments)]
pub fn draw_chests(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, stake: u64, opened: Option<([u32; 3], usize)>, sel: usize, t: f32) {
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    backdrop(list, ctx, vis, lang.get("ui.fortune.chests"), t);
    let rt = ctx.real_time as f32;
    text(list, main, &lang.get("ui.fortune.pick").replace("{n}", &stake.to_string()), Vec2::new(view.x * 0.5, TELLER_Y + 14.0), 1.0, WHITE, Align::Center);
    let picked = opened.map(|o| o.1);
    // suspense : 0 -> 1 pendant que le coffre choisi tremble, puis il s'ouvre
    let s = if opened.is_some() { (t / CHEST_SUSPENSE).min(1.0) } else { 0.0 };
    let age = t - CHEST_SUSPENSE;
    for (k, c) in chest_centers(view).into_iter().enumerate() {
        let chosen = picked.map_or(k == sel, |p| p == k);
        // le coffre choisi s'ouvre au bout du suspense, les deux autres ensuite, l'un apres l'autre
        let open_at = if picked == Some(k) { CHEST_SUSPENSE } else { CHEST_SUSPENSE + 0.9 + 0.25 * k as f32 };
        let open = opened.is_some() && t > open_at;
        let mut off = Vec2::ZERO;
        if picked == Some(k) && !open {
            // tremble de plus en plus fort et decolle du sol
            let q = s * s;
            off = Vec2::new((3.0 * q * (rt * 53.0).sin()).round(), (-3.0 * q * (rt * 37.0).cos().abs() - 4.0 * q).round());
        } else if picked == Some(k) && age < 0.25 {
            off = Vec2::new(0.0, (-6.0 * (1.0 - age / 0.25)).round());
        } else if chosen && !open {
            off.y = (2.0 * (rt * 6.0).sin()).round();
        }
        let c = c + off;
        let (bp, bs) = chest_box(c);
        if picked == Some(k) && !open {
            // lumiere qui monte sous le couvercle
            list.set(Space::Ui, Blend::Add);
            let p = 0.5 + 0.5 * (t * (6.0 + 20.0 * s)).sin();
            for (r, a) in [(56.0, 0.16), (40.0, 0.26), (24.0, 0.45)] {
                disc(list, vis, c, r * (0.5 + 0.6 * s) * (0.85 + 0.25 * p), with_alpha(GOLD, a * s));
            }
            rays(list, vis, c, 30.0, 40.0 + 50.0 * s, rt * 2.0, 12, with_alpha(GOLD, 0.8 * s));
            list.set(Space::Ui, Blend::Alpha);
        } else if chosen && opened.is_none() {
            let pulse = 0.6 + 0.4 * (rt * 6.0).sin();
            list.rect(&vis.white, bp, bs, [1.0, 0.85, 0.3, 0.12 * pulse]);
            frame_rect(list, &vis.white, bp, bs, with_alpha(GOLD, pulse));
        }
        // pendant le suspense, les autres coffres s'effacent dans l'ombre
        let dim = if picked.is_some() && picked != Some(k) && !open { 1.0 - 0.55 * s } else { 1.0 };
        let anim = vis.fortune_chest[open as usize];
        list.frame(ctx.atlas.frame(anim, 0.0), c, &Params::default().scale(Vec2::splat(CHEST_SCALE)));
        if dim < 1.0 {
            let (p, sz) = (c - Vec2::splat(8.0 * CHEST_SCALE), Vec2::splat(16.0 * CHEST_SCALE));
            list.rect(&vis.white, p, sz, [0.1, 0.03, 0.15, 1.0 - dim]);
        }
        let label = match opened {
            Some((mults, _)) if open => {
                let m = mults[k];
                let col = match m {
                    0 => rgb(150, 150, 170),
                    1 => WHITE,
                    _ => GOLD,
                };
                let a = t - open_at;
                if picked == Some(k) {
                    burst(list, ctx, vis, c, a, m, k as u32);
                }
                // contenu qui jaillit du coffre (plus haut pour le coffre choisi)
                let rise = (a / 0.35).clamp(0.0, 1.0);
                let h = if picked == Some(k) { 34.0 } else { 10.0 };
                let jump = h * (1.0 - (1.0 - rise) * (1.0 - rise));
                if m > 0 {
                    let r = ctx.atlas.frame(vis.rupees[2], rt);
                    for j in 0..m {
                        let x = (j as f32 - (m - 1) as f32 * 0.5) * 12.0;
                        list.frame(r, c + Vec2::new(x, -24.0 - jump).round(), &Params::default().scale(Vec2::splat(2.0)));
                    }
                }
                (format!("x{m}"), col)
            }
            _ => (format!("{}", k + 1), with_alpha(WHITE, 0.7 * dim)),
        };
        text(list, main, &label.0, c + Vec2::new(0.0, bs.y * 0.5 + 4.0), 2.0, label.1, Align::Center);
    }
    if let Some((mults, pick)) = opened {
        let a = age - 0.3;
        if a > 0.0 {
            let g = chest_gain(stake, mults[pick]);
            let (s, col) = match g {
                g if g > 0 => (lang.get("ui.fortune.won").replace("{n}", &g.to_string()), GOLD),
                0 => (lang.get("ui.fortune.even").to_string(), WHITE),
                g => (lang.get("ui.fortune.lost").replace("{n}", &(-g).to_string()), rgb(255, 120, 120)),
            };
            // le resultat tombe en grand puis se pose
            let size = crate::hud::title_scale(main, &s, view.x, if a < 0.12 { 3.0 } else { 2.0 });
            text(list, main, &s, Vec2::new(view.x * 0.5, view.y - 52.0), size, col, Align::Center);
        }
        continue_hint(list, ctx, lang, age);
    }
}

/// Duree du suspense : le coffre choisi tremble et brille avant de s'ouvrir.
pub const CHEST_SUSPENSE: f32 = 1.5;

/// Pas du roulement de tambour (`cursor`) pendant un suspense de `dur` secondes : de plus en plus
/// rapproches. Vrai quand un coup tombe entre `t - dt` et `t`.
pub fn drumroll(t: f32, dt: f32, dur: f32) -> bool {
    let beat = |x: f32| (9.0 * (x / dur).clamp(0.0, 1.0).powf(1.8)).floor();
    t < dur && beat(t) > beat(t - dt)
}

/// Bruit deterministe dans [0, 1) : les etincelles sont calculees a partir du temps, sans etat.
pub fn h(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65536.0
}

/// Rayons de lumiere pointilles autour de `c`, de `r0` a `r1`.
#[allow(clippy::too_many_arguments)]
pub fn rays(list: &mut DrawList, vis: &Vis, c: Vec2, r0: f32, r1: f32, spin: f32, n: u32, col: Color) {
    for j in 0..n {
        let ang = spin + j as f32 * std::f32::consts::TAU / n as f32;
        let dir = Vec2::new(ang.cos(), ang.sin());
        let mut d = r0;
        while d < r1 {
            list.rect(&vis.white, (c + dir * d).round(), Vec2::splat(2.0), with_alpha(col, col[3] * (1.0 - (d - r0) / (r1 - r0))));
            d += 4.0;
        }
    }
}

/// Anneau de pixels de rayon `r`.
pub fn ring(list: &mut DrawList, vis: &Vis, c: Vec2, r: f32, col: Color) {
    let n = (r * 0.8).max(12.0) as u32;
    for j in 0..n {
        let ang = j as f32 * std::f32::consts::TAU / n as f32;
        list.rect(&vis.white, (c + Vec2::new(ang.cos(), ang.sin()) * r).round(), Vec2::splat(2.0), col);
    }
}

/// Eclat a l'ouverture (`a` : secondes depuis) : onde de choc, rayons et pluie de rubis pour un
/// gain, simple bouffee grise pour un coffre vide. `m` : multiplicateur (0, 1, 3...), `seed` : graine.
fn burst(list: &mut DrawList, ctx: &Ctx, vis: &Vis, c: Vec2, a: f32, m: u32, seed: u32) {
    if a > 2.5 {
        return;
    }
    let col = match m {
        0 => rgb(150, 150, 170),
        1 => WHITE,
        _ => GOLD,
    };
    list.set(Space::Ui, Blend::Add);
    // onde de choc
    let k = (a / 0.5).min(1.0);
    if k < 1.0 {
        ring(list, vis, c, 10.0 + 90.0 * k, with_alpha(col, 0.8 * (1.0 - k)));
        ring(list, vis, c, 6.0 + 55.0 * k, with_alpha(col, 0.5 * (1.0 - k)));
    }
    // colonne de lumiere et rayons pour un gain
    if m > 0 {
        let f = (1.0 - a / 1.4).max(0.0);
        let w = 26.0 * f + 4.0;
        list.rect(&vis.white, Vec2::new((c.x - w * 0.5).round(), 0.0), Vec2::new(w.round(), c.y), with_alpha(col, 0.3 * f));
        rays(list, vis, c, 20.0, 50.0 + 60.0 * k, a * 1.2, if m > 1 { 16 } else { 8 }, with_alpha(col, 0.6 * f));
    }
    // etincelles projetees qui retombent
    let n = if m > 1 { 40 } else { 18 };
    for i in 0..n {
        let s = seed * 997 + i;
        let ang = -std::f32::consts::PI * (0.1 + 0.8 * h(s));
        let v = 80.0 + 140.0 * h(s + 7);
        let life = 0.8 + 0.8 * h(s + 13);
        if a > life {
            continue;
        }
        let p = c + Vec2::new(ang.cos() * v * a, ang.sin() * v * a + 160.0 * a * a);
        list.rect(&vis.white, p.round(), Vec2::splat(2.0), with_alpha(col, 1.0 - a / life));
    }
    list.set(Space::Ui, Blend::Alpha);
    // pluie de rubis pour un gros gain
    if m > 1 {
        let r = ctx.atlas.frame(vis.rupees[2], ctx.real_time as f32);
        for i in 0..14u32 {
            let s = seed * 131 + i * 3;
            let ang = -std::f32::consts::PI * (0.2 + 0.6 * h(s));
            let v = 110.0 + 90.0 * h(s + 1);
            let p = c + Vec2::new(ang.cos() * v * a, ang.sin() * v * a + 220.0 * a * a);
            if p.y < ctx.view.y + 16.0 {
                list.frame(r, p.round(), &Params::default().scale(Vec2::splat(2.0)));
            }
        }
    }
}

/// Quitte ou double : la boule de cristal brille de plus en plus fort, puis le resultat.
pub fn draw_double(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, win: bool, t: f32) {
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    backdrop(list, ctx, vis, lang.get("ui.fortune.double"), t);
    // boule de la table (voir `draw_table`, echelle 3)
    let c = table_ball(Vec2::new(view.x * 0.5, TELLER_Y + 3.0), 3.0);
    let reveal = t > DOUBLE_GAZE;
    let col = if !reveal { PURPLE } else if win { GOLD } else { rgb(255, 110, 110) };
    // boule : disque de pixels, eclat additif qui pulse de plus en plus vite, comme un coeur qui s'emballe
    let k = (t / DOUBLE_GAZE).min(1.0);
    let pulse = 0.5 + 0.5 * (t * (4.0 + 22.0 * k * k)).sin();
    let a = t - DOUBLE_GAZE;
    let glow = if reveal { (1.0 - a / 1.5).max(0.35) } else { 0.5 + 0.5 * k };
    list.set(Space::Ui, Blend::Add);
    for (r, al) in [(46.0, 0.08), (34.0, 0.12), (22.0, 0.2), (13.0, 0.4)] {
        disc(list, vis, c, r * (0.8 + 0.3 * pulse * k), with_alpha(col, al * glow));
    }
    if !reveal {
        // l'energie converge vers la boule, de plus en plus vite
        for i in 0..24u32 {
            let ph = (t * (0.8 + 1.6 * k) + h(i)).fract();
            let ang = h(i + 50) * std::f32::consts::TAU + t * 0.7;
            let d = 90.0 * (1.0 - ph) + 10.0;
            list.rect(&vis.white, (c + Vec2::new(ang.cos(), ang.sin()) * d).round(), Vec2::splat(3.0), with_alpha(col, k * ph));
        }
        rays(list, vis, c, 18.0, 40.0 + 30.0 * k, t * (1.5 + 3.0 * k), 8, with_alpha(col, 0.35 * k));
    } else if a < 2.5 {
        // revelation : onde de choc et rayons, dore pour un gain ; eclats rouges pour une perte
        let q = (a / 0.6).min(1.0);
        if q < 1.0 {
            ring(list, vis, c, 12.0 + 140.0 * q, with_alpha(col, 0.9 * (1.0 - q)));
            ring(list, vis, c, 8.0 + 80.0 * q, with_alpha(col, 0.5 * (1.0 - q)));
        }
        if win {
            rays(list, vis, c, 20.0, 60.0 + 80.0 * q, a * 0.8, 16, with_alpha(col, 0.6 * (1.0 - a / 2.5)));
        }
        for i in 0..36u32 {
            let ang = h(i * 3 + 1) * std::f32::consts::TAU;
            let v = 60.0 + 150.0 * h(i * 3 + 2);
            let life = 0.7 + 0.9 * h(i * 3 + 3);
            if a < life {
                let p = c + Vec2::new(ang.cos() * v * a, ang.sin() * v * a + 120.0 * a * a);
                list.rect(&vis.white, p.round(), Vec2::splat(if win { 2.0 } else { 3.0 }), with_alpha(col, 1.0 - a / life));
            }
        }
    }
    list.set(Space::Ui, Blend::Alpha);
    if reveal && !win {
        // la boule se ternit
        disc(list, vis, c, 2.6 * 3.0, [0.18, 0.12, 0.22, (a / 0.2).min(1.0) * 0.85]);
    }
    if reveal && win && a < 2.5 {
        // pluie de rubis
        let r = ctx.atlas.frame(vis.rupees[2], ctx.real_time as f32);
        for i in 0..20u32 {
            let ang = -std::f32::consts::PI * (0.15 + 0.7 * h(i + 300));
            let v = 120.0 + 110.0 * h(i + 400);
            let p = c + Vec2::new(ang.cos() * v * a, ang.sin() * v * a + 230.0 * a * a);
            if p.y < view.y + 16.0 {
                list.frame(r, p.round(), &Params::default().scale(Vec2::splat(2.0)));
            }
        }
    }
    let msg = if reveal {
        lang.get(if win { "ui.fortune.double_win" } else { "ui.fortune.double_lose" }).to_string()
    } else {
        lang.get("ui.fortune.gazing").to_string()
    };
    let size = if !reveal { 1.0 } else { crate::hud::title_scale(main, &msg, view.x, if a < 0.12 { 3.0 } else { 2.0 }) };
    text(list, main, &msg, Vec2::new(view.x * 0.5, view.y - 52.0), size, if reveal { col } else { WHITE }, Align::Center);
    if reveal {
        continue_hint(list, ctx, lang, t - DOUBLE_GAZE);
    }
}

/// Duree du suspense avant le resultat du quitte ou double.
pub const DOUBLE_GAZE: f32 = 2.6;

fn continue_hint(list: &mut DrawList, ctx: &Ctx, lang: &Lang, t: f32) {
    if t < RESULT_LOCK {
        return;
    }
    let a = 0.6 + 0.4 * (ctx.real_time as f32 * 4.0).sin();
    text(list, ctx.atlas.font("main"), lang.get("ui.continue"), Vec2::new(ctx.view.x * 0.5, ctx.view.y - 22.0), 1.0, with_alpha(WHITE, a), Align::Center);
}

pub fn disc(list: &mut DrawList, vis: &Vis, c: Vec2, r: f32, col: Color) {
    let n = r.ceil() as i32;
    for dy in -n..=n {
        let w = (r * r - (dy as f32) * (dy as f32)).max(0.0).sqrt().round();
        if w > 0.0 {
            list.rect(&vis.white, Vec2::new((c.x - w).round(), c.y.round() + dy as f32), Vec2::new(w * 2.0, 1.0), col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_favour_the_player_a_little() {
        let chests: f32 = CHEST_MULTS.iter().map(|&m| m as f32).sum::<f32>() / 3.0;
        assert!(chests > 1.0 && chests < 1.5);
        let double = DOUBLE_WIN * 2.0 + (1.0 - DOUBLE_WIN) * 0.5;
        assert!(double > 1.0 && double < 1.5);
        assert_eq!(chest_gain(500, 3), 1000);
        assert_eq!(chest_gain(500, 1), 0);
        assert_eq!(chest_gain(500, 0), -500);
    }
}
