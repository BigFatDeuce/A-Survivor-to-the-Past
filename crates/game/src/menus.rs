//! Ecrans par-dessus le jeu : choix de niveau (1 parmi 3), coffre, pause, options, fin de partie.
//! Navigation au clavier, a la manette et a la souris.
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use assetkit::meta::Font;
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
/// Cartes d'objets gardes apres la partie (reliques) : or de la Triforce.
const KEEP: Color = rgb(255, 215, 80);
const KEEP_BG: Color = [0.24, 0.17, 0.03, 0.96];

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
    /// Onglet precedent / suivant (gachettes, Q / E).
    pub tab_prev: bool,
    pub tab_next: bool,
    /// Couleur suivante du personnage (R, Y).
    pub skin: bool,
    /// Molette : lignes a faire defiler (positif = vers le bas de la liste).
    pub wheel: i32,
}

/// Stick analogique transforme en appuis (avec repetition quand on le maintient).
pub struct Nav {
    held: [f32; 4],
    /// Temps depuis le dernier pas par direction : deux sources pour un meme appui (manette lue par le jeu
    /// et touches envoyees par gptokeyb sur les consoles PortMaster) ne comptent qu'une fois.
    since: [f32; 4],
    last_mouse: Vec2,
    /// Reste de molette (pave tactile : fractions de cran).
    wheel_acc: f32,
}

impl Nav {
    pub fn new() -> Nav {
        Nav { held: [-1.0; 4], since: [1.0; 4], last_mouse: Vec2::ZERO, wheel_acc: 0.0 }
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
        self.wheel_acc -= i.wheel();
        let wheel = self.wheel_acc.trunc();
        self.wheel_acc -= wheel;
        let mut dir = [
            i.action_pressed(b, "menu_up") || stick[0],
            i.action_pressed(b, "menu_down") || stick[1],
            i.action_pressed(b, "menu_left") || stick[2],
            i.action_pressed(b, "menu_right") || stick[3],
        ];
        for (d, t) in dir.iter_mut().zip(&mut self.since) {
            *t += dt;
            *d &= *t >= 0.09;
            if *d {
                *t = 0.0;
            }
        }
        let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5];
        NavIn {
            up: dir[0],
            down: dir[1],
            left: dir[2],
            right: dir[3],
            // la souris valide par clic sur un element, pas n'importe ou
            confirm: i.action_pressed(b, "confirm") && !mouse_btn,
            back: i.action_pressed(b, "back") || i.action_pressed(b, "pause"),
            hover,
            click: i.mouse_pressed(MouseButton::Left),
            mouse: m,
            digit: digits.iter().position(|&k| i.key_pressed(k)),
            tab_prev: i.action_pressed(b, "tab_prev"),
            tab_next: i.action_pressed(b, "tab_next"),
            skin: i.action_pressed(b, "skin"),
            wheel: wheel as i32,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Defilement des listes longues : la liste ne bouge que pour garder la selection visible (clavier,
// manette) ou a la molette ; survoler une ligne la choisit sans faire defiler (sinon la ligne suivante
// arrive sous la souris et la liste part toute seule).

/// Listes qui defilent (une seule de chaque sorte a l'ecran a la fois).
#[derive(Clone, Copy)]
pub enum List {
    /// Choix du personnage et du stage.
    Select = 0,
    /// Ecrans du hub (boutique, reliques, bestiaire, parties, classements...).
    Hub = 1,
}

thread_local! {
    static SCROLL: [std::cell::Cell<usize>; 2] = const { [std::cell::Cell::new(0), std::cell::Cell::new(0)] };
}

/// Premiere ligne visible : celle retenue, decalee juste assez pour montrer `sel` (sans recentrer).
/// `sel >= n` (bouton Retour, onglets) ne deplace pas la liste.
pub fn first_row(list: List, n: usize, fit: usize, sel: usize) -> usize {
    if n <= fit {
        return 0;
    }
    let mut f = SCROLL.with(|s| s[list as usize].get()).min(n - fit);
    if sel < n {
        if sel < f {
            f = sel;
        } else if sel >= f + fit {
            f = sel + 1 - fit;
        }
    }
    f
}

/// Retient la premiere ligne visible apres les entrees de l'image.
pub fn keep_scroll(list: List, n: usize, fit: usize, sel: usize) {
    let f = first_row(list, n, fit, sel);
    SCROLL.with(|s| s[list as usize].set(f));
}

/// Molette : fait defiler la liste et garde la selection dans la partie visible.
/// Renvoie vrai si la selection a change.
pub fn wheel_scroll(list: List, wheel: i32, n: usize, fit: usize, sel: &mut usize) -> bool {
    if wheel == 0 || n <= fit {
        return false;
    }
    let f0 = first_row(list, n, fit, *sel);
    let f = (f0 as i64 + wheel as i64).clamp(0, (n - fit) as i64) as usize;
    SCROLL.with(|s| s[list as usize].set(f));
    if *sel < n {
        let s = (*sel).clamp(f, f + fit - 1);
        if s != *sel {
            *sel = s;
            return true;
        }
    }
    false
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
    /// Partenaire d'evolution : (icone, nom, deja possede). Le nom d'un partenaire possede porte
    /// son niveau (« Abeilles 3/8 ») : la fusion demande les deux au niveau maximal.
    /// Partenaires de fusion : icone, nom, possede, inconnu (arme pas encore debloquee : « ??? »).
    pub evo: Vec<(AnimId, String, bool, bool)>,
    /// Texte devant `evo` (vide : « Evolue avec »).
    pub evo_label: String,
    /// Objet garde apres la partie (relique) : carte doree, bandeau et etincelles.
    pub keep: bool,
    /// Fusion proposee (1) ou fusion ultime (2) : la carte la plus mise en valeur de l'ecran.
    pub fusion: u8,
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
            let evo = world
                .recipes_of(id)
                .into_iter()
                .filter_map(|e| c.passive_index(&e.passive))
                // partenaire deja pris par une autre fusion : cette arme n'evoluera plus avec lui
                .filter(|&pid| world.partner_free(pid))
                .map(|pid| {
                    let lv = world.passive_level(pid);
                    let mut name = lang.get(&format!("passive.{}.name", c.passive_ids[pid as usize])).to_string();
                    if let Some(l) = lv {
                        name = format!("{name} {l}/{}", c.passives[pid as usize].max_level());
                    }
                    (vis.passives[pid as usize], name, lv.is_some(), false)
                })
                .collect();
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
                keep: false,
                fusion: 0,
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
                .filter(|(k, _)| world.partner_free(id) && world.recipes_of(*k as u16).iter().any(|e| &e.passive == pid))
                .map(|(k, w)| {
                    // arme pas encore debloquee (boss intermediaire) : le joueur ne peut pas l'avoir
                    if world.locked_weapons.contains(&(k as u16)) {
                        return (vis.weapons[k].icon, "???".to_string(), false, true);
                    }
                    let lv = world.weapon_level(k as u16);
                    let mut name = lang.get(&format!("weapon.{}.name", c.weapon_ids[k])).to_string();
                    if let Some(l) = lv {
                        name = format!("{name} {l}/{}", w.max_level());
                    }
                    (vis.weapons[k].icon, name, lv.is_some(), false)
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
                keep: false,
                fusion: 0,
            }
        }
        Offer::Heal => OfferInfo {
            icon: vis.specials[1], // fee : ne pas confondre avec le Receptacle de coeur
            name: lang.get("ui.heal").to_string(),
            tag: String::new(),
            is_new: false,
            desc: lang.get("ui.heal_desc").to_string(),
            level: 0,
            max: 0,
            evo: Vec::new(),
            evo_label: String::new(),
            keep: false,
            fusion: 0,
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
            keep: false,
            fusion: 0,
        },
        Offer::Evolve(into) => {
            let id = &c.weapon_ids[into as usize];
            // recette : l'arme et son partenaire, tous deux en main
            let (from, recipe) = c
                .weapons
                .iter()
                .enumerate()
                .find_map(|(k, w)| w.recipes().find(|e| &e.into == id).map(|e| (k, e)))
                .map_or((None, None), |(k, e)| (Some(k), Some(e)));
            let mut evo = Vec::new();
            if let Some(k) = from {
                evo.push((vis.weapons[k].icon, lang.get(&format!("weapon.{}.name", c.weapon_ids[k])).to_string(), true, false));
            }
            if let Some(pid) = recipe.and_then(|e| c.passive_index(&e.passive)) {
                evo.push((vis.passives[pid as usize], lang.get(&format!("passive.{}.name", c.passive_ids[pid as usize])).to_string(), true, false));
            }
            let ultimate = from.is_some_and(|k| c.weapons[k].ultimate.as_ref().is_some_and(|u| &u.into == id));
            OfferInfo {
                icon: vis.weapons[into as usize].icon,
                name: crate::hud::hl_fusion(lang.get(&format!("weapon.{id}.name"))),
                tag: lang.get("ui.evolution").to_string(),
                is_new: true,
                desc: lang.get(&format!("weapon.{id}.desc")).to_string(),
                level: 0,
                max: 0,
                evo,
                evo_label: lang.get("ui.fusion_of").to_string(),
                keep: false,
                fusion: if ultimate { 2 } else { 1 },
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Choix de niveau

/// Disposition des cartes : (position, taille) de chacune.
pub fn card_rects(view: Vec2, n: usize) -> Vec<(Vec2, Vec2)> {
    card_rects_ex(view, n, 0.0)
}

/// Cartes de choix avec `extra` pixels reserves dessous (boutons Relancer / Passer).
pub fn card_rects_ex(view: Vec2, n: usize, extra: f32) -> Vec<(Vec2, Vec2)> {
    let w = (view.x - 40.0).min(400.0).round();
    let h = 64.0;
    let gap = 8.0;
    let total = n as f32 * h + (n.saturating_sub(1)) as f32 * gap + extra;
    let y0 = (78.0f32).max(((view.y - total) * 0.5 + 18.0).round());
    (0..n).map(|k| (Vec2::new(((view.x - w) * 0.5).round(), y0 + k as f32 * (h + gap)), Vec2::new(w, h))).collect()
}

/// Delai avant d'accepter un choix (evite de valider par erreur en attaquant).
pub const LEVELUP_LOCK: f32 = 0.3;

/// Place reservee sous les cartes de montee de niveau pour Relancer / Passer.
pub const LEVELUP_EXTRA: f32 = 30.0;

/// Choix a la montee de niveau : une carte, ou l'un des deux boutons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LevelPick {
    Card(usize),
    Reroll,
    Skip,
}

/// Boutons Relancer et Passer, sous les cartes.
pub fn levelup_buttons(view: Vec2, n: usize) -> [(Vec2, Vec2); 2] {
    let rects = card_rects_ex(view, n, LEVELUP_EXTRA);
    let (p, s) = rects.last().copied().unwrap_or((Vec2::new(20.0, 80.0), Vec2::new(view.x - 40.0, 64.0)));
    let y = p.y + s.y + 10.0;
    let bw = ((s.x - 8.0) * 0.5).floor();
    [(Vec2::new(p.x, y), Vec2::new(bw, 20.0)), (Vec2::new(p.x + s.x - bw, y), Vec2::new(bw, 20.0))]
}

#[allow(clippy::too_many_arguments)]
pub fn draw_levelup(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, offers: &[Offer], sel: usize, t: f32) {
    let infos: Vec<OfferInfo> = offers.iter().map(|&o| offer_info(world, vis, lang, o)).collect();
    // une fusion est proposee : le sous-titre l'annonce
    let sub = if infos.iter().any(|i| i.fusion > 0) {
        crate::hud::hl_fusion(lang.get("ui.choose_fusion"))
    } else if offers.iter().all(|o| matches!(o, Offer::Heal | Offer::Rupees)) {
        lang.get("ui.choose_fallback").to_string()
    } else {
        lang.get("ui.choose").to_string()
    };
    draw_cards_ex(list, ctx, vis, lang, (lang.get("ui.levelup"), &sub, GOLD), &infos, sel, t, LEVELUP_EXTRA, None);
    // Relancer / Passer : nombre restant dans la partie ; grise quand il n'en reste plus
    let main = ctx.atlas.font("main");
    let white = &vis.white;
    let n = offers.len();
    let u = ((t - n as f32 * 0.05) / 0.18).clamp(0.0, 1.0);
    let rt = ctx.real_time as f32;
    let counts = [world.mods.rerolls, world.mods.skips];
    let labels = [lang.get("ui.reroll"), lang.get("ui.skip")];
    for (k, (pos, size)) in levelup_buttons(ctx.view, n).into_iter().enumerate() {
        let on = counts[k] > 0;
        let selected = sel == n + k;
        let a = u * if on { 1.0 } else { 0.45 };
        list.rect(white, pos, size, with_alpha(CARD_BG, a));
        let border = if selected {
            with_alpha(GOLD, a * if t < LEVELUP_LOCK { 0.6 } else { 0.75 + 0.25 * (rt * 6.0).sin() })
        } else {
            [1.0, 1.0, 1.0, 0.3 * a]
        };
        frame_rect(list, white, pos, size, border);
        if selected && t >= LEVELUP_LOCK {
            crate::hub::focus_frame(list, ctx, pos, size);
        }
        let label = format!("{}  ({})", labels[k], counts[k]);
        let c = if selected { WHITE } else { SOFT };
        text(list, main, &label, Vec2::new(pos.x + size.x * 0.5, crate::hud::mid_y(main, pos.y, size.y, 1.0)), 1.0, with_alpha(c, a), Align::Center);
    }
}

/// Choix de l'arme qui evolue quand plusieurs partagent le meme objet partenaire : une carte par
/// arme evoluee, l'arme actuelle en dessous.
#[allow(clippy::too_many_arguments)]
pub fn draw_evo_choice(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, options: &[(usize, u16)], sel: usize, t: f32) {
    let c = &world.content;
    let partner = options
        .first()
        .and_then(|&(slot, into)| {
            let w = world.weapons.get(slot)?;
            c.weapons[w.id as usize].recipes().find(|e| e.into == c.weapon_ids[into as usize])
        })
        .map(|e| lang.get(&format!("passive.{}.name", e.passive)).to_string())
        .unwrap_or_default();
    let infos: Vec<OfferInfo> = options
        .iter()
        .map(|&(slot, into)| {
            let from = world.weapons.get(slot).map_or(0, |w| w.id);
            OfferInfo {
                icon: vis.weapons[into as usize].icon,
                name: lang.get(&format!("weapon.{}.name", c.weapon_ids[into as usize])).to_string(),
                tag: lang.get("ui.evolution").to_string(),
                is_new: true,
                desc: lang.get(&format!("weapon.{}.desc", c.weapon_ids[into as usize])).to_string(),
                level: 0,
                max: 0,
                evo: vec![(vis.weapons[from as usize].icon, lang.get(&format!("weapon.{}.name", c.weapon_ids[from as usize])).to_string(), true, false)],
                evo_label: lang.get("ui.evo_from").to_string(),
                keep: false,
                fusion: 1,
            }
        })
        .collect();
    let sub = lang.get("ui.evo_choose").replace("{a}", &crate::hud::hl(&partner));
    draw_cards(list, ctx, vis, lang, (lang.get("ui.evolution"), &sub, rgb(255, 160, 255)), &infos, sel, t);
}

/// Cartes de choix (montee de niveau, recompense de rencontre) : titre, sous-titre et couleur du titre.
#[allow(clippy::too_many_arguments)]
pub fn draw_cards(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, head: (&str, &str, Color), infos: &[OfferInfo], sel: usize, t: f32) {
    draw_cards_ex(list, ctx, vis, lang, head, infos, sel, t, 0.0, None);
}

/// Cartes d'un choix a part (butin d'un champion) : fond, bordure, coins ornes et etiquettes a la
/// couleur `theme`, fond de l'ecran teinte ; on ne les confond pas avec une montee de niveau.
#[allow(clippy::too_many_arguments)]
pub fn draw_cards_themed(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, head: (&str, &str, Color), infos: &[OfferInfo], sel: usize, t: f32, theme: Color) {
    draw_cards_ex(list, ctx, vis, lang, head, infos, sel, t, 0.0, Some(theme));
}

#[allow(clippy::too_many_arguments)]
fn draw_cards_ex(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, head: (&str, &str, Color), infos: &[OfferInfo], sel: usize, t: f32, extra: f32, theme: Option<Color>) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let white = &vis.white;
    // choix a part : l'ecran entier prend un voile de sa couleur
    let veil = theme.map_or(DIM, |c| [c[0] * 0.12, c[1] * 0.06, c[2] * 0.16, DIM[3]]);
    dim(list, vis, view, with_alpha(veil, (t / 0.15).min(1.0)));
    let rects = card_rects_ex(view, infos.len(), extra);
    let y0 = rects.first().map_or(94.0, |r| r.0.y);
    let big = crate::hud::title_scale(main, head.0, view.x, 2.0);
    let pop = if t < 0.1 { big + 1.0 } else { big };
    text(list, main, head.0, Vec2::new(view.x * 0.5, y0 - 54.0 - (pop - 2.0) * 7.0 + (2.0 - big) * 7.0), pop, head.2, Align::Center);
    text(list, main, head.1, Vec2::new(view.x * 0.5, y0 - 19.0), 1.0, SOFT, Align::Center);
    let locked = t < LEVELUP_LOCK;
    let rt = ctx.real_time as f32;
    for (k, ((pos, size), info)) in rects.into_iter().zip(infos).enumerate() {
        // fusion possible avec un objet (ou une arme) deja possede : la carte brille en rose
        let star = info.fusion;
        let fusion = info.evo.iter().any(|e| e.2) && !info.keep && star == 0;
        let keep = info.keep;
        // fusion proposee : couleur de la carte (rose ; or pour la fusion ultime)
        let hc_star = if star == 2 { KEEP } else { FUSION };
        // entree echelonnee par la droite
        let u = ((t - k as f32 * 0.05) / 0.18).clamp(0.0, 1.0);
        let e = 1.0 - (1.0 - u) * (1.0 - u);
        let selected = k == sel;
        let pos = pos + Vec2::new(((1.0 - e) * 60.0).round() + if selected { 4.0 } else { 0.0 }, 0.0);
        let a = e;
        let bg = if keep || star == 2 {
            KEEP_BG
        } else if fusion || star > 0 {
            FUSION_BG
        } else if let Some(c) = theme {
            [c[0] * 0.22, c[1] * 0.12, c[2] * 0.26, 0.95]
        } else {
            CARD_BG
        };
        list.rect(white, pos, size, with_alpha(bg, a));
        let pulse = 0.75 + 0.25 * (rt * 6.0).sin();
        let border = if selected {
            with_alpha(GOLD, a * if locked { 0.6 } else { pulse })
        } else if star > 0 {
            with_alpha(hc_star, a * (0.75 + 0.25 * (rt * 5.0).sin()))
        } else if keep {
            with_alpha(KEEP, a * (0.7 + 0.3 * (rt * 4.0).sin()))
        } else if let Some(c) = theme {
            with_alpha(c, a * (0.6 + 0.2 * (rt * 3.0).sin()))
        } else if fusion {
            with_alpha(FUSION, a * (0.55 + 0.35 * (rt * 4.0).sin()))
        } else {
            [1.0, 1.0, 1.0, 0.3 * a]
        };
        frame_rect(list, white, pos, size, border);
        if selected && !locked {
            crate::hub::focus_frame(list, ctx, pos, size);
        } else if selected {
            frame_rect(list, white, pos + Vec2::ONE, size - Vec2::splat(2.0), with_alpha(border, 0.5));
        }
        if let Some(c) = theme {
            // coins ornes, hors de la carte : la forme dit « choix a part » avant la couleur
            let cc = with_alpha(c, a);
            for (x, y, sx, sy) in [(0.0, 0.0, -1.0, -1.0), (1.0, 0.0, 1.0, -1.0), (0.0, 1.0, -1.0, 1.0), (1.0, 1.0, 1.0, 1.0)] {
                let corner = pos + Vec2::new(x * size.x, y * size.y);
                let o = corner + Vec2::new(if sx < 0.0 { -4.0 } else { 2.0 }, if sy < 0.0 { -4.0 } else { 2.0 });
                let hx = if sx < 0.0 { o.x } else { o.x - 8.0 };
                let vy = if sy < 0.0 { o.y } else { o.y - 8.0 };
                list.rect(white, Vec2::new(hx, o.y), Vec2::new(10.0, 2.0), cc);
                list.rect(white, Vec2::new(o.x, vy), Vec2::new(2.0, 10.0), cc);
            }
        }
        if fusion || keep || star > 0 {
            // halo qui respire autour de la carte (rose : fusion possible ; or : objet garde) et des
            // etincelles qui en font le tour ; fusion proposee : le halo le plus fort, reflet compris
            let strong = keep || star > 0;
            let hc = if star > 0 { hc_star } else if keep { KEEP } else { FUSION };
            let glow = if strong { 0.4 + 0.25 * (rt * 4.0).sin() } else { 0.25 + 0.2 * (rt * 4.0).sin() };
            list.set(Space::Ui, Blend::Add);
            frame_rect(list, white, pos - Vec2::splat(2.0), size + Vec2::splat(4.0), with_alpha(hc, glow * a));
            frame_rect(list, white, pos - Vec2::splat(3.0), size + Vec2::splat(6.0), with_alpha(hc, glow * 0.5 * a));
            if strong {
                frame_rect(list, white, pos - Vec2::splat(5.0), size + Vec2::splat(10.0), with_alpha(hc, glow * 0.25 * a));
                // reflet qui balaie la carte
                let sweep = ((rt * 0.6).fract() * (size.x + 60.0)) - 30.0;
                for i in 0..6 {
                    let x = (pos.x + sweep + i as f32 * 3.0).clamp(pos.x, pos.x + size.x - 2.0);
                    list.rect(white, Vec2::new(x.round(), pos.y + 1.0), Vec2::new(2.0, size.y - 2.0), with_alpha(hc, 0.05 * a));
                }
            }
            let per = 2.0 * (size.x + size.y);
            let n = if star > 0 { 6 } else if keep { 4 } else { 2 };
            for s in 0..n {
                let d = ((rt * 140.0 + s as f32 * per / n as f32) % per + per) % per;
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
                list.rect(white, (q - Vec2::splat(3.0)).round(), Vec2::splat(6.0), with_alpha(hc, 0.35 * a));
            }
            list.set(Space::Ui, Blend::Alpha);
        }
        if keep || star > 0 {
            // bandeau a cheval sur le bord haut : « Gardee apres la partie » (or), « Fusion ! » (rose),
            // « Fusion ultime ! » (or)
            let label = match star {
                1 => lang.get("ui.fusion_banner"),
                2 => lang.get("ui.fusion_ultimate"),
                _ => lang.get("ui.keep_banner"),
            };
            let (fill, edge, ink) = if star == 1 { (FUSION, rgb(110, 20, 95), rgb(60, 0, 50)) } else { (KEEP, rgb(120, 70, 0), rgb(60, 30, 0)) };
            let lw = main.measure(label) as f32 + 12.0;
            let bp = Vec2::new((pos.x + size.x * 0.5 - lw * 0.5).round(), pos.y - 8.0);
            list.rect(white, bp, Vec2::new(lw, 16.0), with_alpha(fill, a));
            frame_rect(list, white, bp, Vec2::new(lw, 16.0), with_alpha(edge, a));
            text(list, main, label, Vec2::new(bp.x + lw * 0.5, crate::hud::mid_y(main, bp.y, 16.0, 1.0)), 1.0, with_alpha(ink, a), Align::Center);
        }
        // icone x2 dans sa case
        let ib = Vec2::new(44.0, 44.0);
        let ip = pos + Vec2::new(10.0, (size.y - ib.y) * 0.5);
        list.rect(white, ip, ib, [0.0, 0.0, 0.0, 0.5 * a]);
        frame_rect(list, white, ip, ib, [1.0, 1.0, 1.0, 0.25 * a]);
        let mut icon_at = ip + ib * 0.5;
        if star > 0 {
            // l'arme obtenue rayonne dans sa case et flotte doucement
            list.set(Space::Ui, Blend::Add);
            let glow = 0.18 + 0.1 * (rt * 4.0).sin();
            for r in [18.0f32, 13.0, 8.0] {
                let gp = (ip + ib * 0.5 - Vec2::splat(r)).round();
                list.rect(white, gp, Vec2::splat(r * 2.0), with_alpha(hc_star, glow * a));
            }
            list.set(Space::Ui, Blend::Alpha);
            icon_at.y -= (1.5 * (rt * 3.0).sin()).round();
        }
        list.frame(atlas.frame(info.icon, 0.0), icon_at, &Params::default().scale(Vec2::splat(2.0)).color([1.0, 1.0, 1.0, a]));
        // touche directe
        text(list, atlas.font("hud"), &(k + 1).to_string(), pos + Vec2::new(4.0, 4.0), 1.0, [1.0, 1.0, 1.0, 0.4 * a], Align::Left);
        let tx = ip.x + ib.x + 10.0;
        let name_c = if keep || star == 2 { KEEP } else if star == 1 { FUSION_LIGHT } else if selected { WHITE } else { SOFT };
        text(list, main, &info.name, Vec2::new(tx, pos.y + 6.0), 1.0, with_alpha(name_c, a), Align::Left);
        let mut tag_w = 0.0;
        if !info.tag.is_empty() {
            let c = if star > 0 { hc_star } else if let (Some(c), false) = (theme, keep) { c } else if info.is_new { NEW_GREEN } else { GOLD };
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
            let lc = if star > 0 { hc_star } else if fusion { FUSION } else { rgb(190, 170, 110) };
            let mut x = tx + text(list, main, lbl, Vec2::new(tx, y), 1.0, with_alpha(lc, a), Align::Left) + 10.0;
            for (icon, name, owned, hidden) in &info.evo {
                let c = if *owned { FUSION_LIGHT } else { [1.0, 1.0, 1.0, 0.55] };
                // l'objet deja possede sautille ; inconnu : silhouette sombre
                let hop = if *owned { (2.0 * (rt * 6.0).sin().abs()).round() } else { 0.0 };
                let ic = if *hidden { [0.05, 0.05, 0.1, 0.9] } else if *owned { WHITE } else { c };
                list.frame(atlas.frame(*icon, 0.0), Vec2::new(x, y + 6.0 - hop), &Params::default().color(with_alpha(ic, a)));
                x += 10.0;
                // partenaire possede : nom mis en valeur (la fusion est a portee)
                let shown = if *owned { crate::hud::hl_fusion(name) } else { name.clone() };
                x += text(list, main, &shown, Vec2::new(x, y), 1.0, with_alpha(c, a), Align::Left) + 12.0;
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

/// Montee de niveau : cartes (haut / bas), puis les boutons Relancer et Passer dessous
/// (gauche / droite entre les deux). `avail` : il en reste dans la partie.
pub fn input_levelup_ex(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize, t: f32, avail: [bool; 2]) -> (Option<LevelPick>, bool) {
    let mut moved = false;
    let last = n.saturating_sub(1);
    // bouton epuise : le curseur revient sur l'autre, ou sur la derniere carte
    if *sel >= n && !avail.get(*sel - n).copied().unwrap_or(false) {
        let other = if *sel == n { n + 1 } else { n };
        *sel = if avail[other - n] { other } else { last };
    }
    if nav.up {
        if *sel >= n {
            *sel = last;
            moved = true;
        } else if *sel > 0 {
            *sel -= 1;
            moved = true;
        }
    } else if nav.down {
        if *sel + 1 < n {
            *sel += 1;
            moved = true;
        } else if *sel == last && *sel < n {
            if avail[0] {
                *sel = n;
                moved = true;
            } else if avail[1] {
                *sel = n + 1;
                moved = true;
            }
        }
    } else if (nav.left || nav.right) && *sel >= n {
        let other = if *sel == n { n + 1 } else { n };
        if avail[other - n] {
            *sel = other;
            moved = true;
        }
    }
    let rects = card_rects_ex(view, n, LEVELUP_EXTRA);
    let buttons = levelup_buttons(view, n);
    if let Some(m) = nav.hover {
        for (k, (p, s)) in rects.iter().enumerate() {
            if inside(m, *p, *s) && *sel != k {
                *sel = k;
                moved = true;
            }
        }
        for (k, (p, s)) in buttons.iter().enumerate() {
            if avail[k] && inside(m, *p, *s) && *sel != n + k {
                *sel = n + k;
                moved = true;
            }
        }
    }
    if t < LEVELUP_LOCK {
        return (None, moved);
    }
    let pick = |k: usize| if k < n { LevelPick::Card(k) } else if k == n { LevelPick::Reroll } else { LevelPick::Skip };
    if let Some(d) = nav.digit.filter(|&d| d < n) {
        return (Some(LevelPick::Card(d)), moved);
    }
    if nav.click {
        for (k, (p, s)) in rects.iter().enumerate() {
            if inside(nav.mouse, *p, *s) {
                return (Some(LevelPick::Card(k)), moved);
            }
        }
        for (k, (p, s)) in buttons.iter().enumerate() {
            if avail[k] && inside(nav.mouse, *p, *s) {
                return (Some(pick(n + k)), moved);
            }
        }
    }
    (nav.confirm.then(|| pick(*sel)), moved)
}

// ---------------------------------------------------------------------------------------------
// Coffre

/// Instant ou le coffre s'ouvre, puis ou l'on peut continuer.
pub const CHEST_OPEN: f32 = 0.7;
pub const CHEST_DONE: f32 = 1.5;

#[allow(clippy::too_many_arguments)]
pub fn draw_chest(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, reward: &ChestReward, big: bool, t: f32) {
    if let Some((from, into)) = reward.evolution {
        draw_evolution(list, ctx, world, vis, lang, reward, big, t, from, into);
        return;
    }
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
    text(list, main, title, Vec2::new(view.x * 0.5, 46.0), crate::hud::title_scale(main, title, view.x, 2.0), tc, Align::Center);
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
            Offer::Heal => (vis.specials[1], lang.get("ui.heal").to_string(), 0),
            Offer::Rupees => (vis.icon_rupees, lang.get("ui.rupees").to_string(), 0),
            Offer::Evolve(id) => (vis.weapons[id as usize].icon, lang.get(&format!("weapon.{}.name", world.content.weapon_ids[id as usize])).to_string(), 0),
        };
        let line = if lv > 0 { format!("{name}  {} {lv}", lang.get("ui.level")) } else { name };
        let w = main.measure(&line) as f32 + 22.0;
        let x = ((view.x - w) * 0.5).round();
        let yy = y + (1.0 - uu) * 8.0;
        list.rect(white, Vec2::new(x - 6.0, yy - 4.0), Vec2::new(w + 12.0, 20.0), with_alpha(BACK, uu * 0.8));
        list.frame(atlas.frame(icon, 0.0), Vec2::new(x + 6.0, yy + 6.0), &Params::default().color([1.0, 1.0, 1.0, uu]));
        text(list, main, &line, Vec2::new(x + 20.0, crate::hud::mid_y(main, yy - 4.0, 20.0, 1.0)), 1.0, with_alpha(WHITE, uu), Align::Left);
        y += 24.0;
    }
    if t >= CHEST_DONE && (ctx.real_time * 2.0).fract() < 0.7 {
        text(list, main, lang.get("ui.continue"), Vec2::new(view.x * 0.5, view.y - 26.0), 1.0, SOFT, Align::Center);
    }
}

// ---------------------------------------------------------------------------------------------
// Fusion (coffre avec evolution) : retour 0.6.2, un moment fort. L'arme et son objet partenaire
// sortent du coffre, tournent l'un autour de l'autre en se rapprochant, se fondent dans un eclair,
// puis l'arme evoluee apparait avec son nom, la fusion, ce qui change et sa description.

/// La fusion a lieu ; on peut continuer a partir de EVO_DONE.
pub const EVO_MERGE: f32 = 1.9;
pub const EVO_DONE: f32 = 3.2;

const FUSION_PINK: Color = rgb(255, 130, 240);

/// Point de couleur additif (etincelles, trainees).
fn spark(list: &mut DrawList, white: &assetkit::meta::Frame, p: Vec2, size: f32, c: Color) {
    list.rect(white, (p - Vec2::splat(size * 0.5)).round(), Vec2::splat(size), c);
}

/// Rayons de lumiere tournants autour d'un point.
#[allow(clippy::too_many_arguments)]
fn rays(list: &mut DrawList, white: &assetkit::meta::Frame, at: Vec2, len: f32, n: usize, rot: f32, a: f32, c1: Color, c2: Color) {
    list.set(Space::Ui, Blend::Add);
    for r in 0..n {
        let ang = r as f32 / n as f32 * std::f32::consts::TAU + rot;
        let c = if r % 2 == 0 { c1 } else { c2 };
        let steps = (len / 6.0) as usize;
        for s in 0..steps {
            let d = 8.0 + s as f32 * 6.0;
            let al = (1.0 - s as f32 / steps as f32) * 0.4 * a;
            spark(list, white, at + Vec2::from_angle(ang) * d, 3.0, with_alpha(c, al));
        }
    }
    list.set(Space::Ui, Blend::Alpha);
}

/// Ligne « Degats  24 ▶ 48 » centree : ancienne valeur grise, nouvelle en vert.
#[allow(clippy::too_many_arguments)]
fn stat_line(list: &mut DrawList, font: &Font, label: &str, old: &str, new: &str, cx: f32, y: f32, a: f32) {
    let parts = [(label.to_string(), SOFT), (old.to_string(), rgb(150, 150, 170)), ("▶".to_string(), FUSION_PINK), (new.to_string(), rgb(120, 255, 150))];
    let gap = 6.0;
    let total: f32 = parts.iter().map(|(s, _)| font.measure(s) as f32).sum::<f32>() + gap * 3.0;
    let mut x = (cx - total * 0.5).round();
    for (s, c) in parts {
        x += text(list, font, &s, Vec2::new(x, y), 1.0, with_alpha(c, a), Align::Left) + gap;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_evolution(list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, reward: &ChestReward, big: bool, t: f32, from: u16, into: u16) {
    use std::f32::consts::{PI, TAU};
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let white = &vis.white;
    let rt = ctx.real_time as f32;
    let c = &world.content;
    dim(list, vis, view, with_alpha(DIM, (t / 0.2).min(1.0)));
    // le fond vire au rose pendant la fusion
    let pink = ((t - CHEST_OPEN) / 1.0).clamp(0.0, 1.0);
    list.rect(white, Vec2::ZERO, view, [0.35, 0.04, 0.32, 0.22 * pink]);
    let cx = (view.x * 0.5).round();
    let merge = Vec2::new(cx, 96.0);
    let chest_pos = Vec2::new(cx, (view.y * 0.5).round());
    let open = t >= CHEST_OPEN;
    let merged = t >= EVO_MERGE;
    let since = t - EVO_MERGE;
    // secousse au moment de la fusion
    let shake = if merged && since < 0.35 {
        let k = 1.0 - since / 0.35;
        Vec2::new(((t * 90.0).sin() * 3.0 * k).round(), ((t * 77.0).cos() * 2.0 * k).round())
    } else {
        Vec2::ZERO
    };
    // titre qui chatoie
    let ts = if merged && since < 0.12 { 3.0 } else { 2.0 };
    text(list, main, &crate::hud::hl_fusion(lang.get("ui.evolution")), Vec2::new(cx, 18.0) + shake, ts, WHITE, Align::Center);

    // le coffre : tremble, s'ouvre, puis s'efface pendant que les objets en sortent
    let ca = if open { (1.0 - (t - CHEST_OPEN) / 0.9).clamp(0.0, 1.0) } else { 1.0 };
    if ca > 0.0 {
        let sh = if open { Vec2::ZERO } else { Vec2::new(((t * 60.0).sin() * (t / CHEST_OPEN) * 2.0).round(), 0.0) };
        let anim = if open { vis.chest_open[big as usize] } else { vis.chest[big as usize] };
        let sc = if big { 2.0 } else { 3.0 };
        list.frame(atlas.frame(anim, 0.0), chest_pos + sh, &Params::default().scale(Vec2::splat(sc)).color([1.0, 1.0, 1.0, ca]));
    }
    if !open {
        return;
    }
    let partner = c.weapons[from as usize].recipes().find(|e| e.into == c.weapon_ids[into as usize]).and_then(|e| c.passive_index(&e.passive));
    let from_name = lang.get(&format!("weapon.{}.name", c.weapon_ids[from as usize])).to_string();
    let partner_name = partner.map(|p| lang.get(&format!("passive.{}.name", c.passive_ids[p as usize])).to_string()).unwrap_or_default();
    let fi = vis.weapons[from as usize].icon;
    let ii = vis.weapons[into as usize].icon;

    if !merged {
        // approche : les deux objets spiralent l'un vers l'autre, de plus en plus vite
        let u = ((t - CHEST_OPEN) / (EVO_MERGE - CHEST_OPEN)).clamp(0.0, 1.0);
        let pos_at = |u: f32, side: f32| {
            let e = u * u;
            let r = 84.0 * (1.0 - e);
            let ang = if side < 0.0 { PI } else { 0.0 } + e * TAU * 1.5;
            let orbit = merge + Vec2::new(ang.cos() * r, ang.sin() * r * 0.55);
            chest_pos.lerp(orbit, (u / 0.18).min(1.0))
        };
        rays(list, white, merge, 30.0 + 60.0 * u, 12, rt * 0.8, u, FUSION_PINK, GOLD);
        // eclair entre les deux quand ils se rapprochent
        let pa = pos_at(u, -1.0);
        let pb = pos_at(u, 1.0);
        if u > 0.45 {
            list.set(Space::Ui, Blend::Add);
            let n = 14;
            for k in 0..=n {
                let f = k as f32 / n as f32;
                let jit = ((rt * 40.0 + k as f32 * 7.3).sin() * 4.0).round();
                let p = pa.lerp(pb, f) + Vec2::new(0.0, jit * (1.0 - (2.0 * f - 1.0).abs()));
                spark(list, white, p, 2.0, [1.0, 0.8, 1.0, 0.8 * (u - 0.45) / 0.55]);
            }
            list.set(Space::Ui, Blend::Alpha);
        }
        for (side, icon, col, label) in [(-1.0, fi, GOLD, &from_name), (1.0, partner.map_or(fi, |p| vis.passives[p as usize]), FUSION_PINK, &partner_name)] {
            // trainee
            list.set(Space::Ui, Blend::Add);
            for k in 1..10 {
                let q = pos_at((u - k as f32 * 0.018).max(0.0), side);
                spark(list, white, q, 3.0 - k as f32 * 0.2, with_alpha(col, 0.5 * (1.0 - k as f32 / 10.0)));
            }
            list.set(Space::Ui, Blend::Alpha);
            let p = pos_at(u, side);
            list.frame(atlas.frame(icon, 0.0), p.round(), &Params::default().scale(Vec2::splat(2.0)).flash([1.0, 1.0, 1.0], u * u));
            // nom sous chaque objet, au debut
            let la = (1.0 - u / 0.5).clamp(0.0, 1.0);
            if la > 0.0 {
                let name = if side < 0.0 { crate::hud::hl(label) } else { crate::hud::hl_fusion(label) };
                text(list, main, &name, (p + Vec2::new(0.0, 20.0)).round(), 1.0, with_alpha(WHITE, la), Align::Center);
            }
        }
        return;
    }

    // eclair blanc, anneaux et gerbe d'etincelles
    if since < 0.4 {
        list.rect(white, Vec2::ZERO, view, [1.0, 1.0, 1.0, 0.9 * (1.0 - since / 0.4)]);
    }
    list.set(Space::Ui, Blend::Add);
    if since < 0.9 {
        for (k, col) in [(0, WHITE), (1, FUSION_PINK), (2, GOLD)] {
            let r = since * (180.0 + k as f32 * 90.0);
            let a = (1.0 - since / 0.9) * 0.8;
            let n = 48;
            for s in 0..n {
                let ang = s as f32 / n as f32 * TAU;
                spark(list, white, merge + Vec2::new(ang.cos() * r, ang.sin() * r * 0.7), 3.0, with_alpha(col, a));
            }
        }
        for s in 0..32 {
            let ang = s as f32 * 2.399;
            let speed = 120.0 + (s as f32 * 37.0) % 140.0;
            let p = merge + Vec2::from_angle(ang) * speed * since + Vec2::new(0.0, 60.0 * since * since);
            spark(list, white, p, 3.0, with_alpha(if s % 2 == 0 { FUSION_PINK } else { GOLD }, 1.0 - since / 0.9));
        }
    }
    list.set(Space::Ui, Blend::Alpha);
    // l'arme evoluee : rayons, halo d'etincelles qui tournent, legere oscillation
    rays(list, white, merge + shake, 80.0, 16, rt * 0.5, 1.0, FUSION_PINK, GOLD);
    list.set(Space::Ui, Blend::Add);
    for k in 0..8 {
        let ang = rt * 1.6 + k as f32 * TAU / 8.0;
        let p = merge + Vec2::new(ang.cos() * 40.0, ang.sin() * 22.0);
        spark(list, white, p, 2.0 + (rt * 5.0 + k as f32).sin().abs() * 2.0, [1.0, 0.85, 1.0, 0.9]);
    }
    list.set(Space::Ui, Blend::Alpha);
    let sc = if since < 0.12 { 5.0 } else { 4.0 };
    let bob = ((rt * 2.2).sin() * 2.0).round();
    list.frame(atlas.frame(ii, 0.0), merge + shake + Vec2::new(0.0, bob), &Params::default().scale(Vec2::splat(sc)).flash([1.0, 1.0, 1.0], (1.0 - since / 0.3).max(0.0)));

    let fade = |start: f32| ((since - start) / 0.3).clamp(0.0, 1.0);
    let mut y = merge.y + 44.0;
    let name = lang.get(&format!("weapon.{}.name", c.weapon_ids[into as usize]));
    text(list, main, &crate::hud::hl_fusion(name), Vec2::new(cx, y), 2.0, with_alpha(WHITE, fade(0.1)), Align::Center);
    y += 30.0;
    let fusion = lang.get("ui.evo_fusion").replace("{a}", &crate::hud::hl(&from_name)).replace("{b}", &crate::hud::hl(&partner_name));
    text(list, main, &fusion, Vec2::new(cx, y), 1.0, with_alpha(SOFT, fade(0.25)), Align::Center);
    y += 18.0;
    // ce qui change : arme d'origine au niveau maximal contre l'arme evoluee
    if let Some(slot) = world.weapons.iter().position(|w| w.id == into) {
        let ws = &world.weapons[slot];
        let new = ws.stats;
        let fd = &c.weapons[from as usize];
        let mut old = sim::upgrades::resolve(fd, fd.max_level(), &world.player.stats);
        ws.boost.apply(&mut old);
        let mut lines: Vec<(String, String, String)> = Vec::new();
        if (new.damage - old.damage).abs() >= 0.5 {
            lines.push((lang.get("stat.damage").to_string(), format!("{:.0}", old.damage), format!("{:.0}", new.damage)));
        }
        if new.amount != old.amount {
            lines.push((lang.get("stat.amount").to_string(), old.amount.to_string(), new.amount.to_string()));
        }
        if (new.cooldown - old.cooldown).abs() >= 0.05 {
            lines.push((lang.get("stat.cooldown").to_string(), format!("{:.1} s", old.cooldown), format!("{:.1} s", new.cooldown)));
        }
        if (new.area - old.area).abs() >= 1.0 {
            lines.push((lang.get("stat.area").to_string(), format!("{:.0}", old.area), format!("{:.0}", new.area)));
        }
        for (k, (l, o, n)) in lines.iter().take(4).enumerate() {
            let a = fade(0.4 + k as f32 * 0.12);
            stat_line(list, main, l, o, n, cx, y + (1.0 - a) * 6.0, a);
            y += 13.0;
        }
    }
    y += 6.0;
    let desc = lang.get(&format!("weapon.{}.desc", c.weapon_ids[into as usize]));
    for l in wrap(main, desc, view.x - 80.0, 1.0).iter().take(2) {
        text(list, main, l, Vec2::new(cx, y), 1.0, with_alpha(rgb(230, 230, 240), fade(0.8)), Align::Center);
        y += 13.0;
    }
    // grand coffre : les ameliorations en plus, sur une ligne d'icones
    if !reward.upgrades.is_empty() {
        y += 6.0;
        let n = reward.upgrades.len() as f32;
        let x0 = cx - (n - 1.0) * 14.0;
        for (k, &o) in reward.upgrades.iter().enumerate() {
            let a = fade(1.0 + k as f32 * 0.1);
            let icon = match o {
                Offer::UpgradeWeapon(id) | Offer::NewWeapon(id) => vis.weapons[id as usize].icon,
                Offer::UpgradePassive(id) | Offer::NewPassive(id) => vis.passives[id as usize],
                Offer::Heal => vis.specials[1],
                Offer::Rupees => vis.icon_rupees,
                Offer::Evolve(id) => vis.weapons[id as usize].icon,
            };
            let p = Vec2::new(x0 + k as f32 * 28.0, y + 8.0);
            list.rect(white, p - Vec2::splat(11.0), Vec2::splat(22.0), with_alpha(BACK, 0.8 * a));
            list.frame(atlas.frame(icon, 0.0), p, &Params::default().color([1.0, 1.0, 1.0, a]));
        }
    }
    if t >= EVO_DONE && (ctx.real_time * 2.0).fract() < 0.7 {
        text(list, main, lang.get("ui.continue"), Vec2::new(cx, view.y - 26.0), 1.0, SOFT, Align::Center);
    }
}

// ---------------------------------------------------------------------------------------------
// Deblocages de fin de partie : personnages et armes, tous affiches, l'un apres l'autre.

/// Ce qu'une partie a debloque.
#[derive(Clone, Debug, PartialEq)]
pub enum Unlock {
    Char(String),
    Weapon(u16),
}

/// Ecran qui suit l'ecran de deblocage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AfterUnlock {
    Victory,
    Dead,
    Title,
    /// « Recommencer » depuis la pause : nouvelle partie ensuite.
    Restart,
}

/// Ecart entre deux apparitions : plus la page est pleine, plus elles s'enchainent (une page
/// pleine se revele en 4 s environ).
pub fn unlock_gap(n: usize) -> f32 {
    (4.0 / n.max(1) as f32).clamp(0.15, 0.55)
}

/// Instant d'apparition du deblocage `i` d'une page de `n`.
pub fn unlock_at(i: usize, n: usize) -> f32 {
    0.5 + i as f32 * unlock_gap(n)
}

/// Fin de l'animation d'une page de `n` deblocages.
pub fn unlocks_done(n: usize) -> f32 {
    unlock_at(n.saturating_sub(1), n) + 0.6
}

/// Pas d'appui au debut de l'ecran (boutons encore martelés pendant la partie), ni juste apres
/// la fin de l'animation : « Continuer » n'apparait qu'a ce moment-la.
pub const UNLOCK_LOCK: f32 = 0.6;
pub const UNLOCK_SETTLE: f32 = 0.3;

/// Deblocages par page : autant que la grille en tient avec les plus petites figures (3 rangees
/// en 16:9) ; les suivants viennent sur les pages d'apres.
pub fn unlocks_per_page(view: Vec2) -> usize {
    let cols = (((view.x - 16.0) / 84.0).floor() as usize).clamp(3, 8);
    // memes mesures que draw_unlocks a l'echelle 1, trois notes sous la grille
    let room = (view.y - 32.0 - 13.0 * 3.0 - 4.0) - 62.0;
    cols * ((room / 70.0).floor() as usize).max(1)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_unlocks(list: &mut DrawList, ctx: &Ctx, content: &sim::defs::Content, vis: &Vis, lang: &Lang, all: &[Unlock], t: f32) {
    let items = &all[..all.len().min(unlocks_per_page(ctx.view))];
    let more = all.len() - items.len();
    let n_items = items.len();
    use std::f32::consts::TAU;
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let white = &vis.white;
    let rt = ctx.real_time as f32;
    let fade_in = (t / 0.25).min(1.0);
    dim(list, vis, view, with_alpha(DIM, fade_in * 0.9));
    list.rect(white, Vec2::ZERO, view, [0.0, 0.0, 0.02, 0.3 * fade_in]);
    let cx = (view.x * 0.5).round();
    let big = crate::hud::title_scale(main, lang.get("ui.unlocks_title"), view.x, 3.0);
    let ts = if t < 0.12 { big + 1.0 } else { big };
    text(list, main, &crate::hud::hl(lang.get("ui.unlocks_title")), Vec2::new(cx, 22.0), ts, WHITE, Align::Center);
    // grille : autant de colonnes que la largeur le permet, rangees equilibrees ; plus il y a de
    // rangees, plus les figures sont petites, pour que tout tienne entre le titre et les notes
    let n = items.len().max(1);
    let avail = view.x - 16.0;
    let rows = n.div_ceil(((avail / 84.0).floor() as usize).clamp(3, 8));
    let per_row = n.div_ceil(rows);
    let cell = (avail / per_row as f32).min(112.0).floor();
    let sc = match rows {
        1 => 3.0,
        2 => 2.0,
        _ => 1.0,
    };
    let kind_label = rows < 3;
    let above = 26.0 * sc;
    let below = 10.0 + 4.0 * sc + if kind_label { 39.0 } else { 26.0 };
    let row_h = above + below + 4.0;
    // notes sous la grille (ce que cela change pour le joueur), au-dessus de « Continuer »
    let ultimate = |u: &Unlock| matches!(u, Unlock::Weapon(w) if content.weapons[*w as usize].evolved);
    let mut notes = Vec::new();
    if items.iter().any(|u| matches!(u, Unlock::Weapon(_)) && !ultimate(u)) {
        notes.push(lang.get("ui.unlocks_weapons_note").to_string());
    }
    if items.iter().any(ultimate) {
        notes.push(crate::hud::hl_fusion(lang.get("ui.ultimate_unlocked_sub")));
    }
    if items.iter().any(|u| matches!(u, Unlock::Char(_))) {
        notes.push(lang.get("ui.unlocks_chars_note").to_string());
    }
    let notes_y = view.y - 32.0 - 13.0 * notes.len() as f32;
    let (top, bottom) = (62.0, notes_y - 4.0);
    let y0 = (top + ((bottom - top - rows as f32 * row_h) * 0.5).max(0.0) + above).round();
    for (i, it) in items.iter().enumerate() {
        let ti = unlock_at(i, n_items);
        if t < ti {
            continue;
        }
        let since = t - ti;
        let row = i / per_row;
        let in_row = (items.len() - row * per_row).min(per_row);
        let col = i % per_row;
        let x = cx + (col as f32 - (in_row as f32 - 1.0) * 0.5) * cell;
        let at = Vec2::new(x.round(), y0 + row as f32 * row_h);
        let (col_main, label) = match it {
            Unlock::Char(_) => (GOLD, lang.get("ui.unlock_kind_char")),
            // fusion ultime (Master Sword) : rose des fusions
            Unlock::Weapon(w) if content.weapons[*w as usize].evolved => (rgb(255, 150, 240), lang.get("ui.unlock_kind_ultimate")),
            Unlock::Weapon(_) => (rgb(120, 255, 150), lang.get("ui.unlock_kind_weapon")),
        };
        // apparition : colonne de lumiere qui descend sur la case, anneau et etincelles
        list.set(Space::Ui, Blend::Add);
        if since < 0.6 {
            let k = 1.0 - since / 0.6;
            let bw = (4.0 + 18.0 * k).round();
            let foot = at.y + 4.0;
            list.rect(white, Vec2::new((at.x - bw * 0.5).round(), 0.0), Vec2::new(bw, foot), with_alpha(col_main, 0.45 * k));
            let core = (bw / 3.0).max(2.0).round();
            list.rect(white, Vec2::new((at.x - core * 0.5).round(), 0.0), Vec2::new(core, foot), [1.0, 1.0, 1.0, 0.6 * k]);
        }
        if since < 0.5 {
            let r = since * 160.0;
            for s in 0..32 {
                let ang = s as f32 / 32.0 * TAU;
                spark(list, white, at + Vec2::new(ang.cos() * r, ang.sin() * r * 0.6), 3.0, with_alpha(col_main, 1.0 - since / 0.5));
            }
        }
        list.set(Space::Ui, Blend::Alpha);
        // socle : ombre, rayons tournants, etincelles qui montent
        rays(list, white, at - Vec2::new(0.0, 10.0 * sc / 3.0), 26.0 * sc, 10, rt * 0.7 + i as f32, (since / 0.3).min(1.0), col_main, WHITE);
        list.rect(white, at + Vec2::new(-12.0 * sc / 3.0 * 2.0, 2.0), Vec2::new(16.0 * sc, 4.0), [0.0, 0.0, 0.0, 0.35]);
        list.set(Space::Ui, Blend::Add);
        for k in 0..4 {
            let ph = (rt * 0.6 + k as f32 * 0.25 + i as f32 * 0.13).fract();
            let p = at + Vec2::new(((k as f32 * 2.7 + i as f32).sin() * 14.0 * sc / 3.0).round(), -ph * 40.0 * sc / 3.0);
            spark(list, white, p, 2.0, with_alpha(col_main, (1.0 - ph) * 0.9));
        }
        list.set(Space::Ui, Blend::Alpha);
        let pop = if since < 0.1 { sc + 1.0 } else { sc };
        let bob = ((rt * 2.0 + i as f32).sin() * 1.5).round();
        let name = match it {
            Unlock::Char(id) => {
                if let Some(d) = content.characters.get(id) {
                    let anim = atlas.find(&d.sprite, "walk_down").or_else(|| atlas.find(&d.sprite, "idle_down"));
                    if let Some(anim) = anim {
                        list.frame(atlas.frame(anim, rt), at + Vec2::new(0.0, bob), &Params::default().scale(Vec2::splat(pop)));
                    }
                }
                crate::chars::name(lang, id)
            }
            Unlock::Weapon(w) => {
                let icon = vis.weapons[*w as usize].icon;
                list.frame(atlas.frame(icon, 0.0), at - Vec2::new(0.0, 8.0 * sc / 3.0 - bob), &Params::default().scale(Vec2::splat(pop)));
                lang.get(&format!("weapon.{}.name", content.weapon_ids[*w as usize])).to_string()
            }
        };
        let a = (since / 0.3).min(1.0);
        let mut ny = at.y + 10.0 + 4.0 * sc;
        // nom sur deux lignes au plus, dans la largeur de sa case
        for l in wrap(main, &name, cell - 6.0, 1.0).iter().take(2) {
            text(list, main, &crate::hud::hl_new(l), Vec2::new(at.x, ny), 1.0, with_alpha(WHITE, a), Align::Center);
            ny += 13.0;
        }
        if kind_label {
            text(list, main, label, Vec2::new(at.x, ny), 1.0, with_alpha(col_main, a * 0.8), Align::Center);
        }
    }
    let done = unlocks_done(n_items);
    if t >= done {
        for (k, s) in notes.iter().enumerate() {
            text(list, main, s, Vec2::new(cx, notes_y + 13.0 * k as f32), 1.0, SOFT, Align::Center);
        }
        if t >= done + UNLOCK_SETTLE && (ctx.real_time * 2.0).fract() < 0.7 {
            let next = if more > 0 { lang.get("ui.unlocks_more").replace("{n}", &more.to_string()) } else { lang.get("ui.continue").to_string() };
            text(list, main, &next, Vec2::new(cx, view.y - 26.0), 1.0, if more > 0 { GOLD } else { SOFT }, Align::Center);
        }
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

#[allow(clippy::too_many_arguments)]
pub fn draw_pause(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, sel: usize, world: &World, known: &std::collections::BTreeSet<String>, fallback: Option<Offer>) {
    let view = ctx.view;
    let main = ctx.atlas.font("main");
    dim(list, vis, view, DIM);
    text(list, main, lang.get("ui.paused"), Vec2::new(view.x * 0.5, (view.y * 0.12).round()), 3.0, WHITE, Align::Center);
    // bonus de niveau garde (tout est au maximum) : une ligne de plus, qui l'inverse
    let mut items: Vec<String> = PAUSE_ITEMS.iter().map(|k| lang.get(k).to_string()).collect();
    if let Some(f) = fallback {
        let name = lang.get(if f == Offer::Heal { "ui.heal" } else { "ui.rupees" });
        items.push(lang.get("ui.fallback_row").replace("{a}", &crate::hud::hl(name)));
    }
    for (k, label) in items.iter().enumerate() {
        let y = pause_y(view, k);
        let c = if k == sel { GOLD } else { SOFT };
        if k == sel {
            let w = main.measure(label) as f32;
            let bw = (w + 48.0).max(160.0);
            crate::hub::focus_frame(list, ctx, Vec2::new(((view.x - bw) * 0.5).round(), y - 3.0), Vec2::new(bw, 20.0));
        }
        let w = text(list, main, label, Vec2::new(view.x * 0.5, y), 1.0, c, Align::Center);
        if k == sel {
            text(list, main, "▶", Vec2::new(((view.x - w) * 0.5 - 14.0).round(), y), 1.0, GOLD, Align::Left);
        }
    }
    let y0 = pause_y(view, items.len()) + 16.0;
    // fusions possibles avec les armes en main (a la place de l'aide des commandes)
    if draw_fusions(list, ctx, vis, lang, world, known, y0) {
        return;
    }
    let help = ["ui.help.move", "ui.help.attack", "ui.help.dash", "ui.help.auto", "ui.help.keys"];
    for (k, h) in help.iter().enumerate() {
        text(list, main, lang.get(h), Vec2::new(view.x * 0.5, y0 + k as f32 * 13.0), 1.0, [0.75, 0.75, 0.85, 0.8], Align::Center);
    }
}

/// Une carte par arme qui peut evoluer : arme + objet partenaire, et l'arme evoluee en dessous
/// (silhouette tant qu'elle n'a jamais ete obtenue). Rose qui pulse : prete pour le prochain coffre ;
/// partenaire manquant : grise ; partenaire deja utilise par une autre fusion : barree.
fn draw_fusions(list: &mut DrawList, ctx: &Ctx, vis: &Vis, lang: &Lang, world: &World, known: &std::collections::BTreeSet<String>, y0: f32) -> bool {
    let c = &world.content;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let white = &vis.white;
    let rt = ctx.real_time as f32;
    let cards: Vec<(u16, u16, u16)> = world
        .weapons
        .iter()
        .flat_map(|w| {
            world.recipes_of(w.id).into_iter().filter_map(move |e| Some((w.id, c.passive_index(&e.passive)?, c.weapon_index(&e.into)?)))
        })
        .collect();
    if cards.is_empty() {
        return false;
    }
    let cx = (ctx.view.x * 0.5).round();
    text(list, main, &crate::hud::hl_fusion(lang.get("ui.fusions_title")), Vec2::new(cx, y0), 1.0, WHITE, Align::Center);
    let (cw, ch, gap) = (52.0, 48.0, 6.0);
    let n = cards.len() as f32;
    let x0 = (cx - (n * cw + (n - 1.0) * gap) * 0.5).round();
    let y = y0 + 16.0;
    for (k, &(id, pid, into)) in cards.iter().enumerate() {
        let p = Vec2::new(x0 + k as f32 * (cw + gap), y);
        let wl = world.weapon_level(id).unwrap_or(0);
        let wmax = c.weapons[id as usize].max_level();
        let pl = world.passive_level(pid);
        let pmax = c.passives[pid as usize].max_level();
        let used = !world.partner_free(pid);
        let ready = !used && wl >= wmax && pl.is_some_and(|l| l >= pmax);
        list.rect(white, p, Vec2::new(cw, ch), with_alpha(BACK, 0.85));
        let border = if ready {
            with_alpha(FUSION_PINK, 0.6 + 0.4 * (rt * 6.0).sin())
        } else if used {
            [0.5, 0.5, 0.55, 0.6]
        } else {
            with_alpha(GOLD, 0.6)
        };
        frame_rect(list, white, p, Vec2::new(cw, ch), border);
        let dimc = [0.45, 0.45, 0.5, 0.7];
        list.frame(atlas.frame(vis.weapons[id as usize].icon, 0.0), p + Vec2::new(14.0, 12.0), &Params::default());
        let pc = if pl.is_some() && !used { [1.0, 1.0, 1.0, 1.0] } else { dimc };
        list.frame(atlas.frame(vis.passives[pid as usize], 0.0), p + Vec2::new(38.0, 12.0), &Params::default().color(pc));
        // signe plus dessine (pas du texte : il est dans la carte)
        let pc2 = if ready { FUSION_PINK } else { SOFT };
        list.rect(white, p + Vec2::new(24.0, 11.0), Vec2::new(5.0, 1.0), pc2);
        list.rect(white, p + Vec2::new(26.0, 9.0), Vec2::new(1.0, 5.0), pc2);
        if used {
            // partenaire deja pris par une autre fusion : barre en travers
            list.rect(white, p + Vec2::new(29.0, 11.0), Vec2::new(18.0, 2.0), [1.0, 0.4, 0.4, 0.9]);
        }
        let ic = if known.contains(&c.weapon_ids[into as usize]) { [1.0, 1.0, 1.0, 1.0] } else { [0.05, 0.05, 0.1, 0.9] };
        let bob = if ready { ((rt * 4.0).sin() * 1.5).round() } else { 0.0 };
        list.frame(atlas.frame(vis.weapons[into as usize].icon, 0.0), p + Vec2::new(26.0, 36.0 + bob), &Params::default().color(ic));
        if ready {
            list.set(Space::Ui, Blend::Add);
            for s in 0..4 {
                let a = rt * 2.0 + s as f32 * std::f32::consts::FRAC_PI_2;
                spark(list, white, p + Vec2::new(26.0 + a.cos() * 12.0, 36.0 + a.sin() * 7.0), 2.0, [1.0, 0.8, 1.0, 0.9]);
            }
            list.set(Space::Ui, Blend::Alpha);
        }
        list.probe_block(p, Vec2::new(cw, ch), "carte de fusion");
    }
    true
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

pub fn input_pause(nav: &NavIn, view: Vec2, sel: &mut usize, fallback: bool) -> (Option<usize>, bool) {
    input_menu(nav, view, PAUSE_ITEMS.len() + fallback as usize, sel, |k| pause_y(view, k), 260.0)
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
    /// Sauvegarde : export / import du profil (0.6.4, avant dans le Profil).
    Save,
    Back,
}

#[cfg(not(target_arch = "wasm32"))]
pub const OPTIONS: [Opt; 12] = [
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
    Opt::Save,
    Opt::Back,
];

/// Navigateur : pas de mises a jour (le site sert la derniere version).
#[cfg(target_arch = "wasm32")]
pub const OPTIONS: [Opt; 11] = [
    Opt::Language,
    Opt::Video,
    Opt::Aspect,
    Opt::Filter,
    Opt::Master,
    Opt::Music,
    Opt::Sfx,
    Opt::Fullscreen,
    Opt::Controls,
    Opt::Save,
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
            crate::hub::focus_frame(list, ctx, Vec2::new(x0 - 4.0, y - 3.0), Vec2::new(w + 8.0, 20.0));
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
            Opt::Save => "ui.tab.save",
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
            Opt::Controls | Opt::Save => {
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
    text(list, main, lang.get("ui.gameover"), Vec2::new(view.x * 0.5, (view.y * 0.16).round()), crate::hud::title_scale(main, lang.get("ui.gameover"), view.x, 3.0), rgb(255, 90, 90), Align::Center);
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
