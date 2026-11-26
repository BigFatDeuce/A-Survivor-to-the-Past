//! Personnages : choix avant la partie, conditions de deblocage (d'apres le profil).
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use crate::lang::Lang;
use crate::menus::{self, NavIn};
use crate::profile::Profile;
use engine::app::Ctx;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::defs::{CharacterDef, Content, Stat, UnlockDef};

const SOFT: Color = rgb(210, 210, 236);
const DIMMED: Color = rgb(120, 116, 140);

/// Personnages proposes, dans l'ordre.
pub fn roster(content: &Content) -> Vec<String> {
    let mut v: Vec<(u32, &String)> = content.characters.iter().map(|(id, c)| (c.order, id)).collect();
    v.sort();
    v.into_iter().map(|(_, id)| id.clone()).collect()
}

/// Univers (pages de l'ecran des personnages), dans l'ordre ; texte `group.<univers>`.
pub const GROUPS: [&str; 6] = ["zelda", "nintendo", "rpg", "games", "screen", "community"];

pub fn group_of(def: &CharacterDef) -> usize {
    GROUPS.iter().position(|g| *g == def.group).unwrap_or(0)
}

/// Personnages d'une page (univers), dans l'ordre.
pub fn page_roster(content: &Content, page: usize) -> Vec<String> {
    roster(content).into_iter().filter(|id| group_of(&content.characters[id]) == page).collect()
}

/// Page et rang d'un personnage.
pub fn locate(content: &Content, id: &str) -> Option<(usize, usize)> {
    let page = group_of(content.characters.get(id)?);
    Some((page, page_roster(content, page).iter().position(|i| i == id)?))
}

/// Personnages en vente (onglet de la boutique), du moins cher au plus cher.
pub fn for_sale(content: &Content) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = roster(content)
        .into_iter()
        .filter_map(|id| match content.characters[&id].unlock {
            Some(UnlockDef::Shop(n)) => Some((id, n)),
            _ => None,
        })
        .collect();
    v.sort_by_key(|(_, n)| *n);
    v
}

/// Avancement vers le deblocage : (fait, objectif) ; `None` si deja disponible d'office.
pub fn progress(content: &Content, def: &CharacterDef, p: &Profile) -> Option<(u64, u64)> {
    let t = &p.totals;
    Some(match def.unlock.as_ref()? {
        UnlockDef::Win(s) => (p.won(s) as u64, 1),
        UnlockDef::Kills(n) => (t.kills.min(*n), *n),
        UnlockDef::Bosses(n) => (t.bosses.min(*n) as u64, *n as u64),
        UnlockDef::Level(n) => (t.best_level.min(*n) as u64, *n as u64),
        UnlockDef::Slain(e) => (p.bestiary.get(e).copied().unwrap_or(0).min(1), 1),
        UnlockDef::Shop(_) => (p.chars_bought.contains(&def.id) as u64, 1),
        UnlockDef::AllOthers => {
            // personnages d'origine seulement (pas les centaines du Randomizer) ; ceux-la n'ont pas
            // cette condition : pas de recursion
            let others: Vec<&CharacterDef> = content
                .characters
                .values()
                .filter(|d| d.order < 1000 && !matches!(d.unlock, Some(UnlockDef::AllOthers | UnlockDef::Shop(_))))
                .collect();
            let done = others.iter().filter(|d| progress(content, d, p).is_none_or(|(a, b)| a >= b)).count();
            (done as u64, others.len() as u64)
        }
    })
}

/// Couleur choisie (indice dans `skins`).
pub fn skin(def: &CharacterDef, p: &Profile, id: &str) -> usize {
    p.skins.get(id).copied().unwrap_or(0).min(def.skins.len().saturating_sub(1))
}

/// Couleurs disponibles : toutes, sauf les tenues de la garde-robe pas encore achetees.
pub fn owned_skins(def: &CharacterDef, p: &Profile, meta: &crate::meta::Meta) -> Vec<usize> {
    (0..def.skins.len()).filter(|&k| !meta.skin_locked(p, &def.skins[k].0)).collect()
}

/// Remplace le sprite des personnages a couleurs par la couleur choisie dans le profil.
pub fn apply_skins(content: &mut Content, p: &Profile) {
    for (id, def) in content.characters.iter_mut() {
        if !def.skins.is_empty() {
            let k = skin(def, p, id);
            def.sprite = def.skins[k].0.clone();
        }
    }
}

pub fn unlocked(content: &Content, def: &CharacterDef, p: &Profile) -> bool {
    progress(content, def, p).is_none_or(|(a, b)| a >= b)
}

pub fn name(lang: &Lang, id: &str) -> String {
    lang.get(&format!("char.{id}.name")).to_string()
}

/// Phrase de deblocage (« Gagner la Plaine d'Hyrule »...).
pub fn hint(lang: &Lang, def: &CharacterDef) -> String {
    match &def.unlock {
        None => String::new(),
        Some(UnlockDef::Win(s)) => lang.get("ui.unlock_win").replace("{s}", &crate::hud::hl(&crate::stages::stage_name(lang, s))),
        Some(UnlockDef::Kills(n)) => lang.get("ui.unlock_kills").replace("{n}", &n.to_string()),
        Some(UnlockDef::Bosses(n)) => lang.get("ui.unlock_bosses").replace("{n}", &n.to_string()),
        Some(UnlockDef::Level(n)) => lang.get("ui.unlock_level").replace("{n}", &n.to_string()),
        Some(UnlockDef::Slain(e)) => lang.get("ui.unlock_slain").replace("{s}", &crate::hud::hl(lang.get(&format!("enemy.{e}")))),
        Some(UnlockDef::AllOthers) => lang.get("ui.unlock_all_others").to_string(),
        Some(UnlockDef::Shop(n)) => lang.get("ui.unlock_shop").replace("{n}", &crate::hud::hl(&n.to_string())),
    }
}

/// Personnages qui viennent d'etre debloques (entre deux etats du profil).
pub fn newly_unlocked(content: &Content, before: &Profile, after: &Profile) -> Vec<String> {
    roster(content)
        .into_iter()
        .filter(|id| {
            let d = &content.characters[id];
            !unlocked(content, d, before) && unlocked(content, d, after)
        })
        .collect()
}

/// Grille des personnages (a gauche), fiche du personnage choisi (a droite), « Retour » sous la grille,
/// choix de l'univers (page) au-dessus. Seules les lignes visibles ont une case (defilement).
pub struct GridLayout {
    /// Cases visibles : la case `k` est le personnage `first + k`.
    pub cells: Vec<(Vec2, Vec2)>,
    pub first: usize,
    pub back: (Vec2, Vec2),
    pub tabs: (Vec2, Vec2),
    pub panel: (Vec2, Vec2),
    pub cols: usize,
    /// Echelle des personnages dans les cases.
    pub scale: f32,
}

// sous le titre et le choix de l'univers
const GRID_TOP: f32 = 54.0;

pub fn grid_layout(view: Vec2, n: usize, sel: usize) -> GridLayout {
    let m = 14.0;
    let gap = 4.0;
    let pw = (view.x * 0.42).clamp(190.0, 300.0).round();
    let gw = view.x - 2.0 * m - 10.0 - pw;
    let room = view.y - GRID_TOP - 10.0 - 30.0;
    // la plus grande case qui laisse tenir toute la grille : personnages x2 si possible, sinon x1
    let mut pick = (Vec2::new(30.0, 36.0), 1usize, 1.0);
    // la plus petite taille ne tient pas : la grille defile
    for (cw, ch, sc) in [(52.0f32, 62.0f32, 2.0f32), (48.0, 58.0, 2.0), (40.0, 46.0, 1.0), (34.0, 40.0, 1.0)] {
        let cols = (((gw + gap) / (cw + gap)).floor() as usize).max(1);
        let rows = n.div_ceil(cols);
        pick = (Vec2::new(cw, ch), cols, sc);
        if rows as f32 * (ch + gap) - gap <= room {
            break;
        }
    }
    let (cell, cols, scale) = pick;
    let cols = cols.min(n.max(1));
    let rows = n.div_ceil(cols).max(1);
    let fit = (((room + gap) / (cell.y + gap)).floor() as usize).clamp(1, rows);
    // la ligne choisie reste visible, au milieu quand c'est possible
    let row = if sel < n { sel / cols } else if sel == n { rows - 1 } else { 0 };
    let top = row.saturating_sub((fit - 1) / 2).min(rows - fit);
    let first = top * cols;
    let used = cols as f32 * (cell.x + gap) - gap;
    let x0 = (m + (gw - used) * 0.5).round();
    let cells = (first..n.min(first + fit * cols))
        .map(|k| {
            let (c, r) = ((k % cols) as f32, (k / cols - top) as f32);
            (Vec2::new(x0 + c * (cell.x + gap), GRID_TOP + r * (cell.y + gap)), cell)
        })
        .collect();
    let by = GRID_TOP + fit as f32 * (cell.y + gap) + 6.0;
    let bw = used.clamp(100.0, 160.0);
    let back = (Vec2::new((m + (gw - bw) * 0.5).round(), by), Vec2::new(bw, 20.0));
    let tw = used.clamp(160.0, gw);
    let tabs = (Vec2::new((m + (gw - tw) * 0.5).round(), GRID_TOP - 18.0), Vec2::new(tw, 15.0));
    let px = m + gw + 10.0;
    let panel = (Vec2::new(px, GRID_TOP - 6.0), Vec2::new(view.x - px - m, view.y - GRID_TOP - 4.0));
    GridLayout { cells, first, back, tabs, panel, cols, scale }
}

/// Fleches : deplacement dans la grille (« Retour » sous la derniere ligne, choix de l'univers au-dessus
/// de la premiere : `n + 1`, gauche / droite y changent de page) ; souris : survol et clic.
/// Rend (case choisie, curseur deplace, changement de page -1 / 0 / 1).
pub fn input_select(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize) -> (Option<usize>, bool, i32) {
    let cols = grid_layout(view, n, *sel).cols;
    let before = *sel;
    let (back, tabs) = (n, n + 1);
    let mut page = 0;
    if *sel == tabs {
        page = nav.right as i32 - nav.left as i32;
        if nav.down {
            *sel = if n > 0 { (cols / 2).min(n - 1) } else { back };
        } else if nav.up {
            *sel = back;
        }
    } else if *sel >= n {
        // sur « Retour »
        if nav.up && n > 0 {
            *sel = (n - 1) / cols * cols + (cols / 2).min((n - 1) % cols);
        } else if nav.up || nav.down {
            *sel = tabs;
        }
    } else {
        if nav.left {
            *sel = if *sel == 0 { n - 1 } else { *sel - 1 };
        }
        if nav.right {
            *sel = if *sel + 1 >= n { 0 } else { *sel + 1 };
        }
        if nav.up {
            *sel = if *sel >= cols { *sel - cols } else { tabs };
        } else if nav.down {
            *sel = if *sel + cols < n {
                *sel + cols
            } else if *sel / cols < (n - 1) / cols {
                n - 1
            } else {
                back
            };
        }
        if nav.wheel != 0 {
            *sel = (*sel as i64 + nav.wheel as i64 * cols as i64).clamp(0, n as i64 - 1) as usize;
        }
    }
    let l = grid_layout(view, n, *sel);
    let rects: Vec<(usize, (Vec2, Vec2))> = (0..l.cells.len()).map(|k| (l.first + k, l.cells[k])).chain([(back, l.back), (tabs, l.tabs)]).collect();
    let inside = |m: Vec2, (p, s): (Vec2, Vec2)| m.x >= p.x && m.y >= p.y && m.x < p.x + s.x && m.y < p.y + s.y;
    if let Some(m) = nav.hover {
        if let Some(&(k, _)) = rects.iter().find(|(_, r)| inside(m, *r)) {
            *sel = k;
        }
    }
    let mut pick = None;
    if nav.click {
        if let Some(&(k, r)) = rects.iter().find(|(_, r)| inside(nav.mouse, *r)) {
            if k == tabs {
                page = if nav.mouse.x < r.0.x + r.1.x * 0.5 { -1 } else { 1 };
            } else {
                pick = Some(k);
            }
        }
    }
    let pick = pick.or((nav.confirm && *sel != tabs).then_some(*sel));
    (pick, *sel != before, page)
}

/// Teinte d'une couleur de personnage (bouton des couleurs).
fn skin_rgb(key: &str) -> Color {
    match key {
        "green" => rgb(72, 184, 72),
        "red" => rgb(224, 64, 56),
        "blue" => rgb(72, 120, 240),
        "purple" => rgb(176, 88, 224),
        "pink" => rgb(237, 84, 186),
        "orange" => rgb(239, 125, 13),
        "yellow" => rgb(232, 224, 72),
        "cyan" => rgb(56, 220, 200),
        // Link's Awakening : Game Boy Color (tunique verte vive), Game Boy (vert pale de l'ecran)
        "dx" => rgb(64, 200, 96),
        "gb" => rgb(150, 190, 110),
        "white" => rgb(236, 236, 244),
        "black" => rgb(70, 66, 84),
        "gray" => rgb(150, 150, 160),
        "brown" => rgb(150, 96, 56),
        "dark" => rgb(120, 70, 170),
        _ => rgb(220, 220, 220),
    }
}

/// Portrait de la fiche : position, taille (le personnage y marche et tourne sur lui-meme).
fn portrait_box(l: &GridLayout) -> (Vec2, Vec2, f32) {
    let (pp, ps) = l.panel;
    let sc: f32 = if ps.y > 300.0 { 3.0 } else { 2.0 };
    let bh = (30.0 * sc + 8.0).round();
    let bw = (ps.x - 24.0).min(bh * 2.0).round();
    (Vec2::new((pp.x + (ps.x - bw) * 0.5).round(), pp.y + 32.0), Vec2::new(bw, bh), sc)
}

/// Bouton des couleurs (personnages qui en proposent plusieurs), juste sous le portrait.
pub fn skin_button(ctx: &Ctx, content: &Content, meta: &crate::meta::Meta, profile: &Profile, ids: &[String], sel: usize) -> Option<(Vec2, Vec2)> {
    let id = ids.get(sel)?;
    let def = &content.characters[id];
    if owned_skins(def, profile, meta).len() < 2 || !unlocked(content, def, profile) {
        return None;
    }
    let l = grid_layout(ctx.view, ids.len(), sel);
    let (bp, bs, _) = portrait_box(&l);
    Some((Vec2::new(bp.x, bp.y + bs.y + 4.0), Vec2::new(bs.x, 16.0)))
}

/// Petit triangle plein (fleche du bouton des couleurs), pointe vers `dir` (-1 gauche, 1 droite).
fn arrow(list: &mut DrawList, white: &assetkit::meta::Frame, tip: Vec2, dir: f32, c: Color) {
    for k in 0..4 {
        let h = 1.0 + 2.0 * k as f32;
        list.rect(white, Vec2::new(tip.x - dir * k as f32 - if dir > 0.0 { 1.0 } else { 0.0 }, tip.y - k as f32), Vec2::new(1.0, h), c);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_select(list: &mut DrawList, ctx: &Ctx, lang: &Lang, content: &Content, meta: &crate::meta::Meta, profile: &Profile, page: usize, sel: usize, t: f32) {
    let ids = &page_roster(content, page);
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    let lh = main.line_height as f32;
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 0.55]);
    let l = grid_layout(view, ids.len(), sel);
    text(list, main, lang.get("ui.characters"), Vec2::new(view.x * 0.5, 8.0), 2.0, WHITE, Align::Center);
    let rt = ctx.real_time as f32;
    // univers : fleches aux deux bouts, nom et personnages debloques de la page
    let (tp, ts) = l.tabs;
    let on = sel == ids.len() + 1;
    list.rect(white, tp, ts, with_alpha(BACK, if on { 0.95 } else { 0.8 }));
    frame_rect(list, white, tp, ts, with_alpha(GOLD, if on { 0.0 } else { 0.35 }));
    if on {
        crate::hub::focus_frame(list, ctx, tp, ts);
    }
    list.probe_panel(tp, ts, "univers");
    let mid = (tp.y + ts.y * 0.5).floor();
    arrow(list, white, Vec2::new(tp.x + 5.0, mid), -1.0, if on { GOLD } else { SOFT });
    arrow(list, white, Vec2::new(tp.x + ts.x - 5.0, mid), 1.0, if on { GOLD } else { SOFT });
    let open_n = ids.iter().filter(|id| unlocked(content, &content.characters[*id], profile)).count();
    let label = format!("{} ({open_n}/{})", lang.get(&format!("group.{}", GROUPS[page.min(GROUPS.len() - 1)])), ids.len());
    text(list, main, &label, Vec2::new(tp.x + ts.x * 0.5, (tp.y + (ts.y - lh) * 0.5).round()), 1.0, if on { GOLD } else { SOFT }, Align::Center);
    // grille : chaque personnage de face ; celui sous le curseur marche
    for (j, &(p, s)) in l.cells.iter().enumerate() {
        let k = l.first + j;
        let id = &ids[k];
        let def = &content.characters[id];
        let open = unlocked(content, def, profile);
        let on = k == sel;
        list.rect(white, p, s, with_alpha(BACK, if on { 0.95 } else { 0.75 }));
        if open {
            frame_rect(list, white, p, s, with_alpha(GOLD, if on { 0.0 } else { 0.25 }));
        }
        if on {
            list.set(Space::Ui, Blend::Add);
            list.rect(white, p + Vec2::ONE, s - Vec2::splat(2.0), [1.0, 0.85, 0.4, 0.08 + 0.05 * (rt * 4.0).sin()]);
            list.set(Space::Ui, Blend::Alpha);
            crate::hub::focus_frame(list, ctx, p, s);
        }
        if let Some(a) = atlas.find(&def.sprite, "walk_down") {
            let f = atlas.frame(a, if on && open { t } else { 0.0 });
            let sc = if (f.h as f32) * l.scale > s.y - 4.0 { 1.0 } else { l.scale };
            let mut prm = Params::default().scale(Vec2::splat(sc));
            if !open {
                // silhouette grisee, lisible sur le fond sombre de la case
                prm = prm.flash([0.24, 0.22, 0.32], 1.0);
            }
            let bob = if on && open { ((t * 6.0).sin() * 1.0).round() } else { 0.0 };
            list.frame(f, Vec2::new(p.x + s.x * 0.5, p.y + s.y - 4.0 + bob).round(), &prm);
        }
        if !open {
            crate::stages::padlock(list, white, p + Vec2::new(s.x - 10.0, 3.0), with_alpha(DIMMED, 0.9));
        } else if profile.new_chars.contains(id) {
            // pas encore joue : etincelle verte dans le coin
            let a = 0.6 + 0.4 * (rt * 5.0 + k as f32).sin();
            let c = p + Vec2::new(s.x - 6.0, 5.0);
            list.set(Space::Ui, Blend::Add);
            list.rect(white, c - Vec2::new(2.0, 0.0), Vec2::new(5.0, 1.0), [0.4, 1.0, 0.5, a]);
            list.rect(white, c - Vec2::new(0.0, 2.0), Vec2::new(1.0, 5.0), [0.4, 1.0, 0.5, a]);
            list.set(Space::Ui, Blend::Alpha);
        }
    }
    // Retour
    let (bp, bs) = l.back;
    let on = sel == ids.len();
    list.rect(white, bp, bs, with_alpha(BACK, if on { 0.95 } else { 0.8 }));
    if on {
        crate::hub::focus_frame(list, ctx, bp, bs);
    }
    list.probe_panel(bp, bs, "retour");
    let ty = (bp.y + (bs.y - lh) * 0.5).round();
    text(list, main, lang.get("ui.back"), Vec2::new(bp.x + bs.x * 0.5, ty), 1.0, if on { GOLD } else { SOFT }, Align::Center);

    // fiche du personnage
    let (pp, ps) = l.panel;
    list.rect(white, pp, ps, with_alpha(BACK, 0.92));
    frame_rect(list, white, pp + Vec2::splat(2.0), ps - Vec2::splat(4.0), with_alpha(GOLD, 0.7));
    list.probe_panel(pp, ps, "panneau du personnage");
    let k = sel.min(ids.len().saturating_sub(1));
    let Some(id) = ids.get(k) else { return };
    let def = &content.characters[id];
    let open = unlocked(content, def, profile);
    let x = pp.x + 12.0;
    let w = ps.x - 24.0;
    let title = if open { name(lang, id) } else { lang.get("ui.locked").to_string() };
    let big = main.measure(&title) as f32 * 2.0 <= w;
    let title = if open && profile.new_chars.contains(id) { crate::hud::hl_new(&title) } else { title };
    let tsc = if big { 2.0 } else { 1.0 };
    text(list, main, &title, Vec2::new(pp.x + ps.x * 0.5, pp.y + 7.0 + if big { 0.0 } else { 5.0 }), tsc, if open { GOLD } else { DIMMED }, Align::Center);
    let (bp, bsize, sc) = portrait_box(&l);
    let (bw, bh) = (bsize.x, bsize.y);
    list.rect(white, bp, bsize, [0.0, 0.0, 0.0, 0.35]);
    frame_rect(list, white, bp, bsize, with_alpha(GOLD, 0.35));
    list.probe_block(bp, bsize, "portrait");
    let turn = ((t / 1.4) as usize) % 4;
    let (anim, side) = match turn {
        0 => ("walk_down", 0),
        1 => ("walk_side", 1),
        2 => ("walk_up", 0),
        _ => ("walk_side", -1),
    };
    if let Some(a) = atlas.find(&def.sprite, anim) {
        let f = atlas.frame(a, t);
        // le profil d'origine regarde a gauche ou a droite selon la planche
        let flip = match side {
            1 => def.side_faces_left,
            -1 => !def.side_faces_left,
            _ => false,
        };
        let mut prm = Params::default().scale(Vec2::splat(sc)).flip(flip);
        if !open {
            prm = prm.flash([0.08, 0.06, 0.14], 1.0);
        }
        list.frame(f, Vec2::new(bp.x + bw * 0.5, bp.y + bh - 4.0).round(), &prm);
    }
    let mut y = bp.y + bh + 8.0;
    // bouton des couleurs : fleches dessinees aux deux bouts, nom centre (verifie par --uicheck)
    if let Some((cp, cs)) = skin_button(ctx, content, meta, profile, ids, sel) {
        let owned = owned_skins(def, profile, meta);
        let n = owned.len();
        let cur = skin(def, profile, id);
        let phase = rt / 1.6;
        let a = owned[(phase as usize) % n];
        let b = owned[(phase as usize + 1) % n];
        let u = ((phase.fract() - 0.5) * 2.0).clamp(0.0, 1.0);
        let u = u * u * (3.0 - 2.0 * u);
        let ca = skin_rgb(&def.skins[a].1);
        let cb = skin_rgb(&def.skins[b].1);
        let breath = 0.75 + 0.25 * (rt * 2.4).sin();
        let fill = [
            (ca[0] + (cb[0] - ca[0]) * u) * breath,
            (ca[1] + (cb[1] - ca[1]) * u) * breath,
            (ca[2] + (cb[2] - ca[2]) * u) * breath,
            0.9,
        ];
        list.rect(white, cp, cs, fill);
        list.rect(white, cp + Vec2::ONE, cs - Vec2::splat(2.0), [0.0, 0.0, 0.0, 0.35]);
        frame_rect(list, white, cp, cs, with_alpha(WHITE, 0.6 + 0.3 * breath));
        list.probe_panel(cp, cs, "couleur");
        let mid = (cp.y + cs.y * 0.5).floor();
        arrow(list, white, Vec2::new(cp.x + 5.0, mid), -1.0, WHITE);
        arrow(list, white, Vec2::new(cp.x + cs.x - 5.0, mid), 1.0, WHITE);
        let c = lang.get(&format!("skin.{}", def.skins[cur].1));
        let ty = (cp.y + (cs.y - lh) * 0.5).round();
        text(list, main, c, Vec2::new(cp.x + cs.x * 0.5, ty), 1.0, WHITE, Align::Center);
        text(list, main, lang.get("ui.skin_keys"), Vec2::new(cp.x + cs.x * 0.5, cp.y + cs.y + 3.0), 1.0, DIMMED, Align::Center);
        y = cp.y + cs.y + 18.0;
    }
    let bottom = pp.y + ps.y - 10.0;
    if !open {
        for line in wrap(main, &hint(lang, def), w, 1.0) {
            text(list, main, &line, Vec2::new(x, y), 1.0, SOFT, Align::Left);
            y += 13.0;
        }
        if let Some((a, b)) = progress(content, def, profile).filter(|p| p.1 > 1) {
            let s = format!("{a} / {b}");
            text(list, main, &s, Vec2::new(x, y), 1.0, GOLD, Align::Left);
        }
        return;
    }
    let mut rows: Vec<(String, String)> = vec![
        (lang.get("ui.char_health").to_string(), format!("{}", (def.max_hp / 10.0).round() as u32)),
        (lang.get("ui.char_speed").to_string(), format!("{}", def.speed.round() as u32)),
    ];
    let weapon = lang.get(&format!("weapon.{}.name", def.weapon)).to_string();
    rows.push((lang.get("ui.char_weapon").to_string(), weapon));
    let n = rows.len();
    for (r, (label, value)) in rows.into_iter().enumerate() {
        if y + 12.0 > bottom {
            return;
        }
        text(list, main, &label, Vec2::new(x, y), 1.0, DIMMED, Align::Left);
        let vw = text(list, main, &value, Vec2::new(x + w, y), 1.0, WHITE, Align::Right);
        // arme de depart : son icone devant le nom
        if r + 1 == n {
            let icon = content.weapon_index(&def.weapon).and_then(|i| atlas.find(&content.weapons[i as usize].icon, "0"));
            if let Some(a) = icon {
                let at = Vec2::new((x + w - vw - 12.0).round(), y + 6.0);
                list.frame(atlas.frame(a, 0.0), at, &Params::default());
                list.probe_block(at - Vec2::splat(8.0), Vec2::splat(16.0), "icone de l'arme");
            }
        }
        y += 15.0;
    }
    for &(s, v) in &def.stats {
        if y + 12.0 > bottom {
            return;
        }
        let line = menus::stat_text(lang, s, v);
        // une recharge plus courte est un avantage : vert pour les avantages, rouge pour les defauts
        let good = if matches!(s, Stat::Cooldown | Stat::DashCooldown) { v < 0.0 } else { v > 0.0 };
        let c = if good { rgb(150, 230, 150) } else { rgb(240, 130, 120) };
        text(list, main, &line, Vec2::new(x, y), 1.0, c, Align::Left);
        y += 13.0;
    }
    y += 3.0;
    let mut desc = lang.get(&format!("char.{id}.desc")).to_string();
    if !def.credit.is_empty() {
        desc = format!("{desc} {}", lang.get("ui.char_credit").replace("{s}", &def.credit));
    }
    for line in wrap(main, &desc, w, 1.0) {
        if y + 12.0 > bottom {
            break;
        }
        text(list, main, &line, Vec2::new(x, y), 1.0, SOFT, Align::Left);
        y += 13.0;
    }
}

#[cfg(test)]
mod tests {
    use crate::profile::Profile;
    use sim::pickups::PickupKind;
    use sim::{Event, World};
    use std::path::Path;

    fn content() -> sim::defs::Content {
        crate::content::load_content(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    }

    #[test]
    fn characters_load_and_unlock() {
        let c = content();
        let ids = super::roster(&c);
        assert_eq!(ids.first().map(String::as_str), Some("link"));
        assert!(ids.len() >= 3, "{ids:?}");
        let mut p = Profile::new();
        let before = p.clone();
        assert!(super::unlocked(&c, &c.characters["link"], &p));
        assert!(!super::unlocked(&c, &c.characters["link_minish"], &p));
        p.stages.entry("hyrule_field".into()).or_default().wins = 1;
        p.totals.kills = 5000;
        let new = super::newly_unlocked(&c, &before, &p);
        assert!(new.contains(&"link_minish".to_string()) && new.contains(&"link_capless".to_string()), "{new:?}");
        // Shadow Link : il faut avoir vaincu Ganon
        assert!(!super::unlocked(&c, &c.characters["shadow_link"], &p));
        p.bestiary.insert("ganon".into(), 1);
        assert!(super::unlocked(&c, &c.characters["shadow_link"], &p));
        // Ganondorf : toujours le dernier, debloque seulement quand tous les autres le sont
        assert_eq!(super::roster(&c).into_iter().filter(|id| c.characters[id].order < 1000).last().as_deref(), Some("ganondorf"));
        let mut all = Profile::new();
        assert!(!super::unlocked(&c, &c.characters["ganondorf"], &all));
        all.totals.kills = 1_000_000;
        all.totals.bosses = 1000;
        all.totals.best_level = 1000;
        for d in c.characters.values() {
            if let Some(sim::defs::UnlockDef::Win(s)) = &d.unlock {
                all.stages.entry(s.clone()).or_default().wins = 1;
            }
        }
        for e in c.enemy_ids.iter() {
            all.bestiary.insert(e.clone(), 1);
        }
        assert!(super::unlocked(&c, &c.characters["ganondorf"], &all));
        // couleurs : le choix du profil remplace le sprite
        let mut c2 = c.clone();
        p.skins.insert("link_fsa".into(), 1);
        super::apply_skins(&mut c2, &p);
        assert_eq!(c2.characters["link_fsa"].sprite, "link_fsa_blue");
        // 0.6.5 : personnages du Randomizer, chacun dans une page, un tiers en boutique
        let sale = super::for_sale(&c);
        assert!(sale.len() > 50 && sale.windows(2).all(|w| w[0].1 <= w[1].1), "{}", sale.len());
        let paged: usize = (0..super::GROUPS.len()).map(|g| super::page_roster(&c, g).len()).sum();
        assert_eq!(paged, ids.len());
        assert_eq!(super::locate(&c, "mario").map(|l| l.0), Some(1));
        let atlas = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/gen/atlas.ron")).unwrap();
        for d in c.characters.values() {
            for (s, _) in d.skins.iter().chain([(d.sprite.clone(), String::new())].iter()) {
                assert!(atlas.contains(&format!("\"{s}\": {{")), "sprite absent de l'atlas : {s}");
            }
        }
        // chaque personnage demarre une partie (sprites, arme de depart, bonus)
        for id in &ids {
            let w = World::new(c.clone(), id, "hyrule_field", 1);
            assert_eq!(w.weapons.len(), 1, "{id}");
            assert!(w.player.max_hp > 0.0);
        }
    }

    #[test]
    fn bobomb_clears_screen_but_not_bosses() {
        let c = content();
        let mut w = World::new(c.clone(), "link", "hyrule_field", 3);
        w.spawning = false;
        let soldier = c.enemy_index("soldier_green").unwrap();
        let boss = c.enemy_index("hinox").unwrap();
        let bomb = w.bomb_kind.expect("bob-omb");
        for k in 0..20 {
            let at = w.player.pos + engine::Vec2::new(-150.0 + k as f32 * 15.0, 60.0);
            w.spawn_enemy(soldier, at);
        }
        let far = w.spawn_enemy(soldier, w.player.pos + engine::Vec2::new(2000.0, 0.0));
        let b = w.spawn_enemy(boss, w.player.pos + engine::Vec2::new(0.0, -80.0));
        let i = w.spawn_enemy(bomb, w.player.pos + engine::Vec2::new(40.0, 0.0));
        assert_eq!(c.enemies[bomb as usize].damage, 0.0);
        w.damage_enemy(i, 999.0, false, engine::Vec2::X, 0.0, 0);
        let cleared = w.events.iter().find_map(|e| match e {
            Event::ScreenClear { count, .. } => Some(*count),
            _ => None,
        });
        assert_eq!(cleared, Some(20));
        assert!(w.horde.hp[b] > 0.0, "le boss survit");
        assert!(w.horde.hp[far] > 0.0, "hors de l'ecran : epargne");
    }

    #[test]
    fn encounter_champion_waits_then_gives_a_reward() {
        use crate::reward::{self, Reward};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let c = content();
        let meta = crate::meta::Meta::load(&root);
        let mut w = World::new(c, "link", "hyrule_field", 7);
        w.player.god = true;
        w.encounter_t = 0.0;
        let input = sim::PlayerInput::default();
        w.tick(&input);
        assert!(w.events.iter().any(|e| matches!(e, Event::EncounterSpawned { .. })));
        let anchor = w.encounter.expect("rencontre").anchor;
        // loin du joueur : le champion attend sur place
        for _ in 0..60 {
            w.tick(&input);
        }
        let at = w.champion_pos().expect("champion");
        assert!((at - anchor).length() < 0.01, "il attend : {at} {anchor}");
        let i = w.horde.uid.iter().position(|&u| u == w.encounter.unwrap().uid).unwrap();
        assert!(w.horde.elite[i]);
        w.damage_enemy(i, 1e6, false, engine::Vec2::X, 0.0, 0);
        assert!(w.events.iter().any(|e| matches!(e, Event::EncounterWon { .. })));
        assert_eq!(w.pending_rewards, 1);
        assert!(w.encounter.is_none());
        // recompenses : une amelioration d'arme, une relique de rencontre, une benediction
        let mut rng = sim::rng::Rng::new(3);
        let p = Profile::new();
        let offers = reward::roll(&w, &meta, &p, &[], &mut rng);
        assert_eq!(offers.len(), 3);
        assert!(matches!(offers[0], Reward::Boost { .. }));
        let Reward::Relic(k) = offers[1] else { panic!("{offers:?}") };
        assert!(meta.relics[k].encounter);
        let before = w.weapons[0].stats.damage;
        reward::apply(&mut w, &meta, Reward::Boost { slot: 0, boost: sim::upgrades::WeaponBoost { damage: 0.3, ..Default::default() } });
        assert!((w.weapons[0].stats.damage - before * 1.3).abs() < 1e-3);
        let hp = w.player.max_hp;
        assert_eq!(reward::apply(&mut w, &meta, Reward::Blessing(sim::defs::Stat::MaxHp, 20.0)), None);
        assert!(w.player.max_hp > hp);
        assert_eq!(reward::apply(&mut w, &meta, Reward::Relic(k)), Some(meta.relics[k].id.clone()));
        // deja choisie pendant la partie : plus proposee
        let taken = vec![meta.relics[k].id.clone()];
        for _ in 0..30 {
            let o = reward::roll(&w, &meta, &p, &taken, &mut rng);
            assert!(!o.contains(&Reward::Relic(k)));
        }
    }

    #[test]
    fn bushes_break_and_drop_items() {
        let c = content();
        let mut w = World::new(c, "link", "hyrule_field", 5);
        // beaucoup de chance : presque chaque buisson laisse un objet
        w.player.stats.luck = 50.0;
        for k in 0..40 {
            let at = engine::Vec2::new(-1000.0 + k as f32 * 50.0, 400.0);
            w.hit_props(at, 120.0);
        }
        let broken = w.events.iter().filter(|e| matches!(e, Event::PropBroken { .. })).count();
        assert!(broken > 3, "buissons casses : {broken}");
        assert_eq!(broken, w.broken.len());
        assert!(w.pickups.len() > broken / 2, "objets : {}", w.pickups.len());
        // une case cassee ne se recasse pas
        let n = w.events.len();
        w.hit_props(engine::Vec2::new(-1000.0, 400.0), 120.0);
        assert_eq!(w.events.len(), n);
        let _ = PickupKind::Fairy;
    }
    /// Un objet partenaire ne sert qu'a une seule fusion par partie ; les armes de la 0.6.2 frappent.
    /// Fusion parmi les choix de niveau : jamais avant que l'arme ET son objet soient au maximum
    /// (la condition du coffre) ; ensuite proposee, et elle passe avant les soins / rubis.
    #[test]
    fn fusion_offered_only_when_chest_could_give_it() {
        use sim::upgrades::Offer;
        let c = content();
        let mut w = World::new(c.clone(), "link", "hyrule_field", 9);
        w.spawning = false;
        let bow = c.weapon_index("bow").unwrap();
        let quiver = c.passive_index("quiver").unwrap();
        let silver = c.weapon_index("silver_bow").unwrap();
        let has_evo = |w: &mut World| (0..300).any(|_| w.roll_offers().contains(&Offer::Evolve(silver)));
        w.add_weapon(bow);
        while w.weapon_level(bow).unwrap() < c.weapons[bow as usize].max_level() {
            w.apply_offer(Offer::UpgradeWeapon(bow));
        }
        w.apply_offer(Offer::NewPassive(quiver));
        assert!(!has_evo(&mut w), "carquois pas encore au maximum : pas de fusion");
        while w.passive_level(quiver).unwrap() < c.passives[quiver as usize].max_level() {
            w.apply_offer(Offer::UpgradePassive(quiver));
        }
        assert!(w.evolution_ready().is_some());
        assert!(has_evo(&mut w), "pret comme pour un coffre : la fusion peut etre proposee");
        // tout le reste rempli au maximum : la fusion passe avant soins et rubis
        for _ in 0..2000 {
            let o = w.roll_offers().into_iter().find(|o| !matches!(o, Offer::Evolve(_) | Offer::Heal | Offer::Rupees));
            match o {
                Some(o) => w.apply_offer(o),
                None if w.roll_offers().iter().all(|o| matches!(o, Offer::Evolve(_))) => break,
                None => {}
            }
        }
        let offers = w.roll_offers();
        assert!(offers.contains(&Offer::Evolve(silver)), "{offers:?}");
        assert!(!offers.contains(&Offer::Heal) && !offers.contains(&Offer::Rupees));
        w.apply_offer(Offer::Evolve(silver));
        assert!(w.weapons.iter().any(|s| s.id == silver), "l'arc est devenu l'arc d'argent");
        assert!(!w.roll_offers().contains(&Offer::Evolve(silver)));
    }

    #[test]
    fn one_fusion_per_partner_and_new_weapons_hit() {
        use sim::upgrades::Offer;
        let c = content();
        let mut w = World::new(c.clone(), "link", "hyrule_field", 5);
        w.spawning = false;
        let bow = c.weapon_index("bow").unwrap();
        let sling = c.weapon_index("slingshot").unwrap();
        let quiver = c.passive_index("quiver").unwrap();
        w.add_weapon(bow);
        w.add_weapon(sling);
        for wid in [bow, sling] {
            while w.weapon_level(wid).unwrap() < c.weapons[wid as usize].max_level() {
                w.apply_offer(Offer::UpgradeWeapon(wid));
            }
        }
        w.apply_offer(Offer::NewPassive(quiver));
        while w.passive_level(quiver).unwrap() < c.passives[quiver as usize].max_level() {
            w.apply_offer(Offer::UpgradePassive(quiver));
        }
        // les deux sont pretes : le joueur choisit (ici le lance-pierre, deuxieme emplacement)
        let choices = w.evolution_choices();
        assert_eq!(choices.len(), 2, "{choices:?}");
        let slot = w.weapons.iter().position(|s| s.id == sling).unwrap();
        let hyper = c.weapon_index("hyper_slingshot").unwrap();
        assert_eq!(w.open_chest_choice(false, Some(slot)).evolution, Some((sling, hyper)), "fusion choisie");
        assert_eq!(w.weapon_level(bow), Some(c.weapons[bow as usize].max_level()), "l'arc reste tel quel");
        assert!(w.evolution_ready().is_none(), "le carquois a deja servi");
        assert!(w.open_chest(true).evolution.is_none());
        assert!(!w.partner_free(quiver));

        let soldier = c.enemy_index("soldier_green").unwrap();
        for id in ["slingshot", "hyper_slingshot", "roc_feather", "roc_cape", "lantern", "blue_lantern", "gust_jar", "vaati_jar", "ocarina", "ocarina_of_time", "din_fire", "din_wrath"] {
            let mut w = World::new(c.clone(), "link", "hyrule_field", 7);
            w.spawning = false;
            w.weapons.clear();
            w.add_weapon(c.weapon_index(id).unwrap_or_else(|| panic!("{id}")));
            w.player.invuln = 1e9;
            let mut ids = Vec::new();
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                ids.push(w.spawn_enemy(soldier, w.player.pos + engine::Vec2::from_angle(a) * 40.0));
            }
            let before: f32 = ids.iter().map(|&i| w.horde.hp[i]).sum();
            for _ in 0..360 {
                w.tick(&sim::PlayerInput::default());
            }
            let after: f32 = ids.iter().map(|&i| w.horde.hp.get(i).copied().unwrap_or(0.0)).sum();
            assert!(w.player.kills > 0 || after < before, "{id} ne touche rien");
        }
    }

    #[test]
    fn finished_burn_does_not_keep_its_strength() {
        use sim::defs::StatusDef;
        let c = content();
        let mut w = World::new(c.clone(), "link", "hyrule_field", 3);
        w.spawning = false;
        w.weapons.clear();
        let soldier = c.enemy_index("soldier_green").unwrap();
        let i = w.spawn_enemy(soldier, w.player.pos + engine::Vec2::new(60.0, 0.0));
        w.horde.hp[i] = 1e9;
        w.horde.max_hp[i] = 1e9;
        let might = w.player.stats.might;
        let uid = w.horde.uid[i];
        // plusieurs durees : la fin de brulure ne tombe pas toujours sur un pas de degats (seul cas
        // ou l'ancien code remettait la force a zero)
        for time in [0.3, 0.4, 0.45, 0.55] {
            let j = w.horde.uid.iter().position(|&u| u == uid).unwrap();
            w.apply_status(j, StatusDef::Burn { dps: 20.0, time });
            // une brulure plus faible pendant la forte : la forte reste
            w.apply_status(j, StatusDef::Burn { dps: 2.0, time });
            assert_eq!(w.horde.burn_dps[j], 20.0 * might);
            for _ in 0..60 {
                w.tick(&sim::PlayerInput::default());
            }
            let j = w.horde.uid.iter().position(|&u| u == uid).unwrap();
            assert!(w.horde.burn[j] <= 0.0);
            // brulure terminee : une faible arrive, elle ne prend pas la force de l'ancienne
            w.apply_status(j, StatusDef::Burn { dps: 2.0, time: 0.1 });
            assert_eq!(w.horde.burn_dps[j], 2.0 * might, "duree {time} : la force de l'ancienne brulure est restee");
            for _ in 0..20 {
                w.tick(&sim::PlayerInput::default());
            }
        }
    }

    #[test]
    fn locked_weapons_wait_for_their_mini_boss() {
        use sim::upgrades::Offer;
        let c = content();
        let locked: Vec<u16> = (0..c.weapons.len() as u16).filter(|&k| c.weapons[k as usize].unlock.is_some()).collect();
        // six armes de mini-boss, plus la Master Sword (fusion ultime, Ganon vaincu)
        assert_eq!(locked.iter().filter(|&&k| !c.weapons[k as usize].evolved).count(), 6, "une arme par mini-boss");
        let ultimate: Vec<&str> = locked.iter().filter(|&&k| c.weapons[k as usize].evolved).map(|&k| c.weapon_ids[k as usize].as_str()).collect();
        assert_eq!(ultimate, ["master_sword"]);
        assert_eq!(c.weapons[c.weapon_index("master_sword").unwrap() as usize].unlock.as_deref(), Some("ganon"));
        for &k in &locked {
            let boss = c.weapons[k as usize].unlock.as_ref().unwrap();
            let e = c.enemy_index(boss).unwrap_or_else(|| panic!("ennemi inconnu : {boss}"));
            assert!(c.enemies[e as usize].boss, "{boss} n'est pas un boss");
        }
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..40 {
            let mut w = World::new(c.clone(), "link", "hyrule_field", seed);
            w.locked_weapons = locked.clone();
            for _ in 0..30 {
                for o in w.roll_offers() {
                    if let Offer::NewWeapon(k) = o {
                        assert!(!locked.contains(&k), "arme verrouillee proposee : {}", c.weapon_ids[k as usize]);
                        seen.insert(k);
                    }
                }
            }
        }
        assert!(!seen.is_empty());
        // une fois debloquee, l'arme peut sortir
        let sling = c.weapon_index("slingshot").unwrap();
        let mut found = false;
        for seed in 0..80 {
            let mut w = World::new(c.clone(), "link", "hyrule_field", seed);
            w.locked_weapons = locked.iter().copied().filter(|&k| k != sling).collect();
            found |= (0..30).any(|_| w.roll_offers().contains(&Offer::NewWeapon(sling)));
        }
        assert!(found, "le lance-pierre debloque n'est jamais propose");
    }
}
