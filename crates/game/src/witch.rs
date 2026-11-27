//! Sorciere (0.6.3) : trois potions tirees au hasard, chacune avec un ou deux bonus et un malus
//! (valeurs tirees dans des fourchettes), pour toute la partie. Le prix en rubis verts depend de la
//! force du tirage, du stage, de la difficulte et de l'heure : tot dans la partie, tout est plus cher.
use crate::lang::Lang;
use crate::menus::{stat_text, OfferInfo};
use crate::visuals::Vis;
use sim::defs::Stat;
use sim::rng::Rng;

#[derive(Clone, Debug, PartialEq)]
pub struct Potion {
    /// 0 rouge (attaque), 1 verte (vie), 2 bleue (adresse) : couleur du premier bonus.
    pub color: usize,
    pub bonus: Vec<(Stat, f32)>,
    pub malus: (Stat, f32),
    pub price: u32,
}

/// Bonus possibles : statistique, fourchette (min, max) et couleur de la potion.
const BONUS: [(Stat, f32, f32, usize); 11] = [
    (Stat::Might, 0.08, 0.25, 0),
    (Stat::Cooldown, -0.05, -0.14, 0),
    (Stat::Area, 0.08, 0.22, 0),
    (Stat::Duration, 0.10, 0.25, 0),
    (Stat::MaxHp, 10.0, 30.0, 1),
    (Stat::Regen, 0.10, 0.40, 1),
    (Stat::Armor, 1.0, 2.0, 1),
    (Stat::MoveSpeed, 0.05, 0.14, 2),
    (Stat::Magnet, 0.20, 0.60, 2),
    (Stat::Luck, 0.10, 0.30, 2),
    (Stat::Growth, 0.08, 0.22, 2),
];

/// Malus possibles : du plus doux (min) au plus dur (max).
const MALUS: [(Stat, f32, f32); 9] = [
    (Stat::Might, -0.04, -0.12),
    (Stat::Cooldown, 0.03, 0.08),
    (Stat::Area, -0.05, -0.12),
    (Stat::MaxHp, -10.0, -20.0),
    (Stat::Armor, -1.0, -1.0),
    (Stat::MoveSpeed, -0.04, -0.10),
    (Stat::Magnet, -0.15, -0.30),
    (Stat::Luck, -0.08, -0.20),
    (Stat::Growth, -0.05, -0.12),
];

/// Prix de base (rubis verts, qualite 1, a 5 min, stage 1, difficulte normale). Mesure en demo : ~80
/// rubis a 2:00, ~150 a 3:00, ~300 a 4:30 ; une potion moyenne (qualite ~1,2) coute ~150 a 5:00 et
/// ~240 a 2:00, une tres bonne (qualite 3) ~375 a 5:00 et ~600 a 2:00.
const BASE_PRICE: f32 = 125.0;

/// Arrondi des statistiques entieres (coeurs par demi-coeur, armure a l'unite).
fn snap(s: Stat, v: f32) -> f32 {
    match s {
        Stat::MaxHp => (v / 5.0).round() * 5.0,
        Stat::Armor => v.round(),
        _ => (v * 100.0).round() / 100.0,
    }
}

/// Prix : `quality` (0,1 a 3) x racine de la difficulte du stage (`power`) x racine du gain de la
/// difficulte choisie (`rupees`) x heure (x1,6 a 2:00, x1 a partir de 5:00), arrondi a 5.
pub fn price(quality: f32, stage_power: f32, difficulty: f32, time: f32) -> u32 {
    let early = 1.6 - 0.6 * ((time - 120.0) / 180.0).clamp(0.0, 1.0);
    let p = BASE_PRICE * quality.max(0.2) * stage_power.max(1.0).sqrt() * difficulty.max(0.1).sqrt() * early;
    ((p / 5.0).round() * 5.0).max(5.0) as u32
}

/// Trois potions differentes.
pub fn roll(rng: &mut Rng, stage_power: f32, difficulty: f32, time: f32) -> Vec<Potion> {
    (0..3)
        .map(|_| {
            let n = if rng.chance(0.4) { 2 } else { 1 };
            let mut pool: Vec<usize> = (0..BONUS.len()).collect();
            let mut bonus = Vec::new();
            let mut quality = 0.0;
            for _ in 0..n {
                let (s, a, b, _) = BONUS[pool.swap_remove(rng.below(pool.len() as u32) as usize)];
                let r = rng.range(0.0, 1.0);
                bonus.push((s, snap(s, a + (b - a) * r)));
                quality += 0.5 + r;
            }
            let bad: Vec<usize> = (0..MALUS.len()).filter(|&k| !bonus.iter().any(|b| b.0 == MALUS[k].0)).collect();
            let (s, a, b) = MALUS[bad[rng.below(bad.len() as u32) as usize]];
            let m = rng.range(0.0, 1.0);
            quality -= 0.4 * m;
            let color = BONUS.iter().find(|x| x.0 == bonus[0].0).map_or(0, |x| x.3);
            Potion { color, bonus, malus: (s, snap(s, a + (b - a) * m)), price: price(quality, stage_power, difficulty, time) }
        })
        .collect()
}

/// Effets de la potion, a ajouter aux statistiques de la partie.
pub fn effects(p: &Potion) -> Vec<(Stat, f32)> {
    p.bonus.iter().copied().chain(std::iter::once(p.malus)).collect()
}

/// Carte de la potion (achat chez la sorciere).
pub fn info(vis: &Vis, lang: &Lang, p: &Potion, rupees: u32) -> OfferInfo {
    let bonus: Vec<String> = p.bonus.iter().map(|&(s, v)| crate::hud::hl_new(&stat_text(lang, s, v))).collect();
    let desc = format!("{}  {} {}", bonus.join(", "), lang.get("ui.witch.malus"), stat_text(lang, p.malus.0, p.malus.1));
    let tag = if rupees >= p.price { lang.get("ui.witch.price") } else { lang.get("ui.witch.too_poor") };
    OfferInfo {
        icon: vis.potions[p.color],
        name: lang.get(&format!("ui.witch.potion{}", p.color)).to_string(),
        tag: tag.replace("{n}", &p.price.to_string()),
        is_new: rupees >= p.price,
        desc,
        level: 0,
        max: 0,
        evo: Vec::new(),
        evo_label: String::new(),
        keep: false,
        fusion: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potions_are_dearer_early_and_on_hard_stages() {
        assert!(price(1.0, 1.0, 1.0, 120.0) > price(1.0, 1.0, 1.0, 300.0));
        assert!(price(1.0, 2.35, 1.0, 300.0) > price(1.0, 1.0, 1.0, 300.0));
        assert!(price(2.5, 1.0, 1.0, 300.0) > price(1.0, 1.0, 1.0, 300.0));
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            for p in roll(&mut rng, 1.5, 1.0, 200.0) {
                assert!((1..=2).contains(&p.bonus.len()));
                assert!(p.bonus.iter().all(|b| b.0 != p.malus.0), "bonus et malus sur la meme statistique");
                // un bonus aide, un malus gene (le temps de recharge baisse quand il aide)
                let good = |s: Stat, v: f32| if s == Stat::Cooldown { v < 0.0 } else { v > 0.0 };
                assert!(p.bonus.iter().all(|&(s, v)| good(s, v)) && !good(p.malus.0, p.malus.1), "{p:?}");
                assert!(p.price >= 5);
            }
        }
    }
}
