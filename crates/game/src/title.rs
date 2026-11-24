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

pub const TITLE_ITEMS: [&str; 4] = ["ui.play", "ui.options", "ui.about", "ui.quit"];
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|t| ron::from_str(&t).map_err(|e| log::warn!("{} : {e}", p.display())).ok())
            .unwrap_or_default()
    }

    /// Ouvre un lien dans le navigateur ; renvoie faux en cas d'echec.
    pub fn open(url: &str) -> bool {
        if url.is_empty() {
            return false;
        }
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
}

impl TitleVis {
    pub fn new(ctx: &Ctx) -> TitleVis {
        let a = &ctx.atlas;
        TitleVis { bg: a.get("title_bg", "0"), logo: a.get("title_logo", "0"), spark: a.get("fx_spark", "0") }
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

pub fn layout(view: Vec2) -> Layout {
    let k = (view.x / BG_W).max(view.y / BG_H);
    let bg_pos = ((view - Vec2::new(BG_W, BG_H) * k) * 0.5).round();
    // le logo reste visible quand le decor est rogne (4:3) et ne touche pas le menu
    let right_edge = (view.x - bg_pos.x) / k;
    let lx = LOGO_AT.x.min(right_edge - LOGO_HALF_W - 4.0);
    let logo = (bg_pos + Vec2::new(lx, LOGO_AT.y) * k).round();
    let big = view.x >= 560.0;
    let left = (view.x * 0.07).round().max(14.0);
    let title_y = (view.y * 0.09).round();
    let item_h = if big { 30.0 } else { 22.0 };
    let item_w = (logo.x - LOGO_HALF_W * k - left - 4.0).clamp(140.0, 260.0);
    let y0 = (view.y * 0.44).round();
    let items = (0..TITLE_ITEMS.len())
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
    // reflets sur le lac
    list.set(Space::Ui, Blend::Add);
    for i in 0..4u32 {
        let cycle = 2.9 + i as f32 * 0.37;
        let tt = t + i as f32 * 1.13;
        let n = (tt / cycle).floor() as u32;
        let local = tt - n as f32 * cycle;
        if local < 0.5 {
            let h = hash(n, i);
            let bx = (h % 200) as f32 + 4.0;
            let by = 104.0 + ((h >> 8) % 36) as f32;
            let p = l.bg_pos + Vec2::new(bx, by) * l.k;
            let f = atlas.frame(v.spark, local);
            list.frame(f, p.round(), &Params::default().color([0.8, 0.9, 1.0, 0.8]));
        }
    }
    list.set(Space::Ui, Blend::Alpha);
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
}

/// Decor seul (sous les options ouvertes depuis le titre).
pub fn draw_backdrop(list: &mut DrawList, ctx: &Ctx, v: &TitleVis, t: f32) {
    draw_scene(list, ctx, v, &layout(ctx.view), t);
}

pub fn draw_title(list: &mut DrawList, ctx: &Ctx, v: &TitleVis, lang: &Lang, white: &assetkit::meta::Frame, sel: usize, t: f32) {
    let view = ctx.view;
    let l = layout(view);
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
    for (k, key) in TITLE_ITEMS.iter().enumerate() {
        let (p, s) = l.items[k];
        let c = if k == sel { GOLD } else { SOFT };
        let ty = (p.y + (s.y - main.line_height as f32 * is) * 0.5 + is).round();
        if k == sel {
            list.rect(white, p, s, [1.0, 1.0, 1.0, 0.07]);
            let blink = (t * 3.0).fract() < 0.75;
            text(list, main, "▶", Vec2::new(p.x + 4.0, ty), is, with_alpha(GOLD, if blink { 1.0 } else { 0.6 }), Align::Left);
        }
        text(list, main, lang.get(key), Vec2::new(p.x + 8.0 + 10.0 * is, ty), is, c, Align::Left);
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

pub fn input_title(nav: &NavIn, view: Vec2, sel: &mut usize) -> (Option<usize>, bool) {
    input_rects(nav, &layout(view).items, sel)
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
    draw_scene(list, ctx, v, &layout(view), t);
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
