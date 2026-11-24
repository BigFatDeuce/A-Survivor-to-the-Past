//! Ecran titre (decor du lac, logo a droite, menu a gauche) et ecran « A propos ».
//! Le decor est un pixel art 256x144 agrandi pour couvrir toute la fenetre, quel que soit son format.
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use crate::lang::Lang;
use crate::menus::{input_rects, NavIn};
use engine::app::Ctx;
use engine::assets::AnimId;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use serde::Deserialize;
use std::path::Path;

const SOFT: Color = rgb(228, 228, 244);
const BG_W: f32 = 256.0;
const BG_H: f32 = 144.0;
/// Position voulue du centre du logo dans le decor (pixels du decor) : a droite, avant le chateau.
const LOGO_AT: Vec2 = Vec2::new(166.0, 70.0);
const LOGO_HALF_W: f32 = 41.0;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Entrees du menu titre ; `Update` n'apparait que lorsqu'une mise a jour est en cours ou possible.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TitleItem {
    Play,
    Shop,
    Profile,
    Options,
    About,
    Update,
    Quit,
}

/// Liens de `data/links.ron`.
#[derive(Deserialize, Default, Clone)]
pub struct Links {
    pub discord: String,
    pub github: String,
    pub website: String,
    #[allow(dead_code)]
    pub releases: String,
}

impl Links {
    pub fn load(root: &Path) -> Links {
        let p = root.join("data/links.ron");
        engine::vfs::read_string(&p)
            .and_then(|t| ron::from_str(&t).map_err(|e| log::warn!("{} : {e}", p.display())).ok())
            .unwrap_or_default()
    }

    /// Ouvre un lien dans le navigateur ; renvoie faux en cas d'echec.
    pub fn open(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window().and_then(|w| w.open_with_url_and_target(url, "_blank").ok().flatten()).is_some()
        }
        #[cfg(not(target_arch = "wasm32"))]
        match open::that_detached(url) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("ouverture de {url} impossible : {e}");
                false
            }
        }
    }
}

pub struct TitleVis {
    bg: AnimId,
    logo: AnimId,
    spark: AnimId,
    drop: AnimId,
    splash: AnimId,
}

impl TitleVis {
    pub fn new(ctx: &Ctx) -> TitleVis {
        let a = &ctx.atlas;
        TitleVis {
            bg: a.get("title_bg", "0"),
            logo: a.get("title_logo", "0"),
            spark: a.get("fx_spark", "0"),
            drop: a.get("fx_rain_drop", "0"),
            splash: a.get("fx_rain_splash", "0"),
        }
    }
}

/// Premier eclair (s) puis intervalle entre deux eclairs (s, minimum + variation).
const STRIKE_FIRST: f32 = 4.5;
const STRIKE_GAP: (f32, u32) = (9.0, 9);
/// Le tonnerre suit l'eclair de peu (orage proche).
pub const THUNDER_DELAY: f32 = 0.3;

/// Dernier eclair avant `t` : (numero, instant). Sans etat, tout se deduit du temps.
pub fn strike_before(t: f32) -> Option<(u32, f32)> {
    let mut at = STRIKE_FIRST;
    if t < at {
        return None;
    }
    let mut k = 0;
    loop {
        let next = at + STRIKE_GAP.0 + (hash(k, 77) % STRIKE_GAP.1) as f32;
        if next > t {
            return Some((k, at));
        }
        at = next;
        k += 1;
    }
}

/// Intensite de l'eclair (0..1) : double eclat bref puis decroissance.
fn flash(t: f32) -> f32 {
    let Some((_, at)) = strike_before(t) else { return 0.0 };
    match t - at {
        d if d < 0.06 => 1.0,
        d if d < 0.13 => 0.15,
        d if d < 0.22 => 0.75,
        d if d < 0.8 => 0.75 * (1.0 - (d - 0.22) / 0.58),
        _ => 0.0,
    }
}

/// Placement de l'ecran titre pour une taille de vue.
pub struct Layout {
    /// Agrandissement du decor (pixels logiques par pixel du decor).
    pub k: f32,
    pub bg_pos: Vec2,
    pub logo: Vec2,
    /// Grands caracteres (vue assez large).
    pub big: bool,
    pub left: f32,
    pub title_y: f32,
    pub items: Vec<(Vec2, Vec2)>,
}

pub fn layout(view: Vec2, n: usize) -> Layout {
    let k = (view.x / BG_W).max(view.y / BG_H);
    let bg_pos = ((view - Vec2::new(BG_W, BG_H) * k) * 0.5).round();
    // le logo reste visible quand le decor est rogne (4:3) et ne touche pas le menu
    let right_edge = (view.x - bg_pos.x) / k;
    let lx = LOGO_AT.x.min(right_edge - LOGO_HALF_W - 4.0);
    let logo = (bg_pos + Vec2::new(lx, LOGO_AT.y) * k).round();
    let big = view.x >= 560.0;
    let left = (view.x * 0.07).round().max(14.0);
    let title_y = (view.y * 0.09).round();
    let mut item_h: f32 = if big { 30.0 } else { 22.0 };
    let item_w = (logo.x - LOGO_HALF_W * k - left - 4.0).clamp(140.0, 260.0);
    // menu long (boutique, profil, mise a jour) : on remonte puis on resserre au-dessus du pied de page
    let foot = view.y - 30.0;
    let mut y0 = (view.y * 0.44).round();
    let need = n as f32 * item_h;
    if y0 + need > foot {
        y0 = (foot - need).max((view.y * 0.3).round());
        item_h = item_h.min(((foot - y0) / n.max(1) as f32).floor()).max(if big { 24.0 } else { 18.0 });
    }
    let items = (0..n)
        .map(|i| (Vec2::new(left - 8.0, y0 + i as f32 * item_h - 4.0), Vec2::new(item_w, item_h - 4.0)))
        .collect();
    Layout { k, bg_pos, logo, big, left, title_y, items }
}

fn hash(a: u32, b: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9e37_79b1) ^ b.wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xc2b2_ae3d);
    h ^ (h >> 13)
}

/// Decor + logo + scintillements (commun au titre et a « A propos »).
fn draw_scene(list: &mut DrawList, ctx: &Ctx, v: &TitleVis, l: &Layout, t: f32) {
    let atlas = &ctx.atlas;
    list.set(Space::Ui, Blend::Alpha);
    list.frame(atlas.frame(v.bg, 0.0), l.bg_pos, &Params::default().scale(Vec2::splat(l.k)));
    // ciel d'orage : decor assombri et bleui, eclaire par les eclairs
    let white = &atlas.white;
    let f = flash(t);
    list.rect(white, Vec2::ZERO, ctx.view, [0.03, 0.05, 0.14, 0.42 * (1.0 - f * 0.8)]);
    if f > 0.0 {
        // l'eclair illumine le decor (le logo reste net)
        list.set(Space::Ui, Blend::Add);
        list.rect(white, Vec2::ZERO, ctx.view, [0.7, 0.75, 1.0, 0.22 * f]);
        list.set(Space::Ui, Blend::Alpha);
    }
    let bob = ((t * 1.4).sin() * 1.5).round();
    let logo_pos = l.logo + Vec2::new(0.0, bob);
    list.frame(atlas.frame(v.logo, 0.0), logo_pos, &Params::default().scale(Vec2::splat(l.k)));
    // eclat sur le pommeau de l'epee
    let st = t % 4.5;
    if st < 0.45 {
        let p = logo_pos + Vec2::new(0.0, -56.0 * l.k);
        list.set(Space::Ui, Blend::Add);
        list.frame(atlas.frame(v.spark, st), p.round(), &Params::default().scale(Vec2::splat(2.0)));
        list.set(Space::Ui, Blend::Alpha);
    }
    draw_rain(list, ctx, v, l, t);
}

/// Bande du lac dans le decor (pixels du decor) : seules les gouttes qui y tombent eclaboussent.
const LAKE: (f32, f32) = (104.0, 144.0);

/// Pluie de l'introduction d'ALttP : gouttes en biais, eclaboussures sur le lac.
/// Pixels a l'echelle du decor ; quantite proportionnelle a la surface de la vue.
fn draw_rain(list: &mut DrawList, ctx: &Ctx, v: &TitleVis, l: &Layout, t: f32) {
    const SPEED: f32 = 290.0; // pixels du decor par seconde (vertical)
    const SLOPE: f32 = 0.5; // 8 pixels vers la gauche pour 16 vers le bas
    const SPLASH: f32 = 0.25;
    let k = l.k;
    let atlas = &ctx.atlas;
    let vw = ctx.view / k;
    let lake = (l.bg_pos.y / k + LAKE.0, (l.bg_pos.y / k + LAKE.1).min(vw.y));
    let n = (vw.x * vw.y / 210.0) as u32;
    let drop = atlas.frame(v.drop, 0.0);
    let ps = Params::default().scale(Vec2::splat(k));
    list.set(Space::Ui, Blend::Add);
    for i in 0..n {
        let cycle = 0.7 + (hash(i, 7) % 500) as f32 / 1000.0;
        let tt = t + (hash(i, 3) % 1000) as f32 / 1000.0 * cycle;
        let c = (tt / cycle).floor() as u32;
        let local = tt - c as f32 * cycle;
        let h = hash(c, i.wrapping_mul(31).wrapping_add(5));
        let fall = 0.22 + (h % 100) as f32 / 100.0 * 0.3;
        // point d'impact (les gouttes viennent du haut a droite)
        let reach = fall * SPEED;
        let gx = ((h >> 7) % 10_000) as f32 / 10_000.0 * (vw.x + reach * SLOPE) - 4.0;
        let gy = ((h >> 14) % 10_000) as f32 / 10_000.0 * (vw.y + 8.0);
        if local < fall {
            let d = (fall - local) * SPEED;
            let p = Vec2::new(gx + d * SLOPE, gy - d) * k;
            list.frame(drop, p.round(), &ps.color([0.9, 0.97, 1.0, 0.8]));
        } else if local < fall + SPLASH && gy >= lake.0 && gy <= lake.1 {
            let f = atlas.frame(v.splash, local - fall);
            list.frame(f, (Vec2::new(gx, gy) * k).round(), &ps.color([0.9, 0.97, 1.0, 0.7]));
        }
    }
    list.set(Space::Ui, Blend::Alpha);
}

/// Decor seul (sous les options ouvertes depuis le titre).
pub fn draw_backdrop(list: &mut DrawList, ctx: &Ctx, v: &TitleVis, t: f32) {
    draw_scene(list, ctx, v, &layout(ctx.view, 4), t);
}

/// `items` : entrees et libelles deja traduits.
pub fn draw_title(
    list: &mut DrawList,
    ctx: &Ctx,
    v: &TitleVis,
    lang: &Lang,
    white: &assetkit::meta::Frame,
    items: &[(TitleItem, String)],
    sel: usize,
    t: f32,
) {
    let view = ctx.view;
    let l = layout(view, items.len());
    draw_scene(list, ctx, v, &l, t);
    let main = ctx.atlas.font("main");
    // nom du jeu (nom propre : identique dans toutes les langues)
    let ts = if l.big { 3.0 } else { 2.0 };
    let lh = main.line_height as f32 * ts;
    text(list, main, "A Survivor", Vec2::new(l.left, l.title_y), ts, GOLD, Align::Left);
    text(list, main, "to the Past", Vec2::new(l.left + 10.0 * ts, l.title_y + lh), ts, GOLD, Align::Left);
    // cadre du menu facon ALttP
    let (p0, _) = l.items[0];
    let (pn, sn) = *l.items.last().unwrap();
    let box_pos = p0 - Vec2::new(10.0, 8.0);
    let box_size = Vec2::new(sn.x + 20.0, pn.y + sn.y - p0.y + 16.0);
    list.rect(white, box_pos, box_size, with_alpha(BACK, 0.82));
    frame_rect(list, white, box_pos + Vec2::splat(2.0), box_size - Vec2::splat(4.0), with_alpha(GOLD, 0.8));
    let is = if l.big { 2.0 } else { 1.0 };
    for (k, (item, label)) in items.iter().enumerate() {
        let (p, s) = l.items[k];
        let c = if k == sel {
            GOLD
        } else if *item == TitleItem::Update {
            rgb(150, 230, 150)
        } else {
            SOFT
        };
        // libelle trop long pour le cadre : plus petit
        let room = s.x - 12.0 - 10.0 * is;
        let is = if main.measure(label) as f32 * is > room { 1.0 } else { is };
        let ty = (p.y + (s.y - main.line_height as f32 * is) * 0.5 + is).round();
        if k == sel {
            list.rect(white, p, s, [1.0, 1.0, 1.0, 0.07]);
            let blink = (t * 3.0).fract() < 0.75;
            text(list, main, "▶", Vec2::new(p.x + 4.0, ty), is, with_alpha(GOLD, if blink { 1.0 } else { 0.6 }), Align::Left);
        }
        let lx = if l.big { p.x + 28.0 } else { p.x + 18.0 };
        text(list, main, label, Vec2::new(lx, ty), is, c, Align::Left);
    }
    // pied de page
    let fy = view.y - 16.0;
    text(list, main, lang.get("ui.fangame"), Vec2::new(l.left - 6.0, fy), 1.0, [1.0, 1.0, 1.0, 0.85], Align::Left);
    text(list, main, &format!("v{VERSION}"), Vec2::new(view.x - 10.0, fy), 1.0, [1.0, 1.0, 1.0, 0.85], Align::Right);
    // ouverture en fondu
    if t < 0.6 {
        list.rect(white, Vec2::ZERO, view, [0.0, 0.0, 0.0, 1.0 - t / 0.6]);
    }
}

pub fn input_title(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize) -> (Option<usize>, bool) {
    *sel = (*sel).min(n.saturating_sub(1));
    input_rects(nav, &layout(view, n).items, sel)
}

// ---------------------------------------------------------------------------------------------
// A propos

pub const ABOUT_ITEMS: [&str; 4] = ["ui.link.discord", "ui.link.github", "ui.link.website", "ui.back"];

pub struct AboutLayout {
    pub panel: (Vec2, Vec2),
    pub text_x: f32,
    pub text_w: f32,
    pub title_y: f32,
    pub para_y: f32,
    pub items: Vec<(Vec2, Vec2)>,
}

pub fn about_layout(view: Vec2, lang: &Lang, font: &assetkit::meta::Font) -> AboutLayout {
    let w = (view.x - 32.0).min(500.0);
    let text_w = w - 32.0;
    let lines: usize = ["ui.about.p1", "ui.about.p2", "ui.about.p3"]
        .iter()
        .map(|k| wrap(font, lang.get(k), text_w, 1.0).len())
        .sum();
    let lh = 13.0;
    let para_h = lines as f32 * lh + 2.0 * 8.0;
    let items_h = ABOUT_ITEMS.len() as f32 * 20.0;
    let h = (28.0 + 14.0 + para_h + 10.0 + items_h + 30.0).min(view.y - 16.0);
    let pos = ((view - Vec2::new(w, h)) * 0.5).round();
    let text_x = pos.x + 16.0;
    let title_y = pos.y + 12.0;
    let para_y = title_y + 40.0;
    let items_y = para_y + para_h + 10.0;
    let items = (0..ABOUT_ITEMS.len())
        .map(|i| (Vec2::new(text_x - 4.0, items_y + i as f32 * 20.0 - 3.0), Vec2::new(text_w + 8.0, 18.0)))
        .collect();
    AboutLayout { panel: (pos, Vec2::new(w, h)), text_x, text_w, title_y, para_y, items }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_about(
    list: &mut DrawList,
    ctx: &Ctx,
    v: &TitleVis,
    lang: &Lang,
    white: &assetkit::meta::Frame,
    links: &Links,
    sel: usize,
    t: f32,
    toast: f32,
) {
    let view = ctx.view;
    draw_scene(list, ctx, v, &layout(view, 4), t);
    let main = ctx.atlas.font("main");
    list.rect(white, Vec2::ZERO, view, [0.0, 0.0, 0.05, 0.45]);
    let l = about_layout(view, lang, main);
    let (pp, ps) = l.panel;
    list.rect(white, pp, ps, with_alpha(BACK, 0.94));
    frame_rect(list, white, pp + Vec2::splat(2.0), ps - Vec2::splat(4.0), with_alpha(GOLD, 0.8));
    text(list, main, lang.get("ui.about"), Vec2::new(pp.x + ps.x * 0.5, l.title_y), 2.0, WHITE, Align::Center);
    let mut y = l.para_y;
    for (i, key) in ["ui.about.p1", "ui.about.p2", "ui.about.p3"].iter().enumerate() {
        let col = if i == 1 { rgb(255, 214, 160) } else { SOFT };
        for line in wrap(main, lang.get(key), l.text_w, 1.0) {
            text(list, main, &line, Vec2::new(l.text_x, y), 1.0, col, Align::Left);
            y += 13.0;
        }
        y += 8.0;
    }
    let urls = [links.discord.as_str(), links.github.as_str(), links.website.as_str(), ""];
    for (k, key) in ABOUT_ITEMS.iter().enumerate() {
        let (p, s) = l.items[k];
        let c = if k == sel { GOLD } else { SOFT };
        if k == sel {
            list.rect(white, p, s, [1.0, 1.0, 1.0, 0.07]);
            text(list, main, "▶", Vec2::new(p.x - 10.0, p.y + 3.0), 1.0, GOLD, Align::Left);
        }
        text(list, main, lang.get(key), Vec2::new(p.x + 4.0, p.y + 3.0), 1.0, c, Align::Left);
    }
    // adresse du lien choisi (ou confirmation d'ouverture)
    let foot = Vec2::new(pp.x + ps.x * 0.5, pp.y + ps.y - 17.0);
    if toast > 0.0 {
        let a = (toast / 0.3).min(1.0);
        text(list, main, lang.get("ui.link.opened"), foot, 1.0, with_alpha(rgb(150, 240, 150), a), Align::Center);
    } else {
        let url = urls.get(sel).copied().unwrap_or("").trim_start_matches("https://").trim_end_matches('/');
        text(list, main, url, foot, 1.0, [0.7, 0.75, 0.9, 0.9], Align::Center);
    }
}

pub fn input_about(nav: &NavIn, view: Vec2, lang: &Lang, font: &assetkit::meta::Font, sel: &mut usize) -> (Option<usize>, bool) {
    input_rects(nav, &about_layout(view, lang, font).items, sel)
}

pub fn about_url(links: &Links, k: usize) -> Option<&str> {
    match k {
        0 => Some(&links.discord),
        1 => Some(&links.github),
        2 => Some(&links.website),
        _ => None,
    }
}
