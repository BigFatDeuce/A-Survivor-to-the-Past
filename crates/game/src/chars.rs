//! Personnages : choix avant la partie, conditions de deblocage (d'apres le profil).
use crate::hud::{frame_rect, text, wrap, BACK, GOLD};
use crate::lang::Lang;
use crate::menus::{self, NavIn};
use crate::profile::Profile;
use crate::stages::select_layout;
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

/// Avancement vers le deblocage : (fait, objectif) ; `None` si deja disponible d'office.
pub fn progress(def: &CharacterDef, p: &Profile) -> Option<(u64, u64)> {
    let t = &p.totals;
    Some(match def.unlock.as_ref()? {
        UnlockDef::Win(s) => (p.won(s) as u64, 1),
        UnlockDef::Kills(n) => (t.kills.min(*n), *n),
        UnlockDef::Bosses(n) => (t.bosses.min(*n) as u64, *n as u64),
        UnlockDef::Level(n) => (t.best_level.min(*n) as u64, *n as u64),
        UnlockDef::Slain(e) => (p.bestiary.get(e).copied().unwrap_or(0).min(1), 1),
    })
}

/// Couleur choisie (indice dans `skins`).
pub fn skin(def: &CharacterDef, p: &Profile, id: &str) -> usize {
    p.skins.get(id).copied().unwrap_or(0).min(def.skins.len().saturating_sub(1))
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

pub fn unlocked(def: &CharacterDef, p: &Profile) -> bool {
    progress(def, p).is_none_or(|(a, b)| a >= b)
}

pub fn name(lang: &Lang, id: &str) -> String {
    lang.get(&format!("char.{id}.name")).to_string()
}

/// Phrase de deblocage (« Gagner la Plaine d'Hyrule »...).
pub fn hint(lang: &Lang, def: &CharacterDef) -> String {
    match &def.unlock {
        None => String::new(),
        Some(UnlockDef::Win(s)) => lang.get("ui.unlock_win").replace("{s}", &crate::stages::stage_name(lang, s)),
        Some(UnlockDef::Kills(n)) => lang.get("ui.unlock_kills").replace("{n}", &n.to_string()),
        Some(UnlockDef::Bosses(n)) => lang.get("ui.unlock_bosses").replace("{n}", &n.to_string()),
        Some(UnlockDef::Level(n)) => lang.get("ui.unlock_level").replace("{n}", &n.to_string()),
        Some(UnlockDef::Slain(e)) => lang.get("ui.unlock_slain").replace("{s}", lang.get(&format!("enemy.{e}"))),
    }
}

/// Personnages qui viennent d'etre debloques (entre deux etats du profil).
pub fn newly_unlocked(content: &Content, before: &Profile, after: &Profile) -> Vec<String> {
    roster(content)
        .into_iter()
        .filter(|id| {
            let d = &content.characters[id];
            !unlocked(d, before) && unlocked(d, after)
        })
        .collect()
}

pub fn input_select(nav: &NavIn, view: Vec2, n: usize, sel: &mut usize) -> (Option<usize>, bool) {
    menus::input_rects(nav, &select_layout(view, n, *sel).items, sel)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_select(list: &mut DrawList, ctx: &Ctx, lang: &Lang, content: &Content, profile: &Profile, ids: &[String], sel: usize, t: f32) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 0.55]);
    let l = select_layout(view, ids.len(), sel);
    text(list, main, lang.get("ui.characters"), Vec2::new(view.x * 0.5, l.title_y), 2.0, WHITE, Align::Center);
    crate::stages::draw_scroll_marks(list, white, &l, ids.len(), t);
    for (k, &(p, s)) in l.items.iter().enumerate() {
        if !l.visible(k) {
            continue;
        }
        let back = k == ids.len();
        list.rect(white, p, s, with_alpha(BACK, if k == sel { 0.95 } else { 0.8 }));
        if k == sel {
            frame_rect(list, white, p, s, with_alpha(GOLD, 0.9));
        }
        list.probe_panel(p, s, "ligne de personnage");
        let ty = p.y + 7.0;
        if back {
            text(list, main, lang.get("ui.back"), Vec2::new(p.x + 22.0, ty), 1.0, if k == sel { GOLD } else { SOFT }, Align::Left);
            continue;
        }
        let def = &content.characters[&ids[k]];
        let open = unlocked(def, profile);
        let num = format!("{}", k + 1);
        text(list, main, &num, Vec2::new(p.x + 8.0, ty), 1.0, if open { GOLD } else { DIMMED }, Align::Left);
        let label = if open { name(lang, &ids[k]) } else { lang.get("ui.locked").to_string() };
        let c = if !open {
            DIMMED
        } else if k == sel {
            GOLD
        } else {
            SOFT
        };
        let room = s.x - 22.0 - 8.0;
        let lines = wrap(main, &label, room, 1.0);
        let shown = if lines.len() > 1 { format!("{}…", lines[0]) } else { label };
        text(list, main, &shown, Vec2::new(p.x + 22.0, ty), 1.0, c, Align::Left);
    }
    // panneau du personnage choisi
    let (pp, ps) = l.panel;
    list.rect(white, pp, ps, with_alpha(BACK, 0.92));
    frame_rect(list, white, pp + Vec2::splat(2.0), ps - Vec2::splat(4.0), with_alpha(GOLD, 0.7));
    list.probe_panel(pp, ps, "panneau du personnage");
    let k = sel.min(ids.len().saturating_sub(1));
    let Some(id) = ids.get(k) else { return };
    let def = &content.characters[id];
    let open = unlocked(def, profile);
    let x = pp.x + 12.0;
    let w = ps.x - 24.0;
    let mut y = pp.y + 10.0;
    let title = if open { name(lang, id) } else { lang.get("ui.locked").to_string() };
    let big = main.measure(&title) as f32 * 2.0 <= w;
    text(list, main, &title, Vec2::new(x, y), if big { 2.0 } else { 1.0 }, if open { GOLD } else { DIMMED }, Align::Left);
    y += if big { 30.0 } else { 16.0 };
    // portrait : le personnage marche et tourne sur lui-meme
    let sc: f32 = if ps.y > 300.0 { 3.0 } else { 2.0 };
    let bh = (30.0 * sc + 8.0).round();
    let bw = w.min(bh * 2.0).round();
    let bp = Vec2::new((x + (w - bw) * 0.5).round(), y);
    list.rect(white, bp, Vec2::new(bw, bh), [0.0, 0.0, 0.0, 0.35]);
    frame_rect(list, white, bp, Vec2::new(bw, bh), with_alpha(GOLD, 0.35));
    list.probe_block(bp, Vec2::new(bw, bh), "portrait");
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
        list.set(Space::Ui, Blend::Alpha);
        list.frame(f, Vec2::new(bp.x + bw * 0.5, bp.y + bh - 4.0).round(), &prm);
    }
    y += bh + 8.0;
    let bottom = pp.y + ps.y - 10.0;
    if !open {
        for line in wrap(main, &hint(lang, def), w, 1.0) {
            text(list, main, &line, Vec2::new(x, y), 1.0, SOFT, Align::Left);
            y += 13.0;
        }
        if let Some((a, b)) = progress(def, profile).filter(|p| p.1 > 1) {
            let s = format!("{a} / {b}");
            text(list, main, &s, Vec2::new(x, y), 1.0, GOLD, Align::Left);
        }
        return;
    }
    for line in wrap(main, lang.get(&format!("char.{id}.desc")), w, 1.0) {
        if y + 12.0 > bottom {
            break;
        }
        text(list, main, &line, Vec2::new(x, y), 1.0, SOFT, Align::Left);
        y += 13.0;
    }
    y += 4.0;
    let mut rows: Vec<(String, String)> = vec![
        (lang.get("ui.char_health").to_string(), format!("{}", (def.max_hp / 10.0).round() as u32)),
        (lang.get("ui.char_speed").to_string(), format!("{}", def.speed.round() as u32)),
    ];
    if !def.skins.is_empty() {
        let k = skin(def, profile, id);
        let c = lang.get(&format!("skin.{}", def.skins[k].1));
        rows.push((lang.get("ui.char_color").to_string(), format!("◀ {c} ▶")));
    }
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
        y += 16.0;
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
        assert!(super::unlocked(&c.characters["link"], &p));
        assert!(!super::unlocked(&c.characters["link_minish"], &p));
        p.stages.entry("hyrule_field".into()).or_default().wins = 1;
        p.totals.kills = 5000;
        let new = super::newly_unlocked(&c, &before, &p);
        assert!(new.contains(&"link_minish".to_string()) && new.contains(&"link_capless".to_string()), "{new:?}");
        // Shadow Link : il faut avoir vaincu Ganon
        assert!(!super::unlocked(&c.characters["shadow_link"], &p));
        p.bestiary.insert("ganon".into(), 1);
        assert!(super::unlocked(&c.characters["shadow_link"], &p));
        // couleurs : le choix du profil remplace le sprite
        let mut c2 = c.clone();
        p.skins.insert("link_fsa".into(), 2);
        super::apply_skins(&mut c2, &p);
        assert_eq!(c2.characters["link_fsa"].sprite, "link_fsa_blue");
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
}
