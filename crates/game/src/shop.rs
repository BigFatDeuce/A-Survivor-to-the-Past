//! Boutique (entre les parties) : Cocorico (ameliorations), Grande Fee (benedictions et offrandes),
//! personnages, garde-robe (tenues) et juke-box (musique de l'ecran titre). Les reliques (a equiper)
//! et les defis (difficulte, Sceaux de Ganon) se reglent juste avant de partir, dans l'ecran des
//! Preparatifs (0.8) : memes onglets, autre ecran. Contenu des onglets et achats.
use crate::hub::{Row, DIMMED, GREEN, RED};
use crate::lang::Lang;
use crate::meta::{perk_text, relic_fx, stat_text, Meta};
use crate::profile::Profile;
use engine::assets::Atlas;
use engine::draw::rgb;

/// 0.8 : Cocorico ouvre la boutique, puis la Grande Fee ; reliques et defis passent aux Preparatifs.
pub const TABS: [&str; 7] = ["ui.tab.kakariko", "ui.tab.fairy", "ui.tab.characters", "ui.tab.wardrobe", "ui.tab.jukebox", "ui.tab.relics", "ui.tab.challenges"];
pub const TAB_KAKARIKO: usize = 0;
pub const TAB_FAIRY: usize = 1;
pub const TAB_CHARS: usize = 2;
pub const TAB_WARDROBE: usize = 3;
pub const TAB_JUKEBOX: usize = 4;
pub const TAB_RELICS: usize = 5;
pub const TAB_CHALLENGES: usize = 6;
/// Onglets de la boutique, puis ceux des Preparatifs (avant de lancer une partie).
pub const SHOP: std::ops::Range<usize> = TAB_KAKARIKO..TAB_RELICS;
pub const PREP: std::ops::Range<usize> = TAB_RELICS..TABS.len();

/// Couleurs des rangs de reliques : legendaire, rare, commune.
pub const TIER_COLORS: [engine::draw::Color; 3] = [rgb(255, 205, 70), rgb(200, 140, 255), rgb(130, 200, 255)];

/// Nom d'une musique d'apres son identifiant (titres anglais officiels de la bande originale) :
/// « princess_zelda_s_rescue » -> « Princess Zelda's Rescue ».
pub fn track_title(id: &str) -> String {
    let small = ["of", "the", "to", "in", "a"];
    id.replace("_s_", "'s_")
        .split('_')
        .enumerate()
        .map(|(k, w)| {
            if k > 0 && small.contains(&w) {
                w.to_string()
            } else {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Musique de l'ecran titre choisie au juke-box (None : celle par defaut).
pub fn title_music<'a>(meta: &'a Meta, p: &Profile) -> Option<&'a str> {
    meta.jukebox.iter().find(|t| t.id == p.title_music && track_owned(t, p)).map(|t| t.id.as_str())
}

fn track_owned(t: &crate::meta::Track, p: &Profile) -> bool {
    t.cost == 0 || p.jukebox.contains(&t.id)
}

/// Pseudo au classement selon le titre offert (1 a 3) : vert, or, rose chatoyant.
pub fn titled_name(title: u32, name: &str) -> String {
    match title {
        0 => name.to_string(),
        1 => crate::hud::hl_new(name),
        2 => crate::hud::hl(name),
        _ => crate::hud::hl_fusion(name),
    }
}

const PRICE: engine::draw::Color = rgb(248, 216, 88);

/// Tenues montrees a la garde-robe : celles des personnages deja debloques (ou achetes), pour ne pas
/// noyer la liste sous les centaines de couleurs des personnages encore inconnus.
pub fn wardrobe_items<'a>(meta: &'a Meta, content: &sim::defs::Content, p: &Profile) -> Vec<&'a crate::meta::WardrobeItem> {
    meta.wardrobe
        .iter()
        .filter(|w| content.characters.get(&w.character).is_some_and(|d| crate::chars::unlocked(content, d, p)))
        .collect()
}

fn icon(atlas: &Atlas, name: &str) -> Option<engine::assets::AnimId> {
    atlas.find(name, "0")
}

/// Lignes de l'onglet et texte de detail de chaque ligne.
pub fn rows(meta: &Meta, content: &sim::defs::Content, p: &Profile, lang: &Lang, atlas: &Atlas, tab: usize) -> (Vec<Row>, Vec<String>) {
    let mut rows = Vec::new();
    let mut details = Vec::new();
    match tab {
        TAB_KAKARIKO => {
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
                let effect = match (it.stat, it.perk) {
                    (_, Some(pk)) => perk_text(lang, pk, it.per_level.round() as u32),
                    (Some(s), None) => stat_text(lang, s, it.per_level),
                    (None, None) => format!("+{} % {}", (it.per_level * 100.0).round(), lang.get("stat.greed")),
                };
                details.push(format!("{} {}", lang.get(&format!("shop.{}.desc", it.id)), lang.get("ui.per_level").replace("{v}", &effect)));
            }
        }
        TAB_FAIRY => {
            for b in &meta.blessings {
                let owned = p.blessings.contains(&b.id);
                let open = meta.blessing_open(b, p);
                // la sorciere (en tete de la boutique) : son portrait et son nom en rose chatoyant
                let witch = matches!(b.effect, crate::meta::BlessingEffect::Witch);
                let name = lang.get(&format!("blessing.{}.name", b.id));
                rows.push(Row {
                    label_color: None,
                    icon: if witch { atlas.find("npc_witch", "0") } else { icon(atlas, &b.icon) },
                    icon_dim: !open && !owned,
                    label: if witch && !owned { crate::hud::hl_fusion(name) } else { name.to_string() },
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
            // offrandes : titres au classement, dans l'ordre
            for (j, t) in meta.titles.iter().enumerate() {
                let owned = (p.title as usize) > j;
                let open = p.title as usize >= j;
                let name = lang.get(&format!("title.{}", t.id));
                rows.push(Row {
                    icon: icon(atlas, "icon_fairy_bottle"),
                    icon_dim: !open && !owned,
                    label: format!("{} {}", lang.get("ui.offering"), titled_name(j as u32 + 1, name)),
                    value: if owned { lang.get("ui.owned").to_string() } else { t.cost.to_string() },
                    value_color: Some(if owned {
                        GREEN
                    } else if open && t.cost <= p.rupees {
                        PRICE
                    } else {
                        RED
                    }),
                    dim: owned || !open,
                    mark: owned,
                    ..Default::default()
                });
                let mut d = lang.get("ui.offering_desc").replace("{s}", &titled_name(j as u32 + 1, name));
                if j > 0 && !open {
                    d = format!("{d} {}", lang.get("ui.requires").replace("{s}", lang.get(&format!("title.{}", meta.titles[j - 1].id))));
                }
                details.push(d);
            }
        }
        TAB_CHARS => {
            for (id, cost) in crate::chars::for_sale(content) {
                let owned = p.chars_bought.contains(&id);
                let def = &content.characters[&id];
                rows.push(Row {
                    icon: atlas.find(&def.sprite, "idle_down"),
                    label: crate::chars::name(lang, &id),
                    value: if owned { lang.get("ui.owned").to_string() } else { cost.to_string() },
                    value_color: Some(if owned {
                        GREEN
                    } else if cost <= p.rupees {
                        PRICE
                    } else {
                        RED
                    }),
                    mark: owned,
                    ..Default::default()
                });
                let d = crate::chars::desc(lang, def);
                // ce que le joueur achete : coeurs, vitesse, avantages et defauts, puis la description
                details.push(format!("{}\n{d} {}", crate::chars::stats_line(lang, def), lang.get(if owned { "ui.char_bought" } else { "ui.char_buy" })));
            }
        }
        TAB_WARDROBE => {
            for w in wardrobe_items(meta, content, p) {
                let owned = p.wardrobe.contains(&w.sprite);
                let def = content.characters.get(&w.character);
                let worn = def.is_some_and(|d| d.skins.get(crate::chars::skin(d, p, &w.character, meta)).is_some_and(|s| s.0 == w.sprite));
                // couleur ajoutee au chargement : « Personnage : couleur »
                let color = def.and_then(|d| d.skins.iter().find(|s| s.0 == w.sprite)).map(|s| lang.get(&format!("skin.{}", s.1)).to_string()).unwrap_or_default();
                let cname = crate::chars::name(lang, &w.character);
                rows.push(Row {
                    icon: icon(atlas, &w.icon).or_else(|| atlas.find(&w.sprite, "idle_down")),
                    label: if w.auto { format!("{cname} : {color}") } else { lang.get(&format!("wardrobe.{}.name", w.sprite)).to_string() },
                    value: if worn {
                        lang.get("ui.equipped").to_string()
                    } else if owned {
                        lang.get("ui.owned").to_string()
                    } else {
                        w.cost.to_string()
                    },
                    value_color: Some(if owned {
                        GREEN
                    } else if w.cost <= p.rupees {
                        PRICE
                    } else {
                        RED
                    }),
                    mark: worn,
                    ..Default::default()
                });
                let desc = if w.auto { lang.get("ui.wardrobe_auto_desc").replace("{c}", &cname) } else { lang.get(&format!("wardrobe.{}.desc", w.sprite)).to_string() };
                details.push(format!("{desc} {}", lang.get(if owned { "ui.wardrobe_wear" } else { "ui.wardrobe_buy" })));
            }
        }
        TAB_JUKEBOX => {
            let current = title_music(meta, p);
            for (j, t) in meta.jukebox.iter().enumerate() {
                let owned = track_owned(t, p);
                // sans choix : la premiere (offerte) est celle de l'ecran titre
                let on = current.map_or(j == 0, |c| c == t.id);
                rows.push(Row {
                    icon: icon(atlas, "icon_ocarina"),
                    icon_dim: !owned,
                    label: track_title(&t.id),
                    value: if on {
                        lang.get("ui.jukebox_on").to_string()
                    } else if owned {
                        lang.get("ui.owned").to_string()
                    } else {
                        t.cost.to_string()
                    },
                    value_color: Some(if owned {
                        GREEN
                    } else if t.cost <= p.rupees {
                        PRICE
                    } else {
                        RED
                    }),
                    mark: on,
                    ..Default::default()
                });
                details.push(lang.get(if owned { "ui.jukebox_play" } else { "ui.jukebox_buy" }).to_string());
            }
        }
        TAB_RELICS => {
            let n = p.equipped.len();
            // de la plus forte a la plus faible, le nom a la couleur de son rang
            for r in meta.relic_order().into_iter().map(|k| &meta.relics[k]) {
                let found = p.relics.contains(&r.id);
                let on = p.equipped.contains(&r.id);
                let tier = crate::meta::relic_tier(r);
                rows.push(Row {
                    label_color: (found || r.cost > 0).then_some(TIER_COLORS[tier]),
                    icon: icon(atlas, &r.icon),
                    icon_dim: !found && r.cost == 0,
                    // a vendre : nom et prix visibles avant l'achat
                    label: if found || r.cost > 0 { lang.get(&format!("relic.{}.name", r.id)).to_string() } else { "???".into() },
                    value: if on {
                        lang.get("ui.equipped").to_string()
                    } else if found {
                        String::new()
                    } else if r.cost > 0 {
                        r.cost.to_string()
                    } else {
                        lang.get("ui.not_found").to_string()
                    },
                    value_color: Some(if on {
                        GREEN
                    } else if !found && r.cost > 0 {
                        if r.cost <= p.rupees {
                            PRICE
                        } else {
                            RED
                        }
                    } else {
                        DIMMED
                    }),
                    dim: !found && r.cost == 0,
                    mark: on,
                });
                let mut fx = relic_fx(lang, r);
                if let Some(first) = fx.first_mut() {
                    *first = format!("{} : {first}", lang.get(&format!("ui.tier.{tier}")));
                }
                let slots = lang.get("ui.relic_slots").replace("{a}", &n.to_string()).replace("{b}", &meta.max_relics.to_string());
                details.push(if found {
                    // les emplacements sur leur propre ligne (retour 0.6.1)
                    format!("{}\n{}", fx.join(", "), slots)
                } else if r.cost > 0 {
                    format!("{}\n{}", fx.join(", "), lang.get("ui.relic_buy").replace("{n}", &r.cost.to_string()))
                } else {
                    if r.encounter {
                        lang.get("ui.relic_hint_encounter").to_string()
                    } else {
                        lang.get("ui.relic_hint").replace("{s}", &crate::stages::stage_name(lang, &r.stage))
                    }
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
    /// Achat fait (profil a enregistrer).
    Done,
    /// Choix fait sans achat : equiper, porter une tenue, choisir une musique (profil a enregistrer).
    Selected,
    /// Refus, avec la cle du message (pas assez de rubis, maximum, verrouille).
    Refused(&'static str),
}

/// Action sur la ligne `k` : `dir` 0 = choisir, -1 / +1 = gauche / droite.
pub fn act(meta: &Meta, content: &sim::defs::Content, p: &mut Profile, tab: usize, k: usize, dir: i32) -> Outcome {
    match tab {
        TAB_KAKARIKO if dir == 0 => {
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
        TAB_FAIRY if dir == 0 && k >= meta.blessings.len() => {
            let j = k - meta.blessings.len();
            let Some(t) = meta.titles.get(j) else { return Outcome::Nothing };
            if (p.title as usize) > j {
                return Outcome::Refused("ui.owned");
            }
            if (p.title as usize) < j {
                return Outcome::Refused("ui.locked");
            }
            if t.cost > p.rupees {
                return Outcome::Refused("ui.not_enough");
            }
            p.rupees -= t.cost;
            p.title = j as u32 + 1;
            Outcome::Done
        }
        TAB_FAIRY if dir == 0 => {
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
        TAB_RELICS if dir == 0 => {
            let Some(r) = meta.relic_order().get(k).map(|&i| &meta.relics[i]) else { return Outcome::Nothing };
            if !p.relics.contains(&r.id) && r.cost > 0 {
                // achat : equipee d'office s'il reste un emplacement (comme une relique trouvee)
                if r.cost > p.rupees {
                    return Outcome::Refused("ui.not_enough");
                }
                p.rupees -= r.cost;
                p.relics.insert(r.id.clone());
                if p.equipped.len() < meta.max_relics {
                    p.equipped.push(r.id.clone());
                }
                return Outcome::Done;
            }
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
            Outcome::Selected
        }
        TAB_CHARS if dir == 0 => {
            let Some((id, cost)) = crate::chars::for_sale(content).into_iter().nth(k) else { return Outcome::Nothing };
            if p.chars_bought.contains(&id) {
                return Outcome::Refused("ui.owned");
            }
            if cost > p.rupees {
                return Outcome::Refused("ui.not_enough");
            }
            p.rupees -= cost;
            p.chars_bought.insert(id.clone());
            // pas encore joue : etincelle dans l'ecran des personnages
            p.new_chars.insert(id);
            Outcome::Done
        }
        // la tenue est ensuite portee par le jeu (il connait les couleurs du personnage)
        TAB_WARDROBE if dir == 0 => {
            let Some(w) = wardrobe_items(meta, content, p).get(k).copied() else { return Outcome::Nothing };
            if p.wardrobe.contains(&w.sprite) {
                return Outcome::Selected;
            }
            if w.cost > p.rupees {
                return Outcome::Refused("ui.not_enough");
            }
            p.rupees -= w.cost;
            p.wardrobe.insert(w.sprite.clone());
            Outcome::Done
        }
        TAB_JUKEBOX if dir == 0 => {
            let Some(t) = meta.jukebox.get(k) else { return Outcome::Nothing };
            let bought = !track_owned(t, p);
            if bought {
                if t.cost > p.rupees {
                    return Outcome::Refused("ui.not_enough");
                }
                p.rupees -= t.cost;
                p.jukebox.insert(t.id.clone());
            }
            p.title_music = t.id.clone();
            if bought {
                Outcome::Done
            } else {
                Outcome::Selected
            }
        }
        TAB_CHALLENGES if k == 0 => {
            let n = meta.difficulties.len().max(1);
            let step = if dir == 0 { 1 } else { dir };
            p.difficulty = ((p.difficulty.min(n - 1) as i32 + step).rem_euclid(n as i32)) as usize;
            Outcome::Selected
        }
        TAB_CHALLENGES if dir == 0 => {
            let Some(s) = meta.seals.get(k - 1) else { return Outcome::Nothing };
            if !p.seals.remove(&s.id) {
                p.seals.insert(s.id.clone());
            }
            Outcome::Selected
        }
        _ => Outcome::Nothing,
    }
}

/// Gauche / droite changent d'onglet, sauf sur la ligne de difficulte.
pub fn row_uses_sides(tab: usize, k: usize) -> bool {
    tab == TAB_CHALLENGES && k == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_rupee_sinks() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let meta = Meta::load(&root);
        let content = crate::content::load_content(&root);
        let mut p = Profile::new();
        p.rupees = 100_000;
        // offrandes : dans l'ordre seulement
        let nb = meta.blessings.len();
        assert!(matches!(act(&meta, &content, &mut p, TAB_FAIRY, nb + 1, 0), Outcome::Refused("ui.locked")));
        assert!(matches!(act(&meta, &content, &mut p, TAB_FAIRY, nb, 0), Outcome::Done));
        assert_eq!(p.title, 1);
        assert!(matches!(act(&meta, &content, &mut p, TAB_FAIRY, nb, 0), Outcome::Refused("ui.owned")));
        // garde-robe : achetee une fois, puis seulement portee
        assert!(meta.skin_locked(&p, "link_blue"));
        assert!(matches!(act(&meta, &content, &mut p, TAB_WARDROBE, 0, 0), Outcome::Done));
        assert!(!meta.skin_locked(&p, "link_blue"));
        let left = p.rupees;
        assert!(matches!(act(&meta, &content, &mut p, TAB_WARDROBE, 0, 0), Outcome::Selected));
        assert_eq!(p.rupees, left);
        // juke-box : la premiere est offerte, les autres achetees puis choisies
        assert_eq!(title_music(&meta, &p), None);
        assert!(matches!(act(&meta, &content, &mut p, TAB_JUKEBOX, 0, 0), Outcome::Selected));
        assert!(matches!(act(&meta, &content, &mut p, TAB_JUKEBOX, 3, 0), Outcome::Done));
        assert_eq!(title_music(&meta, &p), Some(meta.jukebox[3].id.as_str()));
        assert_eq!(track_title("princess_zelda_s_rescue"), "Princess Zelda's Rescue");
        assert_eq!(track_title("title_link_to_the_past"), "Title Link to the Past");
        // personnages : achetes une fois, alors debloques
        let (id, cost) = crate::chars::for_sale(&content)[0].clone();
        assert!(!crate::chars::unlocked(&content, &content.characters[&id], &p));
        let left = p.rupees;
        assert!(matches!(act(&meta, &content, &mut p, TAB_CHARS, 0, 0), Outcome::Done));
        assert_eq!(p.rupees, left - cost);
        assert!(crate::chars::unlocked(&content, &content.characters[&id], &p));
        assert!(matches!(act(&meta, &content, &mut p, TAB_CHARS, 0, 0), Outcome::Refused("ui.owned")));
        // pas assez de rubis : rien ne change
        p.rupees = 0;
        assert!(matches!(act(&meta, &content, &mut p, TAB_JUKEBOX, 4, 0), Outcome::Refused("ui.not_enough")));
        assert!(!p.jukebox.contains(&meta.jukebox[4].id));
    }
}
