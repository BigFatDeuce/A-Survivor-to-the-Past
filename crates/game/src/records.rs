//! Ecran Profil : bestiaire, armurerie, statistiques et sauvegarde (export / import du profil).
use crate::hub::{Row, DIMMED, GREEN};
use crate::lang::Lang;
use crate::profile::Profile;
use crate::visuals::Vis;
use sim::defs::Content;
use sim::horde::FACE_DOWN;

pub const TABS: [&str; 4] = ["ui.tab.bestiary", "ui.tab.armory", "ui.tab.stats", "ui.tab.save"];

/// Lignes de l'onglet Sauvegarde.
pub const SAVE_EXPORT: usize = 0;
pub const SAVE_IMPORT: usize = 1;

fn fmt_hours(s: f64) -> String {
    let m = (s / 60.0) as u64;
    format!("{}:{:02}", m / 60, m % 60)
}

pub fn rows(content: &Content, vis: &Vis, p: &Profile, lang: &Lang, tab: usize, armed: bool) -> (Vec<Row>, Vec<String>) {
    let mut rows = Vec::new();
    let mut details = Vec::new();
    match tab {
        0 => {
            // ennemis ordinaires d'abord, puis les boss
            let mut order: Vec<usize> = (0..content.enemies.len()).collect();
            order.sort_by_key(|&k| content.enemies[k].boss);
            for k in order {
                let id = &content.enemy_ids[k];
                let n = p.bestiary.get(id).copied().unwrap_or(0);
                let e = &content.enemies[k];
                rows.push(Row {
                    icon: Some(vis.enemies[k].pick(FACE_DOWN, 0.0, false).0),
                    icon_dim: n == 0,
                    label: if n > 0 { lang.get(&format!("enemy.{id}")).to_string() } else { "???".into() },
                    value: n.to_string(),
                    value_color: Some(if n > 0 { GREEN } else { DIMMED }),
                    dim: n == 0,
                    mark: e.boss && n > 0,
                });
                details.push(if n > 0 {
                    let kind = if e.boss { lang.get("ui.boss") } else { "" };
                    format!("{kind} {}", lang.get("ui.defeated_n").replace("{n}", &n.to_string())).trim().to_string()
                } else {
                    lang.get("ui.unknown_enemy").to_string()
                });
            }
        }
        1 => {
            for (k, id) in content.weapon_ids.iter().enumerate() {
                let w = &content.weapons[k];
                let n = p.armory.get(id).copied().unwrap_or(0);
                let known = n > 0 || p.evolutions.contains(id);
                rows.push(Row {
                    icon: Some(vis.weapons[k].icon),
                    icon_dim: !known,
                    label: if known { lang.get(&format!("weapon.{id}.name")).to_string() } else { "???".into() },
                    value: if w.evolved && known { lang.get("ui.evolution").to_string() } else { n.to_string() },
                    value_color: Some(if !known {
                        DIMMED
                    } else if w.evolved {
                        GREEN
                    } else {
                        crate::hud::GOLD
                    }),
                    dim: !known,
                    mark: w.evolved && known,
                });
                details.push(if known {
                    let mut d = lang.get(&format!("weapon.{id}.desc")).to_string();
                    if let Some(e) = &w.evolution {
                        if let Some(need) = content.passive_index(&e.passive) {
                            let pid = &content.passive_ids[need as usize];
                            let into = lang.get(&format!("weapon.{}.name", e.into)).to_string();
                            let into = if p.evolutions.contains(&e.into) { into } else { "???".into() };
                            d = format!("{d} {}", lang.get("ui.evolves_into").replace("{a}", lang.get(&format!("passive.{pid}.name"))).replace("{b}", &into));
                        }
                    }
                    d
                } else {
                    lang.get(if w.evolved { "ui.unknown_evolution" } else { "ui.unknown_item" }).to_string()
                });
            }
            for (k, id) in content.passive_ids.iter().enumerate() {
                let n = p.armory.get(id).copied().unwrap_or(0);
                rows.push(Row {
                    icon: Some(vis.passives[k]),
                    icon_dim: n == 0,
                    label: if n > 0 { lang.get(&format!("passive.{id}.name")).to_string() } else { "???".into() },
                    value: n.to_string(),
                    value_color: Some(if n > 0 { crate::hud::GOLD } else { DIMMED }),
                    dim: n == 0,
                    mark: false,
                });
                details.push(if n > 0 { lang.get(&format!("passive.{id}.desc")).to_string() } else { lang.get("ui.unknown_item").to_string() });
            }
        }
        2 => {
            let t = &p.totals;
            let won = p.stages.values().filter(|r| r.wins > 0).count();
            let stats: [(&str, String); 12] = [
                ("ui.st.runs", t.runs.to_string()),
                ("ui.st.wins", t.wins.to_string()),
                ("ui.st.stages", format!("{won} / {}", content.maps.values().filter(|m| m.order > 0).count())),
                ("ui.st.deaths", t.deaths.to_string()),
                ("ui.st.time", fmt_hours(t.time)),
                ("ui.st.kills", t.kills.to_string()),
                ("ui.st.elites", t.elites.to_string()),
                ("ui.st.bosses", t.bosses.to_string()),
                ("ui.st.chests", t.chests.to_string()),
                ("ui.st.best_level", t.best_level.to_string()),
                ("ui.st.rupees", t.rupees.to_string()),
                ("ui.st.banked", t.banked.to_string()),
            ];
            for (key, v) in stats {
                rows.push(Row { label: lang.get(key).to_string(), value: v, value_color: Some(crate::hud::GOLD), ..Default::default() });
                details.push(String::new());
            }
        }
        _ => {
            rows.push(Row { label: lang.get("ui.export").to_string(), ..Default::default() });
            details.push(lang.get("ui.export_desc").to_string());
            rows.push(Row {
                label: lang.get("ui.import").to_string(),
                value: if armed { lang.get("ui.confirm_again").to_string() } else { String::new() },
                value_color: Some(crate::hub::RED),
                ..Default::default()
            });
            details.push(lang.get("ui.import_desc").to_string());
        }
    }
    (rows, details)
}
