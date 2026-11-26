//! Recompenses des rencontres : un champion errant vaincu laisse le choix entre trois cartes.
//! - benediction : un bonus de statistique pour le reste de la partie ;
//! - amelioration speciale d'une arme possedee (elle suit l'arme quand elle evolue) ;
//! - relique de rencontre : son bonus s'applique tout de suite et elle rejoint la collection
//!   si la partie est gagnee.
use crate::lang::Lang;
use crate::menus::{stat_text, OfferInfo};
use crate::meta::Meta;
use crate::profile::Profile;
use crate::visuals::Vis;
use engine::assets::Atlas;
use sim::defs::{Stat, WeaponKind};
use sim::rng::Rng;
use sim::upgrades::WeaponBoost;
use sim::World;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reward {
    Blessing(Stat, f32),
    Boost { slot: usize, boost: WeaponBoost },
    /// Indice dans `meta.relics`.
    Relic(usize),
}

/// Benedictions possibles (un peu plus fortes qu'un niveau d'objet).
pub const BLESSINGS: [(Stat, f32); 11] = [
    (Stat::Might, 0.12),
    (Stat::Area, 0.12),
    (Stat::Cooldown, -0.08),
    (Stat::MaxHp, 20.0),
    (Stat::MoveSpeed, 0.1),
    (Stat::Regen, 0.25),
    (Stat::Luck, 0.15),
    (Stat::Armor, 1.0),
    (Stat::Duration, 0.12),
    (Stat::Magnet, 0.35),
    (Stat::Growth, 0.12),
];

/// Ameliorations speciales : degats, zone, recharge ; un projectile de plus pour les armes qui tirent.
fn roll_boost(world: &World, slot: usize, rng: &mut Rng) -> WeaponBoost {
    let kind = world.content.weapons[world.weapons[slot].id as usize].kind;
    let shoots = matches!(kind, WeaponKind::Projectile | WeaponKind::Boomerang | WeaponKind::Lob | WeaponKind::Orbit | WeaponKind::Pillars | WeaponKind::Bees | WeaponKind::Tornado);
    let n = if shoots { 4 } else { 3 };
    match rng.below(n) {
        0 => WeaponBoost { damage: 0.3, ..Default::default() },
        1 => WeaponBoost { area: 0.2, ..Default::default() },
        2 => WeaponBoost { cooldown: 0.15, ..Default::default() },
        _ => WeaponBoost { amount: 1, ..Default::default() },
    }
}

/// Trois cartes : une amelioration d'arme, une relique de rencontre pas encore trouvee, des benedictions.
pub fn roll(world: &World, meta: &Meta, profile: &Profile, taken: &[String], rng: &mut Rng) -> Vec<Reward> {
    let mut out = Vec::new();
    if !world.weapons.is_empty() {
        let slot = rng.below(world.weapons.len() as u32) as usize;
        out.push(Reward::Boost { slot, boost: roll_boost(world, slot, rng) });
    }
    let free: Vec<usize> = meta
        .relics
        .iter()
        .enumerate()
        .filter(|(_, r)| r.encounter && !profile.relics.contains(&r.id) && !taken.contains(&r.id))
        .map(|(k, _)| k)
        .collect();
    if !free.is_empty() {
        out.push(Reward::Relic(free[rng.below(free.len() as u32) as usize]));
    }
    let mut pool: Vec<(Stat, f32)> = BLESSINGS.to_vec();
    while out.len() < 3 && !pool.is_empty() {
        let (s, v) = pool.swap_remove(rng.below(pool.len() as u32) as usize);
        out.push(Reward::Blessing(s, v));
    }
    out
}

/// Applique la recompense choisie ; renvoie l'identifiant de la relique gagnee le cas echeant.
pub fn apply(world: &mut World, meta: &Meta, r: Reward) -> Option<String> {
    match r {
        Reward::Blessing(s, v) => {
            world.add_run_stats(&[(s, v)]);
            None
        }
        Reward::Boost { slot, boost } => {
            world.boost_weapon(slot, boost);
            None
        }
        Reward::Relic(k) => {
            let rel = &meta.relics[k];
            world.add_run_stats(&rel.stats);
            world.mods.rerolls += rel.rerolls;
            world.mods.skips += rel.skips;
            Some(rel.id.clone())
        }
    }
}

fn boost_text(lang: &Lang, b: &WeaponBoost) -> String {
    let mut parts = Vec::new();
    if b.damage != 0.0 {
        parts.push(format!("{} {:+}%", lang.get("stat.damage"), (b.damage * 100.0).round() as i32));
    }
    if b.amount != 0 {
        parts.push(format!("{} +{}", lang.get("stat.amount"), b.amount));
    }
    if b.area != 0.0 {
        parts.push(format!("{} {:+}%", lang.get("stat.area"), (b.area * 100.0).round() as i32));
    }
    if b.cooldown != 0.0 {
        parts.push(format!("{} {:+}%", lang.get("stat.cooldown"), (-b.cooldown * 100.0).round() as i32));
    }
    parts.join(", ")
}

/// Icone d'une benediction : l'objet passif qui donne cette statistique, sinon un coeur.
fn blessing_icon(world: &World, vis: &Vis, s: Stat) -> engine::assets::AnimId {
    world
        .content
        .passives
        .iter()
        .position(|p| p.levels.first().is_some_and(|l| l.iter().any(|&(st, _)| st == s)))
        .map(|k| vis.passives[k])
        .unwrap_or(vis.icon_heart)
}

pub fn info(world: &World, vis: &Vis, atlas: &Atlas, lang: &Lang, meta: &Meta, r: Reward) -> OfferInfo {
    let blank = |icon, name: String, tag: &str, desc: String| OfferInfo {
        icon,
        name,
        tag: tag.to_string(),
        is_new: false,
        desc,
        level: 0,
        max: 0,
        evo: Vec::new(),
        evo_label: String::new(),
        keep: false,
    };
    match r {
        Reward::Blessing(s, v) => blank(blessing_icon(world, vis, s), stat_text(lang, s, v), lang.get("ui.reward.blessing"), lang.get("ui.reward.blessing_desc").to_string()),
        Reward::Boost { slot, boost } => {
            let id = world.weapons[slot].id as usize;
            let wid = &world.content.weapon_ids[id];
            let mut o = blank(
                vis.weapons[id].icon,
                lang.get(&format!("weapon.{wid}.name")).to_string(),
                lang.get("ui.reward.boost"),
                format!("{}. {}", boost_text(lang, &boost), lang.get("ui.reward.boost_desc")),
            );
            o.is_new = true;
            o
        }
        Reward::Relic(k) => {
            let rel = &meta.relics[k];
            let fx = crate::meta::relic_fx(lang, rel);
            let mut o = blank(
                atlas.find(&rel.icon, "0").unwrap_or(vis.icon_heart),
                lang.get(&format!("relic.{}.name", rel.id)).to_string(),
                lang.get("ui.reward.relic"),
                format!("{}. {}", fx.join(", "), lang.get("ui.reward.relic_desc")),
            );
            o.keep = true;
            o
        }
    }
}
