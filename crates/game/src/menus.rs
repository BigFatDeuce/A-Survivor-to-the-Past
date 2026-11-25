//! Ecrans par-dessus le jeu : choix de niveau (1 parmi 3), coffre, pause, options, fin de partie.
//! Navigation au clavier, a la manette et a la souris.
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use crate::lang::Lang;
use crate::visuals::Vis;
use engine::app::Ctx;
use engine::assets::AnimId;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::input::{Bindings, KeyCode, MouseButton};
use engine::Vec2;
use sim::defs::{Mod, Stat, WeaponKind};
use sim::upgrades::{ChestReward, Offer};
use sim::World;

pub const DIM: Color = [0.02, 0.0, 0.06, 0.72];
const CARD_BG: Color = [0.09, 0.07, 0.18, 0.94];
const NEW_GREEN: Color = rgb(120, 240, 120);
const SOFT: Color = rgb(200, 200, 230);
/// Cartes qui fusionnent avec un objet deja possede.
const FUSION: Color = rgb(255, 120, 230);
const FUSION_LIGHT: Color = rgb(255, 190, 245);
const FUSION_BG: Color = [0.2, 0.06, 0.22, 0.95];

// ---------------------------------------------------------------------------------------------
// Navigation

/// Commandes de menu lues pour une image.
#[derive(Default, Clone, Copy)]
pub struct NavIn {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
    /// Souris deplacee : position en coordonnees d'interface.
    pub hover: Option<Vec2>,
    pub click: bool,
    pub mouse: Vec2,
    /// Touche 1..9 : choix direct.
    pub digit: Option<usize>,
}

/// Stick analogique transforme en appuis (avec repetition quand on le maintient).
pub struct Nav {
    held: [f32; 4],
    last_mouse: Vec2,
}

impl Nav {
    pub fn new() -> Nav {
        Nav { held: [-1.0; 4], last_mouse: Vec2::ZERO }
    }

    pub fn read(&mut self, ctx: &Ctx, b: &Bindings, dt: f32) -> NavIn {
        let i = &ctx.input;
        let s = i.stick(false);
        let dirs = [s.y < -0.6, s.y > 0.6, s.x < -0.6, s.x > 0.6];
        let mut stick = [false; 4];
        for k in 0..4 {
            if dirs[k] {
                if self.held[k] < 0.0 {
                    stick[k] = true;
                    self.held[k] = 0.0;
                } else {
                    self.held[k] += dt;
                    if self.held[k] > 0.4 {
                        stick[k] = true;
                        self.held[k] = 0.28;
                    }
                }
            } else {
                self.held[k] = -1.0;
            }
        }
        let mouse_btn = i.mouse_pressed(MouseButton::Left) || i.mouse_pressed(MouseButton::Right);
        let m = ctx.mouse_ui();
        let hover = ((m - self.last_mouse).length_squared() > 0.25).then_some(m);
        self.last_mouse = m;
        let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5];
        NavIn {
            up: i.action_pressed(b, "menu_up") || stick[0],
            down: i.action_pressed(b, "menu_down") || stick[1],
            left: i.action_pressed(b, "menu_left") || stick[2],
            right: i.action_pressed(b, "menu_right") || stick[3],
            // la souris valide par clic sur un element, pas n'importe ou
            confirm: i.action_pressed(b, "confirm") && !mouse_btn,
            back: i.action_pressed(b, "back") || i.action_pressed(b, "pause"),
            hover,
            click: i.mouse_pressed(MouseButton::Left),
            mouse: m,
            digit: digits.iter().position(|&k| i.key_pressed(k)),
        }
    }
}

fn inside(p: Vec2, pos: Vec2, size: Vec2) -> bool {
    p.x >= pos.x && p.y >= pos.y && p.x < pos.x + size.x && p.y < pos.y + size.y
}

fn dim(list: &mut DrawList, vis: &Vis, view: Vec2, c: Color) {
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(&vis.white, Vec2::ZERO, view, c);
}

// ---------------------------------------------------------------------------------------------
// Descriptions des propositions

pub struct OfferInfo {
    pub icon: AnimId,
    pub name: String,
    /// « Nouveau ! » ou « Nv 3 ».
    pub tag: String,
    pub is_new: bool,
    pub desc: String,
    /// Niveau apres le choix et niveau maximal (0 = sans niveaux).
    pub level: u32,
    pub max: u32,
    /// Partenaire d'evolution : (icone, nom, deja possede).
    pub evo: Vec<(AnimId, String, bool)>,
    /// Texte devant `evo` (vide : « Evolue avec »).
    pub evo_label: String,
}

fn signed(v: f32) -> String {
    if (v - v.round()).abs() < 0.001 {
        format!("{:+}", v.round() as i32)
    } else {
        format!("{v:+.1}")
    }
}

fn pct(v: f32) -> String {
    format!("{:+}%", (v * 100.0).round() as i32)
}

fn mod_text(lang: &Lang, m: &Mod, kind: WeaponKind) -> String {
    let (k, v) = match *m {
        Mod::Damage(d) => ("stat.damage", signed(d)),
        Mod::Cooldown(c) => ("stat.cooldown", pct(c)),
        Mod::Amount(n) => ("stat.amount", signed(n as f32)),
        Mod::Area(a) => ("stat.area", pct(a)),
        Mod::Speed(s) => ("stat.speed", pct(s)),
        Mod::Duration(d) => ("stat.duration", pct(d)),
        Mod::Pierce(n) => (if kind == WeaponKind::Swing { "stat.targets" } else { "stat.pierce" }, signed(n as f32)),
        Mod::Knockback(k) => ("stat.knockback", pct(k)),
        Mod::Crit(c) => ("stat.crit", pct(c)),
    };
    format!("{} {v}", lang.get(k))
}

pub fn stat_text(lang: &Lang, s: Stat, v: f32) -> String {
    let (k, val) = match s {
        Stat::Might => ("stat.might", pct(v)),
        Stat::Area => ("stat.area", pct(v)),
        Stat::Cooldown => ("stat.cooldown", pct(v)),
        Stat::Amount => ("stat.amount", signed(v)),
        Stat::Duration => ("stat.duration", pct(v)),
        Stat::Speed => ("stat.speed", pct(v)),
        Stat::MoveSpeed => ("stat.move_speed", pct(v)),
        Stat::MaxHp => ("stat.max_hp", signed(v / 10.0)),
        Stat::Armor => ("stat.armor", signed(v)),
        Stat::Magnet => ("stat.magnet", pct(v)),
        Stat::DashCooldown => ("stat.dash_cd", pct(v)),
        // vie par seconde -> coeurs par minute
        Stat::Regen => ("stat.regen", format!("{}/min", signed(v * 6.0))),
        Stat::Luck => ("stat.luck", pct(v)),
        Stat::Growth => ("stat.growth", pct(v)),
    };
    format!("{} {val}", lang.get(k))
}

pub fn offer_info(world: &World, vis: &Vis, lang: &Lang, o: Offer) -> OfferInfo {
    let c = &world.content;
    let new_tag = lang.get("ui.new").to_string();
    let lv_tag = |l: u32| format!("{} {l}", lang.get("ui.level"));
    match o {
        Offer::NewWeapon(id) | Offer::UpgradeWeapon(id) => {
            let def = &c.weapons[id as usize];
            let wid = &c.weapon_ids[id as usize];
            let cur = world.weapon_level(id);
            let level = cur.map_or(1, |l| l + 1);
            let is_new = cur.is_none();
            let desc = if is_new {
                lang.get(&format!("weapon.{wid}.desc")).to_string()
            } else {
                def.levels
                    .get(level as usize - 2)
                    .map(|ms| ms.iter().map(|m| mod_text(lang, m, def.kind)).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default()
            };
            let evo = def
                .evolution
                .as_ref()
                .and_then(|e| c.passive_index(&e.passive))
                .map(|pid| {
                    let owned = world.passive_level(pid).is_some();
                    let name = lang.get(&format!("passive.{}.name", c.passive_ids[pid as usize])).to_string();
                    vec![(vis.passives[pid as usize], name, owned)]
                })
                .unwrap_or_default();
            OfferInfo {
                icon: vis.weapons[id as usize].icon,
                name: lang.get(&format!("weapon.{wid}.name")).to_string(),
                tag: if is_new { new_tag } else { lv_tag(level) },
                is_new,
                desc,
                level,
                max: def.max_level(),
                evo,
                evo_label: String::new(),
            }
        }
        Offer::NewPassive(id) | Offer::UpgradePassive(id) => {
            let def = &c.passives[id as usize];
            let pid = &c.passive_ids[id as usize];
            let cur = world.passive_level(id);
            let level = cur.map_or(1, |l| l + 1);
            let is_new = cur.is_none();
            let mut desc = if is_new { lang.get(&format!("passive.{pid}.desc")).to_string() } else { String::new() };
            if let Some(stats) = def.levels.get(level as usize - 1) {
                let s = stats.iter().map(|&(st, v)| stat_text(lang, st, v)).collect::<Vec<_>>().join(", ");
                if is_new {
                    desc = format!("{desc} ({s})");
                } else {
                    desc = s;
                }
            }
            // armes qui evoluent grace a cet objet
            let evo = c
                .weapons
                .iter()
                .enumerate()
                .filter(|(_, w)| w.evolution.as_ref().is_some_and(|e| &e.passive == pid))
                .map(|(k, _)| {
                    let owned = world.weapon_level(k as u16).is_some();
                    let name = lang.get(&format!("weapon.{}.name", c.weapon_ids[k])).to_string();
                    (vis.weapons[k].icon, name, owned)
                })
                .collect();
            OfferInfo {
                icon: vis.passives[id as usize],
                name: lang.get(&format!("passive.{pid}.name")).to_string(),
                tag: if is_new { new_tag } else { lv_tag(level) },
                is_new,
                desc,
                level,
                max: def.max_level(),
                evo,
                evo_label: String::new(),
            }
        }
        Offer::Heal => OfferInfo {
            icon: vis.icon_heart,
            name: lang.get("ui.heal").to_string(),
            tag: String::new(),
            is_new: false,
            desc: lang.get("ui.heal_desc").to_string(),
            level: 0,
            max: 0,
            evo: Vec::new(),
            evo_label: String::new(),
        },
        Offer::Rupees => OfferInfo {
            icon: vis.icon_rupees,
            name: lang.get("ui.rupees").to_string(),
            tag: String::new(),
            is_new: false,
            desc: lang.get("ui.rupees_desc").to_string(),
            level: 0,
            max: 0,
            evo: Vec::new(),
            evo_label: String::new(),
        },
    }
}

// ---------------------------------------------------------------------------------------------
// Choix de niveau

/// Disposition des cartes : (position, taille) de chacune.
pub fn card_rects(view: Vec2, n: usize) -> Vec<(Vec2, Vec2)> {
    let w = (view.x - 40.0).min(400.0).round();
    let h = 64.0;
    let gap = 8.0;
    let total = n as f32 * h + (n.saturating_sub(1)) as f32 * gap;
    let y0 = (78.0f32).max(((view.y - total) * 0.5 + 18.0).round());
    (0..n).map(|k| (Vec2::new(((view.x - w) * 0.5).round(), y0 + k as f32 * (h + gap)), Vec2::new(w, h))).collect()
}

/// Delai avant d'accepter un choix (evite de valider par erreur en attaquant).
pub const LEVELUP_LOCK: f32 = 0.35;

#[allow(clippy::too_many_arguments)]
pub fn draw_levelup(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, offers: &[Offer], sel: usize, t: f32) {
    let infos: Vec<OfferInfo> = offers.iter().map(|&o| offer_info(world, vis, lang, o)).collect();
    draw_cards(list, ctx, vis, lang, (lang.get("ui.levelup"), lang.get("ui.choose"), GOLD), &infos, sel, t);
}

/// Cartes de choix (montee de niveau, recompense de rencontre) : titre, sous-titre et couleur du titre.
#[allow(clippy::too_many_arguments)]
pub fn draw_cards(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, head: (&str, &str, Color), infos: &[OfferInfo], sel: usize, t: f32) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let white = &vis.white;
    dim(list, vis, view, with_alpha(DIM, (t / 0.15).min(1.0)));
    let rects = card_rects(view, infos.len());
    let y0 = rects.first().map_or(94.0, |r| r.0.y);
    let pop = if t < 0.1 { 3.0 } else { 2.0 };
    text(list, main, head.0, Vec2::new(view.x * 0.5, y0 - 54.0 - (pop - 2.0) * 7.0), pop, head.2, Align::Center);
    text(list, main, head.1, Vec2::new(view.x * 0.5, y0 - 19.0), 1.0, SOFT, Align::Center);
    let locked = t < LEVELUP_LOCK;
    let rt = ctx.real_time as f32;
    for (k, ((pos, size), info)) in rects.into_iter().zip(infos).enumerate() {
        // fusion possible avec un objet (ou une arme) deja possede : la carte brille en rose
        let fusion = info.evo.iter().any(|e| e.2);
        // entree echelonnee par la droite
        let u = ((t - k as f32 * 0.05) / 0.18).clamp(0.0, 1.0);
        let e = 1.0 - (1.0 - u) * (1.0 - u);
        let selected = k == sel;
        let pos = pos + Vec2::new(((1.0 - e) * 60.0).round() + if selected { 4.0 } else { 0.0 }, 0.0);
        let a = e;
        list.rect(white, pos, size, with_alpha(if fusion { FUSION_BG } else { CARD_BG }, a));
        let pulse = 0.75 + 0.25 * (rt * 6.0).sin();
        let border = if selected {
            with_alpha(GOLD, a * if locked { 0.6 } else { pulse })
        } else if fusion {
            with_alpha(FUSION, a * (0.55 + 0.35 * (rt * 4.0).sin()))
        } else {
            [1.0, 1.0, 1.0, 0.3 * a]
        };
        frame_rect(list, white, pos, size, border);
        if selected {
            frame_rect(list, white, pos + Vec2::ONE, size - Vec2::splat(2.0), with_alpha(border, 0.5));
        }
        if fusion {
            // halo rose qui respire autour de la carte, et deux etincelles qui en font le tour
            let glow = 0.25 + 0.2 * (rt * 4.0).sin();
            list.set(Space::Ui, Blend::Add);
            frame_rect(list, white, pos - Vec2::splat(2.0), size + Vec2::splat(4.0), with_alpha(FUSION, glow * a));
            frame_rect(list, white, pos - Vec2::splat(3.0), size + Vec2::splat(6.0), with_alpha(FUSION, glow * 0.5 * a));
            let per = 2.0 * (size.x + size.y);
            for s in 0..2 {
                let d = ((rt * 140.0 + s as f32 * per * 0.5) % per + per) % per;
                let q = if d < size.x {
                    pos + Vec2::new(d, 0.0)
                } else if d < size.x + size.y {
                    pos + Vec2::new(size.x, d - size.x)
                } else if d < 2.0 * size.x + size.y {
                    pos + Vec2::new(size.x - (d - size.x - size.y), size.y)
                } else {
                    pos + Vec2::new(0.0, size.y - (d - 2.0 * size.x - size.y))
                };
                list.rect(white, (q - Vec2::splat(1.5)).round(), Vec2::splat(3.0), with_alpha(WHITE, 0.9 * a));
                list.rect(white, (q - Vec2::splat(3.0)).round(), Vec2::splat(6.0), with_alpha(FUSION, 0.35 * a));
            }
            list.set(Space::Ui, Blend::Alpha);
        }
        // icone x2 dans sa case
        let ib = Vec2::new(44.0, 44.0);
        let ip = pos + Vec2::new(10.0, (size.y - ib.y) * 0.5);
        list.rect(white, ip, ib, [0.0, 0.0, 0.0, 0.5 * a]);
        frame_rect(list, white, ip, ib, [1.0, 1.0, 1.0, 0.25 * a]);
        list.frame(atlas.frame(info.icon, 0.0), ip + ib * 0.5, &Params::default().scale(Vec2::splat(2.0)).color([1.0, 1.0, 1.0, a]));
        // touche directe
        text(list, atlas.font("hud"), &(k + 1).to_string(), pos + Vec2::new(4.0, 4.0), 1.0, [1.0, 1.0, 1.0, 0.4 * a], Align::Left);
        let tx = ip.x + ib.x + 10.0;
        let name_c = if selected { WHITE } else { SOFT };
        text(list, main, &info.name, Vec2::new(tx, pos.y + 6.0), 1.0, with_alpha(name_c, a), Align::Left);
        let mut tag_w = 0.0;
        if !info.tag.is_empty() {
            let c = if info.is_new { NEW_GREEN } else { GOLD };
            tag_w = text(list, main, &info.tag, Vec2::new(pos.x + size.x - 8.0, pos.y + 6.0), 1.0, with_alpha(c, a), Align::Right);
        }
        // pastilles de niveau, sur la ligne du nom, a gauche de l'etiquette
        if info.max > 1 {
            let right = pos.x + size.x - 8.0 - tag_w - 8.0;
            list.probe_block(Vec2::new(right - info.max as f32 * 6.0 + 2.0, pos.y + 10.0), Vec2::new(info.max as f32 * 6.0 - 2.0, 4.0), "niveaux");
            for l in 1..=info.max {
                let px = right - (info.max - l + 1) as f32 * 6.0 + 2.0;
                let c = if l < info.level {
                    [1.0, 1.0, 1.0, 0.8 * a]
                } else if l == info.level {
                    let blink = (ctx.real_time as f32 * 5.0).fract() < 0.6;
                    with_alpha(if blink { GOLD } else { WHITE }, a)
                } else {
                    [1.0, 1.0, 1.0, 0.18 * a]
                };
                list.rect(white, Vec2::new(px, pos.y + 10.0), Vec2::new(4.0, 4.0), c);
            }
        }
        let width = size.x - (tx - pos.x) - 10.0;
        let lines = wrap(main, &info.desc, width, 1.0);
        let evo_room = if info.evo.is_empty() { 0 } else { 1 };
        for (li, l) in lines.iter().take(2 + (1 - evo_room)).enumerate() {
            let y = pos.y + 21.0 + li as f32 * 12.0;
            text(list, main, l, Vec2::new(tx, y), 1.0, with_alpha(rgb(230, 230, 240), a), Align::Left);
        }
        // partenaire d'evolution : c'est la que les choix prennent du sens
        if !info.evo.is_empty() {
            let y = pos.y + size.y - 17.0;
            let lbl = if !info.evo_label.is_empty() {
                info.evo_label.as_str()
            } else if fusion {
                lang.get("ui.fusion_with")
            } else {
                lang.get("ui.evolves")
            };
            let lc = if fusion { FUSION } else { rgb(190, 170, 110) };
            let mut x = tx + text(list, main, lbl, Vec2::new(tx, y), 1.0, with_alpha(lc, a), Align::Left) + 10.0;
            for (icon, name, owned) in &info.evo {
                let c = if *owned { FUSION_LIGHT } else { [1.0, 1.0, 1.0, 0.55] };
                // l'objet deja possede sautille
                let hop = if *owned { (2.0 * (rt * 6.0).sin().abs()).round() } else { 0.0 };
                list.frame(atlas.frame(*icon, 0.0), Vec2::new(x, y + 6.0 - hop), &Params::default().color(with_alpha(if *owned { WHITE } else { c }, a)));
                x += 10.0;
                x += text(list, main, name, Vec2::new(x, y), 1.0, with_alpha(c, a), Align::Left) + 12.0;
            }
        }
    }
}

/// Lit le choix ; renvoie l'indice valide le cas echeant.
pub fn input_levelup(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize, t: f32) -> (Option<usize>, bool) {
    let mut moved = false;
    if nav.up && *sel > 0 {
        *sel -= 1;
        moved = true;
    }
    if nav.down && *sel + 1 < n {
        *sel += 1;
        moved = true;
    }
    let rects = card_rects(view, n);
    if let Some(m) = nav.hover {
        for (k, (p, s)) in rects.iter().enumerate() {
            if inside(m, *p, *s) && *sel != k {
                *sel = k;
                moved = true;
            }
        }
    }
    if t < LEVELUP_LOCK {
        return (None, moved);
    }
    if let Some(d) = nav.digit.filter(|&d| d < n) {
        return (Some(d), moved);
    }
    if nav.click {
        for (k, (p, s)) in rects.iter().enumerate() {
            if inside(nav.mouse, *p, *s) {
                return (Some(k), moved);
            }
        }
    }
    (nav.confirm.then_some(*sel), moved)
}

// ---------------------------------------------------------------------------------------------
// Coffre

/// Instant ou le coffre s'ouvre, puis ou l'on peut continuer.
pub const CHEST_OPEN: f32 = 0.7;
pub const CHEST_DONE: f32 = 1.5;

#[allow(clippy::too_many_arguments)]
pub fn draw_chest(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, reward: &ChestReward, big: bool, t: f32) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let white = &vis.white;
    dim(list, vis, view, with_alpha(DIM, (t / 0.2).min(1.0)));
    let center = Vec2::new((view.x * 0.5).round(), (view.y * 0.4).round());
    let open = t >= CHEST_OPEN;
    let b = big as usize;
    // rayons de lumiere une fois ouvert
    if open {
        let u = ((t - CHEST_OPEN) / 0.4).min(1.0);
        list.set(Space::Ui, Blend::Add);
        let rays = 12;
        for r in 0..rays {
            let ang = r as f32 / rays as f32 * std::f32::consts::TAU + ctx.real_time as f32 * 0.6;
            for s in 0..14 {
                let d = 10.0 + s as f32 * 7.0 * u;
                let p = center - Vec2::new(0.0, 12.0) + Vec2::from_angle(ang) * d;
                let al = (1.0 - s as f32 / 14.0) * 0.35 * u;
                list.rect(white, p.round(), Vec2::splat(3.0), [1.0, 0.9, 0.5, al]);
            }
        }
        list.set(Space::Ui, Blend::Alpha);
    }
    // le coffre tremble de plus en plus avant de s'ouvrir
    let shake = if open { Vec2::ZERO } else { Vec2::new(((t * 60.0).sin() * (t / CHEST_OPEN) * 2.0).round(), 0.0) };
    let anim = if open { vis.chest_open[b] } else { vis.chest[b] };
    let sc = if big { 2.0 } else { 3.0 };
    list.frame(atlas.frame(anim, 0.0), center + Vec2::new(0.0, 20.0) + shake, &Params::default().scale(Vec2::splat(sc)));

    let title = if reward.evolution.is_some() { lang.get("ui.evolution") } else { lang.get("ui.chest") };
    let tc = if reward.evolution.is_some() { rgb(255, 160, 255) } else { GOLD };
    text(list, main, title, Vec2::new(view.x * 0.5, 46.0), 2.0, tc, Align::Center);
    if !open {
        return;
    }
    let u = ((t - CHEST_OPEN) / 0.35).clamp(0.0, 1.0);
    let rise = (1.0 - u) * 16.0;
    let mut y = center.y + 46.0;
    if let Some((from, into)) = reward.evolution {
        // l'arme d'origine se transforme : icones et nom
        let c = &world.content;
        let fi = vis.weapons[from as usize].icon;
        let ii = vis.weapons[into as usize].icon;
        let iy = center.y - 40.0 - rise;
        let flash = if u < 1.0 { 1.0 - u } else { 0.0 };
        list.frame(atlas.frame(fi, 0.0), Vec2::new(center.x - 44.0, iy), &Params::default().scale(Vec2::splat(2.0)).color([1.0, 1.0, 1.0, 0.6]));
        text(list, main, "▶", Vec2::new(center.x, iy - 6.0), 1.0, WHITE, Align::Center);
        list.frame(
            atlas.frame(ii, 0.0),
            Vec2::new(center.x + 44.0, iy),
            &Params::default().scale(Vec2::splat(3.0)).flash([1.0, 1.0, 1.0], flash),
        );
        let name = lang.get(&format!("weapon.{}.name", c.weapon_ids[into as usize]));
        text(list, main, name, Vec2::new(view.x * 0.5, y), 2.0, with_alpha(GOLD, u), Align::Center);
        y += 30.0;
        let desc = lang.get(&format!("weapon.{}.desc", c.weapon_ids[into as usize]));
        for l in wrap(main, desc, view.x - 80.0, 1.0) {
            text(list, main, &l, Vec2::new(view.x * 0.5, y), 1.0, with_alpha(SOFT, u), Align::Center);
            y += 13.0;
        }
        y += 6.0;
    }
    // ameliorations obtenues
    for (k, &o) in reward.upgrades.iter().enumerate() {
        let uu = ((t - CHEST_OPEN - 0.15 - k as f32 * 0.12) / 0.25).clamp(0.0, 1.0);
        if uu <= 0.0 {
            continue;
        }
        // niveau atteint par CETTE amelioration (le meme objet peut revenir plusieurs fois)
        let later = reward.upgrades[k + 1..].iter().filter(|&&x| x == o).count() as u32;
        let (icon, name, lv) = match o {
            Offer::UpgradeWeapon(id) | Offer::NewWeapon(id) => (
                vis.weapons[id as usize].icon,
                lang.get(&format!("weapon.{}.name", world.content.weapon_ids[id as usize])).to_string(),
                world.weapon_level(id).unwrap_or(1).saturating_sub(later),
            ),
            Offer::UpgradePassive(id) | Offer::NewPassive(id) => (
                vis.passives[id as usize],
                lang.get(&format!("passive.{}.name", world.content.passive_ids[id as usize])).to_string(),
                world.passive_level(id).unwrap_or(1).saturating_sub(later),
            ),
            Offer::Heal => (vis.icon_heart, lang.get("ui.heal").to_string(), 0),
            Offer::Rupees => (vis.icon_rupees, lang.get("ui.rupees").to_string(), 0),
        };
        let line = if lv > 0 { format!("{name}  {} {lv}", lang.get("ui.level")) } else { name };
        let w = main.measure(&line) as f32 + 22.0;
        let x = ((view.x - w) * 0.5).round();
        let yy = y + (1.0 - uu) * 8.0;
        list.rect(white, Vec2::new(x - 6.0, yy - 4.0), Vec2::new(w + 12.0, 20.0), with_alpha(BACK, uu * 0.8));
        list.frame(atlas.frame(icon, 0.0), Vec2::new(x + 6.0, yy + 6.0), &Params::default().color([1.0, 1.0, 1.0, uu]));
        text(list, main, &line, Vec2::new(x + 20.0, yy), 1.0, with_alpha(WHITE, uu), Align::Left);
        y += 24.0;
    }
    if t >= CHEST_DONE && (ctx.real_time * 2.0).fract() < 0.7 {
        text(list, main, lang.get("ui.continue"), Vec2::new(view.x * 0.5, view.y - 26.0), 1.0, SOFT, Align::Center);
    }
}

// ---------------------------------------------------------------------------------------------
// Pause et options

pub const PAUSE_ITEMS: [&str; 4] = ["ui.resume", "ui.options", "ui.restart", "ui.to_title"];

fn item_rect(view: Vec2, y: f32, w: f32) -> (Vec2, Vec2) {
    (Vec2::new(((view.x - w) * 0.5).round(), y - 3.0), Vec2::new(w, 18.0))
}

fn pause_y(view: Vec2, k: usize) -> f32 {
    (view.y * 0.36).round() + k as f32 * 22.0
}

pub fn draw_pause(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, sel: usize) {
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    dim(list, vis, view, DIM);
    text(list, main, lang.get("ui.paused"), Vec2::new(view.x * 0.5, (view.y * 0.12).round()), 3.0, WHITE, Align::Center);
    for (k, key) in PAUSE_ITEMS.iter().enumerate() {
        let y = pause_y(view, k);
        let c = if k == sel { GOLD } else { SOFT };
        let w = text(list, main, lang.get(key), Vec2::new(view.x * 0.5, y), 1.0, c, Align::Center);
        if k == sel {
            text(list, main, "▶", Vec2::new(((view.x - w) * 0.5 - 14.0).round(), y), 1.0, GOLD, Align::Left);
        }
    }
    let help = ["ui.help.move", "ui.help.attack", "ui.help.dash", "ui.help.auto", "ui.help.keys"];
    let y0 = pause_y(view, PAUSE_ITEMS.len()) + 16.0;
    for (k, h) in help.iter().enumerate() {
        text(list, main, lang.get(h), Vec2::new(view.x * 0.5, y0 + k as f32 * 13.0), 1.0, [0.75, 0.75, 0.85, 0.8], Align::Center);
    }
}

pub fn input_menu(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize, y_of: impl Fn(usize) -> f32, width: f32) -> (Option<usize>, bool) {
    let rects: Vec<(Vec2, Vec2)> = (0..n).map(|k| item_rect(view, y_of(k), width)).collect();
    input_rects(nav, &rects, sel)
}

/// Liste verticale d'elements cliquables : fleches, survol et clic de souris, validation.
pub fn input_rects(nav: &NavIn, rects: &[(Vec2, Vec2)], sel: &mut usize) -> (Option<usize>, bool) {
    let n = rects.len();
    let mut moved = false;
    if nav.up {
        *sel = (*sel + n - 1) % n;
        moved = true;
    }
    if nav.down {
        *sel = (*sel + 1) % n;
        moved = true;
    }
    if let Some(m) = nav.hover {
        for (k, &(p, s)) in rects.iter().enumerate() {
            if inside(m, p, s) && *sel != k {
                *sel = k;
                moved = true;
            }
        }
    }
    if nav.click {
        for (k, &(p, s)) in rects.iter().enumerate() {
            if inside(nav.mouse, p, s) {
                return (Some(k), moved);
            }
        }
    }
    (nav.confirm.then_some(*sel), moved)
}

pub fn input_pause(nav: &NavIn, view: Vec2, sel: &mut usize) -> (Option<usize>, bool) {
    input_menu(nav, view, PAUSE_ITEMS.len(), sel, |k| pause_y(view, k), 220.0)
}

/// Lignes du menu Options.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opt {
    Language,
    Video,
    Aspect,
    Filter,
    Master,
    Music,
    Sfx,
    Fullscreen,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Updates,
    /// Ecran de reattribution des touches.
    Controls,
    Back,
}

#[cfg(not(target_arch = "wasm32"))]
pub const OPTIONS: [Opt; 11] = [
    Opt::Language,
    Opt::Video,
    Opt::Aspect,
    Opt::Filter,
    Opt::Master,
    Opt::Music,
    Opt::Sfx,
    Opt::Fullscreen,
    Opt::Updates,
    Opt::Controls,
    Opt::Back,
];

/// Navigateur : pas de mises a jour (le site sert la derniere version).
#[cfg(target_arch = "wasm32")]
pub const OPTIONS: [Opt; 10] = [
    Opt::Language,
    Opt::Video,
    Opt::Aspect,
    Opt::Filter,
    Opt::Master,
    Opt::Music,
    Opt::Sfx,
    Opt::Fullscreen,
    Opt::Controls,
    Opt::Back,
];

pub fn options_y(view: Vec2, k: usize) -> f32 {
    (view.y * 0.24).round() + k as f32 * 22.0
}

const OPT_W: f32 = 360.0;

pub struct OptValues<'a> {
    pub lang_name: &'a str,
    pub video: engine::gfx::VideoFilter,
    pub aspect: engine::gfx::Aspect,
    pub filter: engine::audio::AudioFilter,
    pub master: f32,
    pub music: f32,
    pub sfx: f32,
    pub fullscreen: bool,
    pub updates: crate::update::UpdateMode,
}

pub fn draw_options(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, sel: usize, v: &OptValues) {
    use engine::audio::AudioFilter;
    use engine::gfx::{Aspect, VideoFilter};
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    let white = &vis.white;
    dim(list, vis, view, [0.02, 0.0, 0.06, 0.85]);
    text(list, main, lang.get("ui.options"), Vec2::new(view.x * 0.5, (options_y(view, 0) - 44.0).max(8.0)), 2.0, WHITE, Align::Center);
    let w = OPT_W.min(view.x - 24.0);
    let x0 = ((view.x - w) * 0.5).round();
    for (k, o) in OPTIONS.iter().enumerate() {
        let y = options_y(view, k);
        let selected = k == sel;
        let c = if selected { GOLD } else { SOFT };
        if selected {
            list.rect(white, Vec2::new(x0 - 4.0, y - 4.0), Vec2::new(w + 8.0, 20.0), [1.0, 1.0, 1.0, 0.07]);
            text(list, main, "▶", Vec2::new(x0 - 16.0, y), 1.0, GOLD, Align::Left);
        }
        let label = match o {
            Opt::Language => "ui.language",
            Opt::Video => "ui.video_filter",
            Opt::Aspect => "ui.screen_format",
            Opt::Filter => "ui.audio_filter",
            Opt::Master => "ui.master",
            Opt::Music => "ui.music",
            Opt::Sfx => "ui.sfx",
            Opt::Fullscreen => "ui.fullscreen",
            Opt::Updates => "ui.updates",
            Opt::Controls => "ui.controls",
            Opt::Back => "ui.back",
        };
        if *o == Opt::Back {
            text(list, main, lang.get(label), Vec2::new(view.x * 0.5, y), 1.0, c, Align::Center);
            continue;
        }
        text(list, main, lang.get(label), Vec2::new(x0, y), 1.0, c, Align::Left);
        let right = x0 + w;
        match o {
            Opt::Language => {
                text(list, main, v.lang_name, Vec2::new(right, y), 1.0, WHITE, Align::Right);
            }
            Opt::Video => {
                let s = lang.get(match v.video {
                    VideoFilter::None => "ui.video.none",
                    VideoFilter::Snes => "ui.video.snes",
                    VideoFilter::Crt => "ui.video.crt",
                    VideoFilter::Scanlines => "ui.video.scanlines",
                    VideoFilter::Gba => "ui.video.gba",
                    VideoFilter::GameBoy => "ui.video.gb",
                });
                text(list, main, s, Vec2::new(right, y), 1.0, WHITE, Align::Right);
            }
            Opt::Aspect => {
                let s = lang.get(match v.aspect {
                    Aspect::Wide => "ui.aspect.wide",
                    Aspect::Classic => "ui.aspect.classic",
                });
                text(list, main, s, Vec2::new(right, y), 1.0, WHITE, Align::Right);
            }
            Opt::Filter => {
                let s = lang.get(match v.filter {
                    AudioFilter::Off => "ui.filter.off",
                    AudioFilter::Soft => "ui.filter.soft",
                    AudioFilter::Retro => "ui.filter.retro",
                    AudioFilter::Gba => "ui.filter.gba",
                    AudioFilter::GameBoy => "ui.filter.gb",
                });
                text(list, main, s, Vec2::new(right, y), 1.0, WHITE, Align::Right);
            }
            Opt::Master | Opt::Music | Opt::Sfx => {
                let val = match o {
                    Opt::Master => v.master,
                    Opt::Music => v.music,
                    _ => v.sfx,
                };
                let (bp, bs) = volume_bar(right, y);
                for s in 0..10 {
                    let on = val >= (s as f32 + 0.5) / 10.0;
                    let c = if on { if selected { GOLD } else { WHITE } } else { [1.0, 1.0, 1.0, 0.15] };
                    list.rect(white, bp + Vec2::new(s as f32 * (bs.x / 10.0), 0.0), Vec2::new(bs.x / 10.0 - 2.0, bs.y), c);
                }
            }
            Opt::Fullscreen => {
                let s = if v.fullscreen { lang.get("ui.on") } else { lang.get("ui.off") };
                text(list, main, s, Vec2::new(right, y), 1.0, WHITE, Align::Right);
            }
            Opt::Updates => {
                use crate::update::UpdateMode;
                let s = lang.get(match v.updates {
                    UpdateMode::Auto => "ui.update.auto",
                    UpdateMode::Ask => "ui.update.ask",
                    UpdateMode::Off => "ui.off",
                });
                text(list, main, s, Vec2::new(right, y), 1.0, WHITE, Align::Right);
            }
            Opt::Controls => {
                text(list, main, "▶", Vec2::new(right, y), 1.0, if selected { GOLD } else { SOFT }, Align::Right);
            }
            Opt::Back => {}
        }
    }
}

fn volume_bar(right: f32, y: f32) -> (Vec2, Vec2) {
    (Vec2::new(right - 100.0, y + 1.0), Vec2::new(100.0, 9.0))
}

/// Resultat d'une action dans les options.
pub enum OptAction {
    None,
    /// Changer une valeur : -1 / +1.
    Step(Opt, i32),
    /// Volume regle a la souris (0..1).
    Set(Opt, f32),
    Back,
}

pub fn input_options(nav: &NavIn, view: Vec2, sel: &mut usize) -> (OptAction, bool) {
    let n = OPTIONS.len();
    let w = OPT_W.min(view.x - 24.0);
    let (r, moved) = input_menu(nav, view, n, sel, |k| options_y(view, k), w + 8.0);
    let o = OPTIONS[*sel];
    if nav.back {
        return (OptAction::Back, moved);
    }
    if nav.left {
        return (OptAction::Step(o, -1), moved);
    }
    if nav.right {
        return (OptAction::Step(o, 1), moved);
    }
    if let Some(k) = r {
        let o = OPTIONS[k];
        if o == Opt::Back {
            return (OptAction::Back, moved);
        }
        if nav.click && matches!(o, Opt::Master | Opt::Music | Opt::Sfx) {
            let x0 = ((view.x - w) * 0.5).round();
            let (bp, bs) = volume_bar(x0 + w, options_y(view, k));
            let f = ((nav.mouse.x - bp.x) / bs.x).clamp(0.0, 1.0);
            return (OptAction::Set(o, (f * 10.0).ceil() / 10.0), moved);
        }
        return (OptAction::Step(o, 1), moved);
    }
    (OptAction::None, moved)
}

// ---------------------------------------------------------------------------------------------
// Fin de partie

/// `score` : score de la partie et record du stage.
#[allow(clippy::too_many_arguments)]
pub fn draw_dead(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, t: f32, sel: usize, score: Option<(u64, bool)>) {
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    dim(list, vis, view, [0.1, 0.0, 0.02, 0.72]);
    text(list, main, lang.get("ui.gameover"), Vec2::new(view.x * 0.5, (view.y * 0.16).round()), 3.0, rgb(255, 90, 90), Align::Center);
    let p = &world.player;
    let secs = world.time as u32;
    let rows = [
        (lang.get("ui.time_survived"), format!("{:02}:{:02}", secs / 60, secs % 60)),
        (lang.get("ui.level_reached"), p.level.to_string()),
        (lang.get("ui.enemies_defeated"), p.kills.to_string()),
        (lang.get("ui.rupees_collected"), p.rupees.to_string()),
    ];
    let w = 300.0f32.min(view.x - 40.0);
    let x0 = ((view.x - w) * 0.5).round();
    let y0 = (view.y * 0.38).round();
    for (k, (l, v)) in rows.iter().enumerate() {
        let y = y0 + k as f32 * 18.0;
        text(list, main, l, Vec2::new(x0, y), 1.0, SOFT, Align::Left);
        text(list, main, v, Vec2::new(x0 + w, y), 1.0, WHITE, Align::Right);
    }
    let mut ny = y0 + rows.len() as f32 * 18.0;
    if let Some((s, record)) = score {
        text(list, main, lang.get("ui.score"), Vec2::new(x0, ny), 1.0, SOFT, Align::Left);
        text(list, main, &crate::scores::fmt_score(s), Vec2::new(x0 + w, ny), 1.0, GOLD, Align::Right);
        if record {
            let blink = (ctx.real_time as f32 * 2.5).fract() < 0.75;
            let label = lang.get("ui.new_record");
            text(list, main, label, Vec2::new(view.x * 0.5, ny + 16.0), 1.0, with_alpha(GOLD, if blink { 1.0 } else { 0.6 }), Align::Center);
            ny += 16.0;
        }
        ny += 18.0;
    }
    // armes et objets de la partie
    let atlas = &ctx.atlas;
    let mut x = (view.x * 0.5 - (world.weapons.len() + world.passives.len()) as f32 * 11.0).round() + 11.0;
    let y = ny + 14.0;
    for w in &world.weapons {
        list.frame(atlas.frame(vis.weapons[w.id as usize].icon, 0.0), Vec2::new(x, y), &Params::default());
        x += 22.0;
    }
    for p in &world.passives {
        list.frame(atlas.frame(vis.passives[p.id as usize], 0.0), Vec2::new(x, y), &Params::default());
        x += 22.0;
    }
    if t > DEAD_LOCK {
        for (k, key) in DEAD_ITEMS.iter().enumerate() {
            let y = dead_y(view, k);
            let c = if k == sel { GOLD } else { SOFT };
            let w = text(list, main, lang.get(key), Vec2::new(view.x * 0.5, y), 1.0, c, Align::Center);
            if k == sel {
                text(list, main, "▶", Vec2::new(((view.x - w) * 0.5 - 14.0).round(), y), 1.0, GOLD, Align::Left);
            }
        }
    }
}

/// Delai avant de pouvoir choisir (evite de relancer par reflexe).
pub const DEAD_LOCK: f32 = 0.8;
pub const DEAD_ITEMS: [&str; 2] = ["ui.retry", "ui.to_title"];

fn dead_y(view: Vec2, k: usize) -> f32 {
    (view.y - 52.0).round() + k as f32 * 20.0
}

pub fn input_dead(nav: &NavIn, view: Vec2, sel: &mut usize) -> (Option<usize>, bool) {
    input_menu(nav, view, DEAD_ITEMS.len(), sel, |k| dead_y(view, k), 220.0)
}
