//! Trophees (0.6.4) : les exploits du joueur, en trois rangs (bronze, argent, or). Tout est calcule
//! d'apres le profil, donc les parties deja jouees comptent.
use crate::hub::Row;
use crate::lang::Lang;
use crate::meta::Meta;
use crate::profile::Profile;
use engine::assets::Atlas;
use engine::draw::{rgb, Color};
use sim::defs::Content;

pub const TIER_KEYS: [&str; 3] = ["ui.trophy.bronze", "ui.trophy.silver", "ui.trophy.gold"];
pub const TIER_COLORS: [Color; 3] = [
    rgb(214, 140, 76),
    rgb(200, 206, 226),
    crate::hud::GOLD,
];

/// Famille de trophees : une valeur du profil et jusqu'a trois paliers (0 : pas de trophee a ce rang).
pub struct Family {
    pub id: &'static str,
    pub icon: &'static str,
    pub goals: [u64; 3],
    pub value: u64,
    /// Valeur en secondes, affichee en heures.
    pub hours: bool,
}

impl Family {
    fn new(id: &'static str, icon: &'static str, goals: [u64; 3], value: u64) -> Family {
        Family { id, icon, goals, value, hours: false }
    }
}

pub fn families(content: &Content, meta: &Meta, p: &Profile) -> Vec<Family> {
    let t = &p.totals;
    let stages = content.maps.values().filter(|m| m.order > 0).count() as u64;
    let won = p.stages.values().filter(|r| r.wins > 0).count() as u64;
    let seen = |id: &str| p.bestiary.get(id).copied().unwrap_or(0) > 0;
    let bosses: Vec<&String> = content.enemy_ids.iter().zip(&content.enemies).filter(|(_, e)| e.boss && e.bestiary).map(|(id, _)| id).collect();
    let chars = content.characters.values().filter(|d| crate::chars::unlocked(content, d, p)).count() as u64;
    let fusions = content.weapons.iter().filter(|w| w.evolved).count() as u64;
    let bestiary = crate::records::bestiary_order(content, p).len() as u64;
    let best_score = p.scores.values().flatten().map(|r| r.score).max().unwrap_or(0);
    let best_diff = p.stages.values().filter(|r| r.wins > 0).map(|r| r.best_difficulty.max(0) as u64).max().unwrap_or(0);
    let top_diff = meta.difficulties.len().saturating_sub(1) as u64;
    let n = |v: usize| v as u64;
    vec![
        Family::new("wins", "icon_crystal", [1, 10, 50], n(t.wins as usize)),
        Family::new("stages", "icon_pendant_courage", [1, 5, stages], won),
        Family::new("bosses", "icon_big_key", [3, 12, n(bosses.len())], n(bosses.iter().filter(|b| seen(b)).count())),
        Family::new("ganon", "icon_silver_bow", [0, 0, 1], seen("ganon") as u64),
        Family::new("master_sword", "icon_master_sword", [0, 0, 1], p.evolutions.contains("master_sword") as u64),
        Family::new("difficulty", "icon_medallion_quake", [1, top_diff.div_ceil(2), top_diff], best_diff),
        Family::new("score", "icon_rupees", [25_000, 100_000, 300_000], best_score),
        Family::new("kills", "icon_sword", [1_000, 25_000, 250_000], t.kills),
        Family::new("elites", "icon_sage_shadow", [5, 50, 250], n(t.elites as usize)),
        Family::new("level", "icon_blue_potion", [20, 40, 70], n(t.best_level as usize)),
        Family::new("characters", "icon_moon_pearl", [10, 50, content.characters.len() as u64], chars),
        Family::new("fusions", "icon_golden_sword", [3, 10, fusions], n(p.evolutions.len())),
        Family::new("bestiary", "icon_book", [20, 40, bestiary], n(crate::records::bestiary_known(content, p).len())),
        Family::new("figurines", "icon_vaati_jar", [5, 25, bestiary], n(p.figurines.len())),
        Family::new("relics", "icon_pendant_power", [3, 8, n(meta.relics.len())], n(p.relics.len())),
        // tenues decrites dans meta.ron (pas les centaines de couleurs ajoutees en 0.7.1)
        Family::new("wardrobe", "icon_green_mail", [1, 6, n(meta.wardrobe.iter().filter(|w| !w.auto).count())], n(p.wardrobe.len())),
        Family::new("titles", "icon_fairy", [1, 2, n(meta.titles.len())], n(p.title as usize)),
        Family::new("banked", "icon_rupees", [5_000, 50_000, 500_000], t.banked),
        Family::new("chests", "icon_small_key", [25, 250, 1_000], n(t.chests as usize)),
        Family::new("runs", "icon_boots", [10, 100, 500], n(t.runs as usize)),
        Family { hours: true, ..Family::new("time", "icon_ocarina_of_time", [3_600, 36_000, 180_000], t.time as u64) },
    ]
}

/// Trophees obtenus et total, par rang (bronze, argent, or).
pub fn count(fams: &[Family]) -> ([usize; 3], usize) {
    let mut got = [0; 3];
    let mut total = 0;
    for f in fams {
        for (r, &g) in f.goals.iter().enumerate() {
            if g > 0 {
                total += 1;
                got[r] += (f.value >= g) as usize;
            }
        }
    }
    (got, total)
}

fn amount(f: &Family, v: u64) -> String {
    if f.hours {
        format!("{} h", v / 3600)
    } else {
        v.to_string()
    }
}

pub fn rows(content: &Content, meta: &Meta, atlas: &Atlas, p: &Profile, lang: &Lang) -> (Vec<Row>, Vec<String>) {
    let fams = families(content, meta, p);
    let (got, total) = count(&fams);
    let summary = lang
        .get("ui.trophies_count")
        .replace("{a}", &got.iter().sum::<usize>().to_string())
        .replace("{b}", &total.to_string())
        .replace("{g}", &got[2].to_string())
        .replace("{s}", &got[1].to_string())
        .replace("{c}", &got[0].to_string());
    let mut rows = Vec::new();
    let mut details = Vec::new();
    for f in &fams {
        for (r, &goal) in f.goals.iter().enumerate() {
            if goal == 0 {
                continue;
            }
            let ok = f.value >= goal;
            rows.push(Row {
                icon: atlas.find(f.icon, "0").or_else(|| atlas.find("icon_crystal", "0")),
                icon_dim: !ok,
                label: format!("{} - {}", lang.get(&format!("trophy.{}", f.id)), lang.get(TIER_KEYS[r])),
                label_color: ok.then_some(TIER_COLORS[r]),
                value: if ok { String::new() } else { format!("{} / {}", amount(f, f.value.min(goal)), amount(f, goal)) },
                value_color: Some(crate::hub::DIMMED),
                dim: !ok,
                mark: ok,
            });
            let desc = lang.get(&format!("trophy.{}.desc", f.id)).replace("{n}", &amount(f, goal));
            details.push(format!("{desc}\n{summary}"));
        }
    }
    (rows, details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Profil neuf : aucun trophee ; tout gagne : tous. Chaque famille a un texte et une icone.
    #[test]
    fn trophies_follow_the_profile() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = crate::content::load_content(&root);
        let meta = Meta::load(&root);
        let p = Profile::new();
        let fams = families(&content, &meta, &p);
        let (got, total) = count(&fams);
        assert_eq!(got, [0, 0, 0]);
        assert!(total >= 50, "{total} trophees");
        for f in &fams {
            assert!(f.goals[0] <= f.goals[1] || f.goals[1] == 0, "{} : paliers dans le desordre", f.id);
            assert!(f.goals[1] <= f.goals[2], "{} : paliers dans le desordre", f.id);
        }
        let mut big = Profile::new();
        big.totals.kills = 1_000_000;
        let k = families(&content, &meta, &big).into_iter().find(|f| f.id == "kills").unwrap();
        assert!(k.goals.iter().all(|&g| k.value >= g));
    }
}
