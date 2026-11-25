//! Stages : choix du stage (deblocage en gagnant le precedent) et ecran de victoire.
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use crate::lang::Lang;
use crate::menus::{input_rects, NavIn};
use crate::profile::Profile;
use engine::app::Ctx;
use engine::assets::Atlas;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::defs::{Content, MapDef};
use sim::World;

const SOFT: Color = rgb(210, 210, 236);
const DIMMED: Color = rgb(120, 116, 140);
const WIN: Color = rgb(250, 214, 90);

/// Stages proposes, dans l'ordre (les cartes de rang 0 sont des arenes d'essai).
pub fn stage_ids(content: &Content) -> Vec<String> {
    let mut v: Vec<(&u32, &String)> = content.maps.iter().filter(|(_, m)| m.order > 0).map(|(id, m)| (&m.order, id)).collect();
    v.sort();
    v.into_iter().map(|(_, id)| id.clone()).collect()
}

/// Un stage est ouvert si c'est le premier ou si le precedent a ete gagne.
pub fn unlocked(ids: &[String], profile: &Profile, k: usize) -> bool {
    k == 0 || ids.get(k - 1).is_some_and(|p| profile.won(p))
}

/// Premier stage ouvert et pas encore gagne (sinon le dernier ouvert).
pub fn suggested(ids: &[String], profile: &Profile) -> usize {
    (0..ids.len()).find(|&k| unlocked(ids, profile, k) && !profile.won(&ids[k])).unwrap_or_else(|| {
        (0..ids.len()).rev().find(|&k| unlocked(ids, profile, k)).unwrap_or(0)
    })
}

/// Tuile du sol en (tx, ty) : surtout la tuile de base, quelques touffes, fleurs et buissons.
pub fn ground_index(tx: i32, ty: i32, n: usize) -> usize {
    let h = (tx.wrapping_mul(73856093) ^ ty.wrapping_mul(19349663)) as u32;
    let h = h.wrapping_mul(2654435761) >> 16;
    let r = h % 1000;
    let idx = match r {
        0..=919 => 0,
        920..=969 => 1 + (h / 1000 % 3) as usize,
        970..=996 => 4 + (h / 1000 % 2) as usize,
        _ => 6,
    };
    idx.min(n.saturating_sub(1))
}

pub fn fmt_time(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn stage_name(lang: &Lang, id: &str) -> String {
    lang.get(&format!("map.{id}.name")).to_string()
}

// ---------------------------------------------------------------------------------------------
// Choix du stage

pub struct SelectLayout {
    /// Lignes puis « Retour ». Les lignes hors de la fenetre de defilement sont rangees hors de l'ecran
    /// (`visible` faux) : elles ne se dessinent pas et ne captent pas la souris.
    pub items: Vec<(Vec2, Vec2)>,
    pub panel: (Vec2, Vec2),
    pub title_y: f32,
    /// D'autres lignes au-dessus / au-dessous de la fenetre.
    pub more: (bool, bool),
}

impl SelectLayout {
    pub fn visible(&self, k: usize) -> bool {
        self.items.get(k).is_some_and(|(p, _)| p.y >= 0.0)
    }
}

const ROW: f32 = 26.0;

/// Liste a gauche (defilante quand elle est trop longue pour la vue), panneau de details a droite.
pub fn select_layout(view: Vec2, n: usize, sel: usize) -> SelectLayout {
    let m = 14.0;
    let lw = (view.x * 0.34).clamp(150.0, 210.0).round();
    let y0 = 50.0;
    // place pour le bouton Retour (decale de 10) et la marge du bas
    let fit = (((view.y - y0 - 14.0 - 32.0) / ROW).floor() as usize).max(3);
    let first = if n <= fit { 0 } else { sel.min(n - 1).saturating_sub(fit / 2).min(n - fit) };
    let shown = n.min(fit);
    let mut items: Vec<(Vec2, Vec2)> = (0..n)
        .map(|k| {
            let y = if k >= first && k < first + shown { y0 + (k - first) as f32 * ROW } else { -1000.0 };
            (Vec2::new(m, y), Vec2::new(lw, 22.0))
        })
        .collect();
    items.push((Vec2::new(m, y0 + shown as f32 * ROW + 10.0), Vec2::new(lw, 22.0)));
    let px = m + lw + 10.0;
    let panel = (Vec2::new(px, y0 - 4.0), Vec2::new(view.x - px - m, view.y - y0 - 14.0));
    SelectLayout { items, panel, title_y: 14.0, more: (first > 0, first + shown < n) }
}

/// Petits triangles au-dessus et au-dessous de la liste quand elle defile.
pub fn draw_scroll_marks(list: &mut DrawList, white: &assetkit::meta::Frame, l: &SelectLayout, n: usize, t: f32) {
    let Some(&(p0, s0)) = l.items.iter().take(n).find(|(p, _)| p.y >= 0.0) else { return };
    let last = l.items.iter().take(n).filter(|(p, _)| p.y >= 0.0).last().copied().unwrap_or((p0, s0));
    let bob = ((t * 4.0).sin() * 1.5).round();
    let cx = (p0.x + s0.x * 0.5).round();
    let c = with_alpha(GOLD, 0.9);
    if l.more.0 {
        for k in 0..4 {
            let w = 1.0 + 2.0 * k as f32;
            list.rect(white, Vec2::new(cx - (w - 1.0) * 0.5, p0.y - 7.0 + k as f32 - bob), Vec2::new(w, 1.0), c);
        }
    }
    if l.more.1 {
        let y = last.0.y + last.1.y + 2.0;
        for k in 0..4 {
            let w = 7.0 - 2.0 * k as f32;
            list.rect(white, Vec2::new(cx - (w - 1.0) * 0.5, y + k as f32 + bob), Vec2::new(w, 1.0), c);
        }
    }
}

pub fn input_select(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize) -> (Option<usize>, bool) {
    input_rects(nav, &select_layout(view, n, *sel).items, sel)
}

/// Petit cadenas (pixel art) : stage verrouille.
fn padlock(list: &mut DrawList, white: &assetkit::meta::Frame, p: Vec2, c: Color) {
    list.rect(white, p + Vec2::new(1.0, 0.0), Vec2::new(5.0, 1.0), c);
    list.rect(white, p + Vec2::new(1.0, 0.0), Vec2::new(1.0, 4.0), c);
    list.rect(white, p + Vec2::new(5.0, 0.0), Vec2::new(1.0, 4.0), c);
    list.rect(white, p + Vec2::new(0.0, 4.0), Vec2::new(7.0, 5.0), c);
    list.rect(white, p + Vec2::new(3.0, 6.0), Vec2::new(1.0, 2.0), [0.0, 0.0, 0.0, c[3]]);
}

/// Pendentif dore (losange) : stage gagne.
fn pendant(list: &mut DrawList, white: &assetkit::meta::Frame, p: Vec2, c: Color) {
    for (k, w) in [1.0, 3.0, 5.0, 7.0, 5.0, 3.0, 1.0].iter().enumerate() {
        list.rect(white, p + Vec2::new((7.0 - w) * 0.5, k as f32), Vec2::new(*w, 1.0), c);
    }
}

/// Apercu d'un stage : son sol et quelques elements du decor.
fn preview(list: &mut DrawList, atlas: &Atlas, map: &MapDef, pos: Vec2, cols: i32, rows: i32, seed: i32, locked: bool) {
    list.set(Space::Ui, Blend::Alpha);
    let tiles = atlas.tiles(&map.ground);
    let tint = [1.0f32; 4];
    let tint = if locked { [tint[0] * 0.3, tint[1] * 0.3, tint[2] * 0.35, 1.0] } else { tint };
    if !tiles.is_empty() {
        for ty in 0..rows {
            for tx in 0..cols {
                let f = &tiles[ground_index(tx + seed, ty, tiles.len())];
                list.frame(f, pos + Vec2::new(tx as f32 * 16.0, ty as f32 * 16.0), &Params::default().color(tint));
            }
        }
    }
    if locked {
        return;
    }
    // decor : un element par colonne paire, a hauteur variable
    let mut placed: Vec<(f32, &assetkit::meta::Frame)> = Vec::new();
    for (k, pd) in map.props.iter().enumerate() {
        let Some(id) = atlas.find(&pd.sprite, "0") else { continue };
        let f = atlas.frame(id, 0.0);
        let n = (cols as usize / 3).max(1);
        for j in 0..n {
            if (j + k) % map.props.len().max(1) != 0 {
                continue;
            }
            let x = (j as f32 + 0.5) / n as f32 * cols as f32 * 16.0;
            placed.push((x, f));
        }
    }
    let h = rows as f32 * 16.0;
    for (k, (x, f)) in placed.iter().enumerate() {
        let y = h * (0.55 + 0.25 * ((k * 7 % 5) as f32 / 4.0));
        let fh = f.h as f32;
        if y - fh < 0.0 {
            continue;
        }
        list.frame(f, (pos + Vec2::new(*x, y)).round(), &Params::default().color(tint));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_select(
    list: &mut DrawList,
    ctx: &Ctx,
    lang: &Lang,
    content: &Content,
    profile: &Profile,
    ids: &[String],
    sel: usize,
    t: f32,
) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 0.55]);
    let l = select_layout(view, ids.len(), sel);
    text(list, main, lang.get("ui.stages"), Vec2::new(view.x * 0.5, l.title_y), 2.0, WHITE, Align::Center);
    draw_scroll_marks(list, white, &l, ids.len(), t);
    // liste
    for (k, &(p, s)) in l.items.iter().enumerate() {
        if !l.visible(k) {
            continue;
        }
        let back = k == ids.len();
        let open = !back && unlocked(ids, profile, k);
        let won = !back && profile.won(&ids[k]);
        list.rect(white, p, s, with_alpha(BACK, if k == sel { 0.95 } else { 0.8 }));
        if k == sel {
            frame_rect(list, white, p, s, with_alpha(GOLD, 0.9));
        }
        list.probe_panel(p, s, "ligne de stage");
        let ty = p.y + 7.0;
        if back {
            text(list, main, lang.get("ui.back"), Vec2::new(p.x + 22.0, ty), 1.0, if k == sel { GOLD } else { SOFT }, Align::Left);
            continue;
        }
        let num = format!("{}", k + 1);
        text(list, main, &num, Vec2::new(p.x + 8.0, ty), 1.0, if open { GOLD } else { DIMMED }, Align::Left);
        let name = if open { stage_name(lang, &ids[k]) } else { lang.get("ui.locked").to_string() };
        let c = if !open {
            DIMMED
        } else if k == sel {
            GOLD
        } else {
            SOFT
        };
        let room = s.x - 22.0 - 16.0;
        let lines = wrap(main, &name, room, 1.0);
        let shown = if lines.len() > 1 { format!("{}…", lines[0]) } else { name };
        text(list, main, &shown, Vec2::new(p.x + 22.0, ty), 1.0, c, Align::Left);
        let icon = Vec2::new(p.x + s.x - 13.0, p.y + 7.0);
        if won {
            pendant(list, white, icon, WIN);
        } else if !open {
            padlock(list, white, icon, with_alpha(DIMMED, 1.0));
        }
    }
    // panneau du stage choisi
    let (pp, ps) = l.panel;
    list.rect(white, pp, ps, with_alpha(BACK, 0.92));
    frame_rect(list, white, pp + Vec2::splat(2.0), ps - Vec2::splat(4.0), with_alpha(GOLD, 0.7));
    list.probe_panel(pp, ps, "panneau du stage");
    let k = sel.min(ids.len().saturating_sub(1));
    let Some(id) = ids.get(k) else { return };
    let Some(map) = content.maps.get(id) else { return };
    let open = unlocked(ids, profile, k);
    let x = pp.x + 12.0;
    let w = ps.x - 24.0;
    let mut y = pp.y + 10.0;
    let name = if open { stage_name(lang, id) } else { lang.get("ui.locked").to_string() };
    let big = main.measure(&name) as f32 * 2.0 <= w;
    text(list, main, &name, Vec2::new(x, y), if big { 2.0 } else { 1.0 }, if open { GOLD } else { DIMMED }, Align::Left);
    y += if big { 30.0 } else { 16.0 };
    // apercu
    let cols = ((w / 16.0).floor() as i32).max(4);
    let rows = if ps.y > 260.0 { 5 } else { 4 };
    let pv = Vec2::new(x + ((w - cols as f32 * 16.0) * 0.5).round(), y);
    preview(list, atlas, map, pv, cols, rows, k as i32 * 37, !open);
    list.probe_block(pv, Vec2::new(cols as f32 * 16.0, rows as f32 * 16.0), "apercu");
    if !open {
        padlock(list, white, pv + Vec2::new(cols as f32 * 8.0 - 4.0, rows as f32 * 8.0 - 5.0), [0.9, 0.9, 1.0, 0.9]);
    }
    list.set(Space::Ui, Blend::Alpha);
    y += rows as f32 * 16.0 + 8.0;
    if !open {
        let prev = stage_name(lang, &ids[k - 1]);
        for line in wrap(main, &lang.get("ui.unlock_hint").replace("{s}", &prev), w, 1.0) {
            text(list, main, &line, Vec2::new(x, y), 1.0, SOFT, Align::Left);
            y += 13.0;
        }
        return;
    }
    for line in wrap(main, lang.get(&format!("map.{id}.desc")), w, 1.0) {
        text(list, main, &line, Vec2::new(x, y), 1.0, SOFT, Align::Left);
        y += 13.0;
    }
    y += 6.0;
    let boss_name = |b: &Option<sim::defs::SpawnAt>| b.as_ref().map(|b| lang.get(&format!("enemy.{}", b.enemy)).to_string());
    let rec = profile.stages.get(id.as_str());
    let best = rec.filter(|r| r.wins > 0).map(|r| fmt_time(r.best_time)).unwrap_or_else(|| lang.get("ui.not_won").to_string());
    let mut rows_info: Vec<(String, String)> = Vec::new();
    if let Some(b) = boss_name(&map.boss) {
        rows_info.push((lang.get("ui.miniboss").to_string(), b));
    }
    if let Some(b) = boss_name(&map.final_boss) {
        rows_info.push((lang.get("ui.final_boss").to_string(), b));
    }
    rows_info.push((lang.get("ui.best_time").to_string(), best));
    let bottom = pp.y + ps.y - 10.0;
    for (label, value) in rows_info {
        if y + 12.0 > bottom {
            break;
        }
        text(list, main, &label, Vec2::new(x, y), 1.0, DIMMED, Align::Left);
        text(list, main, &value, Vec2::new(x + w, y), 1.0, WHITE, Align::Right);
        y += 14.0;
    }
    let _ = t;
}

// ---------------------------------------------------------------------------------------------
// Victoire

/// Delai avant de pouvoir choisir.
pub const VICTORY_LOCK: f32 = 1.2;

/// Entrees de l'ecran de victoire (cles de texte).
pub fn victory_items(has_next: bool) -> Vec<&'static str> {
    let mut v = Vec::new();
    if has_next {
        v.push("ui.next_stage");
    }
    v.extend(["ui.stage_select", "ui.to_title"]);
    v
}

fn victory_y(view: Vec2, n: usize, k: usize) -> f32 {
    (view.y - 18.0 - n as f32 * 20.0).round() + k as f32 * 20.0
}

pub fn input_victory(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize) -> (Option<usize>, bool) {
    let rects: Vec<(Vec2, Vec2)> =
        (0..n).map(|k| (Vec2::new(((view.x - 240.0) * 0.5).round(), victory_y(view, n, k) - 3.0), Vec2::new(240.0, 18.0))).collect();
    input_rects(nav, &rects, sel)
}

/// Ecran de victoire : bilan de la partie, recompense et stage debloque.
#[allow(clippy::too_many_arguments)]
pub fn draw_victory(
    list: &mut DrawList,
    ctx: &Ctx,
    world: &World,
    lang: &Lang,
    t: f32,
    sel: usize,
    items: &[&str],
    banked: u64,
    score: Option<(u64, bool)>,
    notes: &[String],
) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    let a = (t / 0.6).min(1.0);
    list.rect(white, Vec2::ZERO, view, [0.04, 0.03, 0.0, 0.7 * a]);
    // rayons dores derriere le titre
    list.set(Space::Ui, Blend::Add);
    let c = Vec2::new(view.x * 0.5, (view.y * 0.16).round() + 12.0);
    for r in 0..12 {
        let ang = r as f32 / 12.0 * std::f32::consts::TAU + t * 0.25;
        for s in 0..8 {
            let p = c + Vec2::from_angle(ang) * (30.0 + s as f32 * 9.0);
            list.rect(white, p.round(), Vec2::splat(3.0), [1.0, 0.8, 0.3, 0.12 * a * (1.0 - s as f32 / 8.0)]);
        }
    }
    list.set(Space::Ui, Blend::Alpha);
    text(list, main, lang.get("ui.victory"), Vec2::new(view.x * 0.5, (view.y * 0.16).round()), 3.0, with_alpha(WIN, a), Align::Center);
    let name = stage_name(lang, &world.map_id);
    text(list, main, &name, Vec2::new(view.x * 0.5, (view.y * 0.16).round() + 40.0), 1.0, with_alpha(SOFT, a), Align::Center);
    let p = &world.player;
    let rows = [
        (lang.get("ui.time_win"), fmt_time(world.time)),
        (lang.get("ui.level_reached"), p.level.to_string()),
        (lang.get("ui.enemies_defeated"), p.kills.to_string()),
        (lang.get("ui.rupees_banked"), format!("+{banked}")),
        (lang.get("ui.score"), score.map(|s| crate::scores::fmt_score(s.0)).unwrap_or_default()),
    ];
    let rows = &rows[..if score.is_some() { 5 } else { 4 }];
    let w = 300.0f32.min(view.x - 40.0);
    let x0 = ((view.x - w) * 0.5).round();
    let ry = (view.y * 0.16).round() + 62.0;
    for (k, (l, v)) in rows.iter().enumerate() {
        let y = ry + k as f32 * 16.0;
        text(list, main, l, Vec2::new(x0, y), 1.0, with_alpha(SOFT, a), Align::Left);
        let vc = match k {
            3 => WIN,
            4 => GOLD,
            _ => WHITE,
        };
        text(list, main, v, Vec2::new(x0 + w, y), 1.0, with_alpha(vc, a), Align::Right);
    }
    let mut y = ry + rows.len() as f32 * 16.0 + 8.0;
    let blink = (t * 2.5).fract() < 0.75;
    if score.is_some_and(|s| s.1) {
        text(list, main, lang.get("ui.new_record"), Vec2::new(view.x * 0.5, y), 1.0, with_alpha(GOLD, if blink { a } else { a * 0.6 }), Align::Center);
        y += 14.0;
    }
    for s in notes {
        for line in wrap(main, s, view.x - 40.0, 1.0) {
            text(list, main, &line, Vec2::new(view.x * 0.5, y), 1.0, with_alpha(rgb(150, 240, 150), if blink { a } else { a * 0.7 }), Align::Center);
            y += 13.0;
        }
    }
    if t > VICTORY_LOCK {
        for (k, key) in items.iter().enumerate() {
            let y = victory_y(view, items.len(), k);
            let c = if k == sel { GOLD } else { SOFT };
            let tw = text(list, main, lang.get(key), Vec2::new(view.x * 0.5, y), 1.0, c, Align::Center);
            if k == sel {
                text(list, main, "▶", Vec2::new(((view.x - tw) * 0.5 - 14.0).round(), y), 1.0, GOLD, Align::Left);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use engine::Vec2;
    use sim::pickups::PickupKind;
    use sim::{Event, PlayerInput, World};
    use std::path::Path;

    /// Pikit voleur vaincu : tous les rubis au sol reviennent au joueur.
    #[test]
    fn pikit_brings_back_rupees() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = crate::content::load_content(&root);
        let mut w = World::new(content, "link", "hyrule_field", 11);
        w.player.god = true;
        assert!(w.thief_kind.is_some(), "Pikit absent des ennemis");
        let start = w.player.pos;
        for k in 0..6 {
            let at = start + Vec2::from_angle(k as f32) * 260.0;
            w.pickups.spawn(PickupKind::RupeeRed, 20, at, Vec2::ZERO, 0.0);
        }
        w.thief_t = 0.0;
        let input = PlayerInput::default();
        w.tick(&input);
        let pos = w.thief_pos().expect("Pikit apparu");
        let i = w.horde.pos.iter().position(|&p| p == pos).unwrap();
        let before = w.player.rupees;
        w.damage_enemy(i, 1.0e6, false, Vec2::X, 0.0, 0);
        let mut looted = None;
        for _ in 0..600 {
            w.tick(&input);
            for e in w.events.drain(..) {
                if let Event::ThiefLooted { count, .. } = e {
                    looted = Some(count);
                }
            }
        }
        assert!(looted.unwrap_or(0) >= 6, "rubis comptes : {looted:?}");
        assert!(w.player.rupees >= before + 120, "rubis ramasses : {} -> {}", before, w.player.rupees);
        assert!(w.thief.is_none());
    }

    /// Chaque stage se gagne : boss final, porte de sortie, victoire. Le decor laisse de la place.
    #[test]
    fn every_stage_can_be_won() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = crate::content::load_content(&root);
        let ids = super::stage_ids(&content);
        assert_eq!(ids.len(), 6, "6 stages attendus : {ids:?}");
        for id in &ids {
            let mut w = World::new(content.clone(), "link", id, 7);
            w.player.god = true;
            w.skip_to(w.map.final_boss.as_ref().unwrap().at - 1.0);
            let idle = PlayerInput::default();
            let mut boss = None;
            let mut portal = None;
            let mut won = false;
            for step in 0..60 * 160 {
                w.tick(&idle);
                for e in w.events.drain(..) {
                    match e {
                        Event::BossSpawned { last: true, .. } => boss = w.final_uid,
                        Event::PortalOpened { pos } => portal = Some(pos),
                        Event::Victory { .. } => won = true,
                        _ => {}
                    }
                }
                // le boss final joue 40 s (toutes ses attaques) puis tombe d'un coup
                if let (Some(uid), true) = (boss, step > 60 * 42) {
                    if let Some(i) = (0..w.horde.len()).find(|&i| w.horde.uid[i] == uid) {
                        if step % 30 == 0 && w.horde.hp[i] > 0.0 {
                            w.damage_enemy(i, 1.0e9, false, engine::Vec2::X, 0.0, 0);
                        }
                    }
                }
                if let Some(p) = portal {
                    w.player.pos = p;
                }
                if won {
                    break;
                }
            }
            assert!(boss.is_some(), "{id} : boss final absent");
            assert!(portal.is_some(), "{id} : porte jamais ouverte");
            assert!(won, "{id} : pas de victoire");
        }
    }
}
