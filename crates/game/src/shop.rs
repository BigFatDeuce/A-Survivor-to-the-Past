//! Boutique (entre les parties) : Kakariko (ameliorations), Grande Fee (benedictions),
//! reliques (a equiper) et defis (difficulte, Sceaux de Ganon). Contenu des onglets et achats.
use crate::hub::{Row, DIMMED, GREEN, RED};
use crate::lang::Lang;
use crate::meta::{stat_text, Meta};
use crate::profile::Profile;
use engine::assets::Atlas;
use engine::draw::rgb;

pub const TABS: [&str; 4] = ["ui.tab.kakariko", "ui.tab.fairy", "ui.tab.relics", "ui.tab.challenges"];

const PRICE: engine::draw::Color = rgb(248, 216, 88);

fn icon(atlas: &Atlas, name: &str) -> Option<engine::assets::AnimId> {
    atlas.find(name, "0")
}

/// Lignes de l'onglet et texte de detail de chaque ligne.
pub fn rows(meta: &Meta, p: &Profile, lang: &Lang, atlas: &Atlas, tab: usize) -> (Vec<Row>, Vec<String>) {
    let mut rows = Vec::new();
    let mut details = Vec::new();
    match tab {
        0 => {
            for it in &meta.shop {
                let lv = p.shop_level(&it.id);
                let cost = meta.shop_cost(it, lv);
                let afford = cost.is_some_and(|c| c <= p.rupees);
                rows.push(Row {
                    icon: icon(atlas, &it.icon),
                    label: format!("{}  {}", lang.get(&format!("shop.{}.name", it.id)), lang.get("ui.level_of").replace("{a}", &lv.to_string()).replace("{b}", &it.max.to_string())),
                    value: cost.map(|c| c.to_string()).unwrap_or_else(|| lang.get("ui.max").to_string()),
                    value_color: Some(match cost {
                        None => GREEN,
                        Some(_) if afford => PRICE,
                        Some(_) => RED,
                    }),
                    dim: cost.is_none(),
                    ..Default::default()
                });
                let effect = match it.stat {
                    Some(s) => stat_text(lang, s, it.per_level),
                    None => format!("+{} % {}", (it.per_level * 100.0).round(), lang.get("stat.greed")),
                };
                details.push(format!("{} {}", lang.get(&format!("shop.{}.desc", it.id)), lang.get("ui.per_level").replace("{v}", &effect)));
            }
        }
        1 => {
            for b in &meta.blessings {
                let owned = p.blessings.contains(&b.id);
                let open = meta.blessing_open(b, p);
                rows.push(Row {
                    icon: icon(atlas, &b.icon),
                    icon_dim: !open && !owned,
                    label: lang.get(&format!("blessing.{}.name", b.id)).to_string(),
                    value: if owned { lang.get("ui.owned").to_string() } else { b.cost.to_string() },
                    value_color: Some(if owned {
                        GREEN
                    } else if open && b.cost as u64 <= p.rupees {
                        PRICE
                    } else {
                        RED
                    }),
                    dim: owned || !open,
                    mark: owned,
                });
                let mut d = lang.get(&format!("blessing.{}.desc", b.id)).to_string();
                if let (false, Some(r)) = (open, b.requires.as_ref()) {
                    d = format!("{d} {}", lang.get("ui.requires").replace("{s}", lang.get(&format!("blessing.{r}.name"))));
                }
                details.push(d);
            }
        }
        2 => {
            let n = p.equipped.len();
            for r in &meta.relics {
                let found = p.relics.contains(&r.id);
                let on = p.equipped.contains(&r.id);
                rows.push(Row {
                    icon: icon(atlas, &r.icon),
                    icon_dim: !found,
                    label: if found { lang.get(&format!("relic.{}.name", r.id)).to_string() } else { "???".into() },
                    value: if on {
                        lang.get("ui.equipped").to_string()
                    } else if found {
                        String::new()
                    } else {
                        lang.get("ui.not_found").to_string()
                    },
                    value_color: Some(if on { GREEN } else { DIMMED }),
                    dim: !found,
                    mark: on,
                });
                let fx: Vec<String> = r.stats.iter().map(|&(s, v)| stat_text(lang, s, v)).collect();
                let slots = lang.get("ui.relic_slots").replace("{a}", &n.to_string()).replace("{b}", &meta.max_relics.to_string());
                details.push(if found {
                    format!("{} {}", fx.join(", "), slots)
                } else {
                    lang.get("ui.relic_hint").replace("{s}", &crate::stages::stage_name(lang, &r.stage))
                });
            }
        }
        _ => {
            let d = meta.difficulty(p);
            rows.push(Row {
                label: lang.get("ui.difficulty").to_string(),
                value: d.map(|d| format!("◀ {} ▶", lang.get(&format!("difficulty.{}.name", d.id)))).unwrap_or_default(),
                value_color: Some(PRICE),
                ..Default::default()
            });
            let mult = format!("{:.2}", meta.bank_mult(p));
            let bank = lang.get("ui.bank_mult").replace("{m}", &mult);
            details.push(format!("{} {bank}", d.map(|d| lang.get(&format!("difficulty.{}.desc", d.id))).unwrap_or("")));
            for s in &meta.seals {
                let on = p.seals.contains(&s.id);
                rows.push(Row {
                    icon: icon(atlas, &s.icon),
                    label: lang.get(&format!("seal.{}.name", s.id)).to_string(),
                    value: format!("+{} %", (s.bonus * 100.0).round()),
                    value_color: Some(if on { GREEN } else { DIMMED }),
                    mark: on,
                    ..Default::default()
                });
                details.push(format!("{} {bank}", lang.get(&format!("seal.{}.desc", s.id))));
            }
        }
    }
    (rows, details)
}

pub enum Outcome {
    Nothing,
    /// Achat ou changement fait (profil a enregistrer).
    Done,
    /// Refus, avec la cle du message (pas assez de rubis, maximum, verrouille).
    Refused(&'static str),
}

/// Action sur la ligne `k` : `dir` 0 = choisir, -1 / +1 = gauche / droite.
pub fn act(meta: &Meta, p: &mut Profile, tab: usize, k: usize, dir: i32) -> Outcome {
    match tab {
        0 if dir == 0 => {
            let Some(it) = meta.shop.get(k) else { return Outcome::Nothing };
            let lv = p.shop_level(&it.id);
            match meta.shop_cost(it, lv) {
                Some(c) if c <= p.rupees => {
                    p.rupees -= c;
                    p.shop.insert(it.id.clone(), lv + 1);
                    Outcome::Done
                }
                Some(_) => Outcome::Refused("ui.not_enough"),
                None => Outcome::Refused("ui.maxed"),
            }
        }
        1 if dir == 0 => {
            let Some(b) = meta.blessings.get(k) else { return Outcome::Nothing };
            if p.blessings.contains(&b.id) {
                return Outcome::Refused("ui.owned");
            }
            if !meta.blessing_open(b, p) {
                return Outcome::Refused("ui.locked");
            }
            if (b.cost as u64) > p.rupees {
                return Outcome::Refused("ui.not_enough");
            }
            p.rupees -= b.cost as u64;
            p.blessings.insert(b.id.clone());
            Outcome::Done
        }
        2 if dir == 0 => {
            let Some(r) = meta.relics.get(k) else { return Outcome::Nothing };
            if !p.relics.contains(&r.id) {
                return Outcome::Refused("ui.locked");
            }
            if let Some(i) = p.equipped.iter().position(|e| *e == r.id) {
                p.equipped.remove(i);
            } else if p.equipped.len() < meta.max_relics {
                p.equipped.push(r.id.clone());
            } else {
                return Outcome::Refused("ui.slots_full");
            }
            Outcome::Done
        }
        3 if k == 0 => {
            let n = meta.difficulties.len().max(1);
            let step = if dir == 0 { 1 } else { dir };
            p.difficulty = ((p.difficulty.min(n - 1) as i32 + step).rem_euclid(n as i32)) as usize;
            Outcome::Done
        }
        3 if dir == 0 => {
            let Some(s) = meta.seals.get(k - 1) else { return Outcome::Nothing };
            if !p.seals.remove(&s.id) {
                p.seals.insert(s.id.clone());
            }
            Outcome::Done
        }
        _ => Outcome::Nothing,
    }
}

/// Gauche / droite changent d'onglet, sauf sur la ligne de difficulte.
pub fn row_uses_sides(tab: usize, k: usize) -> bool {
    tab == 3 && k == 0
}
