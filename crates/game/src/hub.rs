//! Ecran a onglets commun a la boutique (Kakariko, Grande Fee, reliques, defis) et au profil
//! (bestiaire, armurerie, statistiques, sauvegarde) : une liste qui defile, un panneau de detail
//! et un bouton Retour. Clavier, manette et souris.
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use crate::menus::NavIn;
use engine::app::Ctx;
use engine::assets::AnimId;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;

/// Fond des lignes (plus couvrant que celui des menus en partie).
const PANEL: Color = [0.05, 0.035, 0.09, 0.97];
pub const SOFT: Color = rgb(210, 210, 236);
pub const DIMMED: Color = rgb(120, 116, 140);
pub const GREEN: Color = rgb(150, 240, 150);
pub const RED: Color = rgb(255, 120, 110);

#[derive(Clone, Default)]
pub struct Row {
    pub icon: Option<AnimId>,
    /// Icone assombrie (pas encore obtenue).
    pub icon_dim: bool,
    pub label: String,
    pub value: String,
    pub value_color: Option<Color>,
    /// Ligne grisee (indisponible).
    pub dim: bool,
    /// Ligne cochee (equipee, active).
    pub mark: bool,
}

pub struct Layout {
    pub title_y: f32,
    pub tabs: Vec<(Vec2, Vec2)>,
    /// Lignes visibles : (indice, position, taille).
    pub rows: Vec<(usize, Vec2, Vec2)>,
    pub back: (Vec2, Vec2),
    pub detail: (Vec2, Vec2),
}

const ROW_H: f32 = 22.0;

pub fn layout(view: Vec2, ntabs: usize, nrows: usize, sel: usize) -> Layout {
    let w = (view.x - 32.0).min(560.0);
    let x0 = ((view.x - w) * 0.5).round();
    let title_y = 10.0;
    let tab_y = 40.0;
    let tw = (w / ntabs.max(1) as f32).floor();
    let tabs = (0..ntabs).map(|k| (Vec2::new(x0 + k as f32 * tw, tab_y), Vec2::new(tw - 4.0, 20.0))).collect();
    let back_h = 20.0;
    let detail_h = 46.0;
    let list_top = tab_y + 28.0;
    let list_bottom = view.y - back_h - detail_h - 18.0;
    let fit = (((list_bottom - list_top) / ROW_H).floor() as usize).max(3);
    let first = if nrows <= fit { 0 } else { sel.min(nrows - 1).saturating_sub(fit / 2).min(nrows - fit) };
    let rows = (first..nrows.min(first + fit))
        .enumerate()
        .map(|(j, k)| (k, Vec2::new(x0, list_top + j as f32 * ROW_H), Vec2::new(w, ROW_H - 3.0)))
        .collect();
    let detail = (Vec2::new(x0, list_bottom + 4.0), Vec2::new(w, detail_h));
    let bw = 140.0;
    let back = (Vec2::new(((view.x - bw) * 0.5).round(), view.y - back_h - 8.0), Vec2::new(bw, back_h));
    Layout { title_y, tabs, rows, back, detail }
}

pub enum Input {
    None,
    Moved,
    /// Onglet choisi a la souris.
    Tab(usize),
    Pick(usize),
    Left(usize),
    Right(usize),
    Back,
}

/// `sel` va de 0 a `nrows` (la derniere valeur est le bouton Retour).
pub fn input(nav: &NavIn, view: Vec2, ntabs: usize, nrows: usize, sel: &mut usize) -> Input {
    let l = layout(view, ntabs, nrows, *sel);
    let n = nrows + 1;
    *sel = (*sel).min(nrows);
    if nav.back {
        return Input::Back;
    }
    if let Some(m) = nav.hover {
        for &(k, p, s) in &l.rows {
            if inside(m, p, s) && *sel != k {
                *sel = k;
                return Input::Moved;
            }
        }
        if inside(m, l.back.0, l.back.1) && *sel != nrows {
            *sel = nrows;
            return Input::Moved;
        }
    }
    if nav.click {
        for (k, &(p, s)) in l.tabs.iter().enumerate() {
            if inside(nav.mouse, p, s) {
                return Input::Tab(k);
            }
        }
        for &(k, p, s) in &l.rows {
            if inside(nav.mouse, p, s) {
                return Input::Pick(k);
            }
        }
        if inside(nav.mouse, l.back.0, l.back.1) {
            return Input::Back;
        }
    }
    if nav.up {
        *sel = (*sel + n - 1) % n;
        return Input::Moved;
    }
    if nav.down {
        *sel = (*sel + 1) % n;
        return Input::Moved;
    }
    if nav.left {
        return Input::Left(*sel);
    }
    if nav.right {
        return Input::Right(*sel);
    }
    if nav.confirm {
        return if *sel == nrows { Input::Back } else { Input::Pick(*sel) };
    }
    Input::None
}

fn inside(m: Vec2, p: Vec2, s: Vec2) -> bool {
    m.x >= p.x && m.y >= p.y && m.x < p.x + s.x && m.y < p.y + s.y
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    list: &mut DrawList,
    ctx: &Ctx,
    title: &str,
    corner: Option<(AnimId, String)>,
    tabs: &[String],
    tab: usize,
    rows: &[Row],
    sel: usize,
    detail: &str,
    back: &str,
) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 0.85]);
    let l = layout(view, tabs.len(), rows.len(), sel);
    text(list, main, title, Vec2::new(view.x * 0.5, l.title_y), 2.0, WHITE, Align::Center);
    if let Some((icon, s)) = corner {
        let x = l.tabs.last().map(|t| t.0.x + t.1.x).unwrap_or(view.x - 16.0);
        let w = text(list, main, &s, Vec2::new(x, l.title_y + 8.0), 1.0, GOLD, Align::Right);
        list.frame(atlas.frame(icon, 0.0), Vec2::new(x - w - 10.0, l.title_y + 13.0), &Params::default());
    }
    for (k, (&(p, s), name)) in l.tabs.iter().zip(tabs).enumerate() {
        let on = k == tab;
        list.rect(white, p, s, with_alpha(BACK, if on { 0.95 } else { 0.7 }));
        if on {
            frame_rect(list, white, p, s, with_alpha(GOLD, 0.9));
        }
        list.probe_panel(p, s, "onglet");
        let lines = wrap(main, name, s.x - 8.0, 1.0);
        let shown = if lines.len() > 1 { format!("{}…", lines[0]) } else { name.clone() };
        text(list, main, &shown, Vec2::new(p.x + s.x * 0.5, p.y + 4.0), 1.0, if on { GOLD } else { SOFT }, Align::Center);
    }
    for &(k, p, s) in &l.rows {
        let r = &rows[k];
        let on = k == sel;
        list.rect(white, p, s, with_alpha(PANEL, if on { 1.0 } else { 0.94 }));
        if on {
            frame_rect(list, white, p, s, with_alpha(GOLD, 0.9));
        }
        list.probe_panel(p, s, "ligne");
        let mut x = p.x + 6.0;
        if r.mark {
            // coche
            for (dx, dy) in [(0.0, 4.0), (1.0, 5.0), (2.0, 6.0), (3.0, 5.0), (4.0, 4.0), (5.0, 3.0), (6.0, 2.0), (7.0, 1.0)] {
                list.rect(white, Vec2::new(x + dx, p.y + 5.0 + dy), Vec2::splat(2.0), GREEN);
            }
        }
        x += 12.0;
        if let Some(icon) = r.icon {
            let f = atlas.frame(icon, 0.0);
            // grands sprites (boss) reduits pour tenir dans la ligne
            let k = (f.w.max(f.h) as f32 / 18.0).ceil().max(1.0);
            let at = Vec2::new(x + 8.0, p.y + s.y * 0.5) + Vec2::new(f.px as f32 - f.w as f32 * 0.5, f.py as f32 - f.h as f32 * 0.5) / k;
            // pas encore obtenue : silhouette sombre
            let pr = Params::default().scale(Vec2::splat(1.0 / k));
            let pr = if r.icon_dim { pr.flash([0.1, 0.1, 0.16], 1.0) } else { pr };
            list.frame(f, at.round(), &pr);
            x += 22.0;
        }
        let vw = if r.value.is_empty() { 0.0 } else { main.measure(&r.value) as f32 + 12.0 };
        let room = p.x + s.x - vw - x - 4.0;
        let lines = wrap(main, &r.label, room.max(20.0), 1.0);
        let shown = if lines.len() > 1 { format!("{}…", lines[0]) } else { r.label.clone() };
        let c = if r.dim {
            DIMMED
        } else if on {
            GOLD
        } else {
            SOFT
        };
        text(list, main, &shown, Vec2::new(x, p.y + 4.0), 1.0, c, Align::Left);
        if !r.value.is_empty() {
            let vc = r.value_color.unwrap_or(if r.dim { DIMMED } else { WHITE });
            text(list, main, &r.value, Vec2::new(p.x + s.x - 6.0, p.y + 4.0), 1.0, vc, Align::Right);
        }
    }
    // panneau de detail
    let (dp, ds) = l.detail;
    list.rect(white, dp, ds, PANEL);
    frame_rect(list, white, dp + Vec2::splat(1.0), ds - Vec2::splat(2.0), with_alpha(GOLD, 0.5));
    list.probe_panel(dp, ds, "detail");
    let mut y = dp.y + 5.0;
    for line in wrap(main, detail, ds.x - 16.0, 1.0).iter().take(3) {
        text(list, main, line, Vec2::new(dp.x + 8.0, y), 1.0, SOFT, Align::Left);
        y += 13.0;
    }
    let (bp, bs) = l.back;
    let on = sel >= rows.len();
    list.rect(white, bp, bs, with_alpha(BACK, if on { 0.95 } else { 0.75 }));
    if on {
        frame_rect(list, white, bp, bs, with_alpha(GOLD, 0.9));
    }
    list.probe_panel(bp, bs, "retour");
    text(list, main, back, Vec2::new(bp.x + bs.x * 0.5, bp.y + 4.0), 1.0, if on { GOLD } else { SOFT }, Align::Center);
}
