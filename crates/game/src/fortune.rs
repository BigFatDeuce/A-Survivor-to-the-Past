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
    OfferInfo { icon, name, tag, is_new: ok, desc, level: 0, max: 0, evo: Vec::new(), evo_label: String::new(), keep: false }
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
    text(list, main, title, Vec2::new(view.x * 0.5, 14.0), 2.0, PURPLE, Align::Center);
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
    for (k, c) in chest_centers(view).into_iter().enumerate() {
        let picked = opened.map(|o| o.1);
        let chosen = picked.map_or(k == sel, |p| p == k);
        // le coffre choisi s'ouvre d'abord, les deux autres juste apres
        let open = opened.is_some() && t > if picked == Some(k) { 0.15 } else { 0.7 };
        let bob = if chosen && !open { (2.0 * (rt * 6.0).sin()).round() } else { 0.0 };
        let (bp, bs) = chest_box(c);
        if chosen {
            let pulse = 0.6 + 0.4 * (rt * 6.0).sin();
            list.rect(&vis.white, bp, bs, [1.0, 0.85, 0.3, 0.12 * pulse]);
            frame_rect(list, &vis.white, bp, bs, with_alpha(GOLD, pulse));
        }
        let anim = vis.fortune_chest[open as usize];
        list.frame(ctx.atlas.frame(anim, 0.0), c + Vec2::new(0.0, bob), &Params::default().scale(Vec2::splat(CHEST_SCALE)));
        let label = match opened {
            Some((mults, _)) if open => {
                let m = mults[k];
                let col = match m {
                    0 => rgb(150, 150, 170),
                    1 => WHITE,
                    _ => GOLD,
                };
                // contenu qui monte du coffre
                let rise = ((t - if picked == Some(k) { 0.15 } else { 0.7 }) / 0.3).clamp(0.0, 1.0);
                if m > 0 {
                    let r = ctx.atlas.frame(vis.rupees[2], rt);
                    for j in 0..m {
                        let x = (j as f32 - (m - 1) as f32 * 0.5) * 12.0;
                        list.frame(r, c + Vec2::new(x, -24.0 - 10.0 * rise), &Params::default().scale(Vec2::splat(2.0)));
                    }
                }
                (format!("x{m}"), col)
            }
            _ => (format!("{}", k + 1), with_alpha(WHITE, 0.7)),
        };
        text(list, main, &label.0, c + Vec2::new(0.0, bs.y * 0.5 + 4.0), 2.0, label.1, Align::Center);
    }
    if let Some((mults, pick)) = opened {
        if t > 0.9 {
            let g = chest_gain(stake, mults[pick]);
            let (s, col) = match g {
                g if g > 0 => (lang.get("ui.fortune.won").replace("{n}", &g.to_string()), GOLD),
                0 => (lang.get("ui.fortune.even").to_string(), WHITE),
                g => (lang.get("ui.fortune.lost").replace("{n}", &(-g).to_string()), rgb(255, 120, 120)),
            };
            text(list, main, &s, Vec2::new(view.x * 0.5, view.y - 52.0), 2.0, col, Align::Center);
        }
        continue_hint(list, ctx, lang, t);
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
    // boule : disque de pixels, eclat additif qui pulse de plus en plus vite
    let k = (t / DOUBLE_GAZE).min(1.0);
    let pulse = 0.5 + 0.5 * (t * (4.0 + 14.0 * k)).sin();
    list.set(Space::Ui, Blend::Add);
    for (r, a) in [(34.0, 0.12), (22.0, 0.2), (13.0, 0.4)] {
        disc(list, vis, c, r * (0.9 + 0.2 * pulse), with_alpha(col, a * (0.5 + 0.5 * k)));
    }
    // rayons qui tournent autour de la boule
    for j in 0..8 {
        let ang = t * 1.5 + j as f32 * std::f32::consts::TAU / 8.0;
        let len = 40.0 + 20.0 * k;
        for d in (18..len as i32).step_by(4) {
            let q = c + Vec2::new(ang.cos(), ang.sin()) * d as f32;
            list.rect(&vis.white, q.round(), Vec2::splat(2.0), with_alpha(col, 0.35 * k * (1.0 - d as f32 / len)));
        }
    }
    list.set(Space::Ui, Blend::Alpha);
    let msg = if reveal {
        lang.get(if win { "ui.fortune.double_win" } else { "ui.fortune.double_lose" }).to_string()
    } else {
        lang.get("ui.fortune.gazing").to_string()
    };
    text(list, main, &msg, Vec2::new(view.x * 0.5, view.y - 52.0), if reveal { 2.0 } else { 1.0 }, if reveal { col } else { WHITE }, Align::Center);
    if reveal {
        continue_hint(list, ctx, lang, t - DOUBLE_GAZE);
    }
}

/// Duree du suspense avant le resultat du quitte ou double.
pub const DOUBLE_GAZE: f32 = 1.6;

fn continue_hint(list: &mut DrawList, ctx: &Ctx, lang: &Lang, t: f32) {
    if t < RESULT_LOCK {
        return;
    }
    let a = 0.6 + 0.4 * (ctx.real_time as f32 * 4.0).sin();
    text(list, ctx.atlas.font("main"), lang.get("ui.continue"), Vec2::new(ctx.view.x * 0.5, ctx.view.y - 22.0), 1.0, with_alpha(WHITE, a), Align::Center);
}

fn disc(list: &mut DrawList, vis: &Vis, c: Vec2, r: f32, col: Color) {
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
