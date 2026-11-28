//! Onglets du Recueil (bestiaire, armes, objets, fusions), des Exploits (trophees, statistiques) et
//! de la Sauvegarde (export / import du profil, depuis les options) : trois ecrans, une seule liste.
use crate::hub::{Row, DIMMED, GREEN};
use crate::lang::Lang;
use crate::profile::Profile;
use crate::visuals::Vis;
use sim::defs::Content;
use sim::horde::FACE_DOWN;

pub const TABS: [&str; 8] = ["ui.tab.bestiary", "ui.tab.weapons", "ui.tab.items", "ui.tab.fusions", "ui.tab.gallery", "ui.tab.trophies", "ui.tab.stats", "ui.tab.save"];
pub const TAB_BESTIARY: usize = 0;
pub const TAB_WEAPONS: usize = 1;
pub const TAB_ITEMS: usize = 2;
pub const TAB_FUSIONS: usize = 3;
/// Galerie : une ligne par aile, A y fait entrer Link (voir `gallery`).
pub const TAB_GALLERY: usize = 4;
pub const TAB_TROPHIES: usize = 5;
pub const TAB_STATS: usize = 6;
pub const TAB_SAVE: usize = 7;
/// Fiches des personnages (statues de la galerie) : pas un onglet, l'indice est dans `chars::roster`.
pub const CARD_CHARS: usize = 100;

/// Ecran d'un onglet : ses onglets voisins et son titre (0.6.4 : l'ancien Profil est partage en deux).
pub fn book(tab: usize) -> (std::ops::Range<usize>, &'static str) {
    match tab {
        TAB_TROPHIES | TAB_STATS => (TAB_TROPHIES..TAB_SAVE, "ui.exploits"),
        TAB_SAVE => (TAB_SAVE..TAB_SAVE + 1, "ui.options"),
        _ => (0..TAB_TROPHIES, "ui.collection"),
    }
}

/// Rang d'un boss : premier stage ou il apparait, mi-parcours (pair) puis final (impair).
pub fn boss_rank(content: &Content, boss: &str) -> u32 {
    content
        .maps
        .values()
        .filter(|m| m.order > 0)
        .filter_map(|m| {
            if m.boss.as_ref().is_some_and(|b| b.enemy == boss) {
                Some(m.order * 2)
            } else if m.final_boss.as_ref().is_some_and(|b| b.enemy == boss) {
                Some(m.order * 2 + 1)
            } else {
                None
            }
        })
        .min()
        .unwrap_or(u32::MAX)
}

/// Armes de l'armurerie, dans l'ordre ou on les rencontre : armes de depart (ordre du fichier), puis
/// celles des boss (ordre des stages). Fusions : dans l'ordre de leur arme de base.
pub fn armory_weapons(content: &Content, fusions: bool) -> Vec<usize> {
    let mut base: Vec<usize> = (0..content.weapons.len()).filter(|&k| !content.weapons[k].evolved).collect();
    base.sort_by_key(|&k| content.weapons[k].unlock.as_deref().map_or(0, |b| boss_rank(content, b)));
    if !fusions {
        return base;
    }
    let mut out: Vec<usize> = Vec::new();
    for k in base {
        for e in content.weapons[k].recipes() {
            if let Some(i) = content.weapon_index(&e.into).map(|i| i as usize) {
                if !out.contains(&i) {
                    out.push(i);
                }
            }
        }
    }
    // fusions sans arme de base (par securite) : a la fin
    out.extend((0..content.weapons.len()).filter(|&k| content.weapons[k].evolved && !out.contains(&k)).collect::<Vec<_>>());
    out
}

/// Elements de l'onglet (armes, fusions ou objets) dans l'ordre des lignes, et ceux deja decouverts
/// (seuls ceux-la ont une fiche).
pub fn card_items(content: &Content, meta: &crate::meta::Meta, p: &Profile, tab: usize) -> (Vec<usize>, Vec<usize>) {
    if tab == TAB_TROPHIES {
        let all: Vec<usize> = (0..crate::trophies::families(content, meta, p).len()).collect();
        return (all.clone(), all);
    }
    if tab == CARD_CHARS {
        let ids = crate::chars::roster(content);
        let known = (0..ids.len()).filter(|&k| crate::chars::unlocked(content, &content.characters[&ids[k]], p)).collect();
        return ((0..ids.len()).collect(), known);
    }
    let all: Vec<usize> = if tab == TAB_ITEMS { (0..content.passives.len()).collect() } else { armory_weapons(content, tab == TAB_FUSIONS) };
    let known = all
        .iter()
        .copied()
        .filter(|&k| {
            if tab == TAB_ITEMS {
                p.armory.get(&content.passive_ids[k]).copied().unwrap_or(0) > 0
            } else {
                let id = &content.weapon_ids[k];
                p.armory.get(id).copied().unwrap_or(0) > 0 || p.evolutions.contains(id)
            }
        })
        .collect();
    (all, known)
}

/// « Fusion : arme + objet » pour une arme evoluee.
pub fn fusion_recipe(content: &Content, lang: &Lang, evolved: usize) -> Option<String> {
    let into = &content.weapon_ids[evolved];
    let (k, e) = content.weapons.iter().enumerate().find_map(|(k, w)| w.recipes().find(|e| &e.into == into).map(|e| (k, e)))?;
    let a = crate::hud::hl(lang.get(&format!("weapon.{}.name", content.weapon_ids[k])));
    let b = crate::hud::hl(lang.get(&format!("passive.{}.name", e.passive)));
    Some(lang.get("ui.evo_fusion").replace("{a}", &a).replace("{b}", &b))
}

/// Lignes de l'onglet Sauvegarde.
pub const SAVE_EXPORT: usize = 0;
pub const SAVE_IMPORT: usize = 1;

fn fmt_hours(s: f64) -> String {
    let m = (s / 60.0) as u64;
    format!("{}:{:02}", m / 60, m % 60)
}

/// Ordre du bestiaire : ennemis ordinaires d'abord, puis les boss (dans l'ordre des stages, comme
/// ils sont declares). Un ennemi qu'aucun stage n'utilise n'y figure que s'il a deja ete vaincu.
pub fn bestiary_order(content: &Content, p: &Profile) -> Vec<usize> {
    let rank = |k: usize| boss_rank(content, &content.enemy_ids[k]);
    // ennemis ordinaires : du plus faible au plus fort (vie x degats de contact)
    let threat = |k: usize| -> u32 {
        let e = &content.enemies[k];
        (e.hp * e.damage.max(5.0)) as u32
    };
    let mut order: Vec<usize> = (0..content.enemies.len())
        .filter(|&k| content.enemies[k].bestiary || p.bestiary.get(&content.enemy_ids[k]).copied().unwrap_or(0) > 0)
        .collect();
    // les rencontres (Bob-omb, voleur de rubis) : entre les ennemis ordinaires et les boss
    let r = &content.rules;
    let encounter = |k: usize| content.enemy_ids[k] == r.bomb_enemy || content.enemy_ids[k] == r.thief.enemy;
    order.sort_by_key(|&k| (content.enemies[k].boss, encounter(k), if content.enemies[k].boss { rank(k) } else { threat(k) }));
    order
}

/// Ennemis deja vaincus, dans l'ordre du bestiaire (ceux que la fiche peut montrer).
pub fn bestiary_known(content: &Content, p: &Profile) -> Vec<usize> {
    bestiary_order(content, p).into_iter().filter(|&k| p.bestiary.get(&content.enemy_ids[k]).copied().unwrap_or(0) > 0).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn rows(content: &Content, meta: &crate::meta::Meta, atlas: &engine::assets::Atlas, vis: &Vis, p: &Profile, lang: &Lang, tab: usize, armed: bool) -> (Vec<Row>, Vec<String>) {
    let mut rows = Vec::new();
    let mut details = Vec::new();
    match tab {
        TAB_TROPHIES => return crate::trophies::rows(content, meta, atlas, p, lang),
        TAB_GALLERY => return crate::gallery::rows(content, meta, vis, atlas, p, lang),
        TAB_BESTIARY => {
            let figs = content.enemy_ids.iter().filter(|id| p.figurines.contains(*id)).count();
            let figs = lang.get("ui.figurines_count").replace("{a}", &figs.to_string()).replace("{b}", &bestiary_order(content, p).len().to_string());
            for k in bestiary_order(content, p) {
                let id = &content.enemy_ids[k];
                let n = p.bestiary.get(id).copied().unwrap_or(0);
                let e = &content.enemies[k];
                // figurine achetee : nom en or
                let fig = p.figurines.contains(id);
                rows.push(Row {
                    label_color: fig.then_some(crate::hud::GOLD),
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
                    let s = format!("{kind} {}", lang.get("ui.defeated_n").replace("{n}", &n.to_string()));
                    format!("{}\n{}  {figs}", s.trim(), lang.get(if fig { "ui.figurine_owned" } else { "ui.beast_open" }))
                } else {
                    lang.get("ui.unknown_enemy").to_string()
                });
            }
        }
        TAB_WEAPONS | TAB_FUSIONS => {
            for k in armory_weapons(content, tab == TAB_FUSIONS) {
                let id = &content.weapon_ids[k];
                let w = &content.weapons[k];
                let n = p.armory.get(id).copied().unwrap_or(0);
                let known = n > 0 || p.evolutions.contains(id);
                rows.push(Row {
                    label_color: None,
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
                details.push(if w.evolved {
                    // fusion : recette toujours affichee (arme + objet), le resultat seulement une fois obtenu
                    let recipe = fusion_recipe(content, lang, k).unwrap_or_default();
                    if known {
                        format!("{}\n{recipe} {}", lang.get(&format!("weapon.{id}.desc")), lang.get("ui.beast_open"))
                    } else {
                        format!("{}\n{recipe}", lang.get("ui.unknown_evolution"))
                    }
                } else if known {
                    let mut d = lang.get(&format!("weapon.{id}.desc")).to_string();
                    if let Some(e) = &w.evolution {
                        if let Some(need) = content.passive_index(&e.passive) {
                            let pid = &content.passive_ids[need as usize];
                            let into = lang.get(&format!("weapon.{}.name", e.into)).to_string();
                            let into = if p.evolutions.contains(&e.into) { crate::hud::hl_fusion(&into) } else { "???".into() };
                            let partner = crate::hud::hl(lang.get(&format!("passive.{pid}.name")));
                            d = format!("{d} {}", lang.get("ui.evolves_into").replace("{a}", &partner).replace("{b}", &into));
                        }
                    }
                    format!("{d}\n{}", lang.get("ui.beast_open"))
                } else if let Some(e) = w.unlock.as_ref().filter(|_| !p.weapons.contains(id)) {
                    let boss = crate::hud::hl(lang.get(&format!("enemy.{e}")));
                    format!("{}\n{}", lang.get("ui.unknown_item"), lang.get("ui.weapon_locked_hint").replace("{a}", &boss))
                } else {
                    lang.get("ui.unknown_item").to_string()
                });
            }
        }
        TAB_ITEMS => {
            for (k, id) in content.passive_ids.iter().enumerate() {
                let n = p.armory.get(id).copied().unwrap_or(0);
                rows.push(Row {
                    label_color: None,
                    icon: Some(vis.passives[k]),
                    icon_dim: n == 0,
                    label: if n > 0 { lang.get(&format!("passive.{id}.name")).to_string() } else { "???".into() },
                    value: n.to_string(),
                    value_color: Some(if n > 0 { crate::hud::GOLD } else { DIMMED }),
                    dim: n == 0,
                    mark: false,
                });
                details.push(if n > 0 { format!("{}\n{}", lang.get(&format!("passive.{id}.desc")), lang.get("ui.beast_open")) } else { lang.get("ui.unknown_item").to_string() });
            }
        }
        TAB_STATS => {
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
