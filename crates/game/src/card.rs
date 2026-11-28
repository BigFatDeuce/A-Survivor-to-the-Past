//! Fiches detaillees de la Collection (armes, fusions, objets, trophees ; traits des ennemis pour le
//! bestiaire). Tout vient des donnees du jeu (weapons.ron, passives.ron, enemies.ron, trophees du
//! profil) : rien n'est invente.
use crate::lang::Lang;
use crate::meta::{stat_text, Meta};
use crate::profile::Profile;
use sim::defs::{Behavior, BossAction, Content, StatusDef, WeaponKind};
use sim::upgrades::{resolve, Stats};

/// Ligne d'une fiche : statistique au niveau 1 et au niveau maximal (vide : arme sans niveaux), ou
/// texte sur toute la largeur.
pub enum Line {
    Stat(String, String, String),
    Text(String, String),
}

pub struct Card {
    pub title: String,
    pub fusion: bool,
    /// Description (texte du jeu), puis les lignes, puis les notes (effets, fusions).
    pub desc: String,
    pub lines: Vec<Line>,
    pub notes: Vec<String>,
}

/// Nombre court : entier s'il est rond, sinon un chiffre apres la virgule.
fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 || v.abs() >= 100.0 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

/// Duree en secondes (deux chiffres sous la seconde).
fn secs(v: f32) -> String {
    if v < 1.0 {
        format!("{v:.2} s")
    } else {
        format!("{} s", num(v))
    }
}

fn kind_key(k: WeaponKind) -> &'static str {
    match k {
        WeaponKind::Swing => "card.kind.swing",
        WeaponKind::Projectile => "card.kind.projectile",
        WeaponKind::Boomerang => "card.kind.boomerang",
        WeaponKind::Lob => "card.kind.lob",
        WeaponKind::Slam => "card.kind.slam",
        WeaponKind::Orbit => "card.kind.orbit",
        WeaponKind::Pillars => "card.kind.pillars",
        WeaponKind::Storm => "card.kind.storm",
        WeaponKind::Aura => "card.kind.aura",
        WeaponKind::Somaria => "card.kind.somaria",
        WeaponKind::Bees => "card.kind.bees",
        WeaponKind::Jump => "card.kind.jump",
        WeaponKind::Tornado => "card.kind.tornado",
    }
}

/// Fiche d'une arme (ou d'une fusion) : caracteristiques au niveau 1 et au maximum, avec le bonus de
/// degats commun a toutes les armes (`rules.damage_bonus`), sans objet ni amelioration de boutique.
pub fn weapon(content: &Content, lang: &Lang, p: &Profile, k: usize) -> Card {
    let id = &content.weapon_ids[k];
    let w = &content.weapons[k];
    let st = Stats { might: 1.0 + content.rules.damage_bonus, ..Stats::default() };
    let max = w.levels.len() as u32 + 1;
    let (a, b) = (resolve(w, 1, &st), resolve(w, max, &st));
    let mut lines = vec![Line::Text(lang.get("card.kind").to_string(), lang.get(kind_key(w.kind)).to_string())];
    // fusion sans amelioration : une seule colonne
    let single = max == 1;
    let mut stat = |key: &str, x: String, y: String| lines.push(Line::Stat(lang.get(key).to_string(), x, if single { String::new() } else { y }));
    stat("stat.damage", num(a.damage), num(b.damage));
    stat("stat.cooldown", secs(a.cooldown), secs(b.cooldown));
    // orbite et nuage frappent en continu (une touche par ennemi toutes les `hit_interval`)
    if !matches!(w.kind, WeaponKind::Orbit | WeaponKind::Aura) {
        stat("card.frequency", format!("{:.2}", a.amount as f32 / a.cooldown), format!("{:.2}", b.amount as f32 / b.cooldown));
    }
    if a.amount > 1 || b.amount > a.amount {
        stat("stat.amount", a.amount.to_string(), b.amount.to_string());
    }
    stat("stat.area", num(a.area), num(b.area));
    if matches!(w.kind, WeaponKind::Projectile | WeaponKind::Boomerang | WeaponKind::Lob | WeaponKind::Bees) {
        stat("card.range", num(w.range), num(w.range));
    }
    if a.speed > 0.0 {
        stat("stat.speed", num(a.speed), num(b.speed));
    }
    if a.duration > 0.0 {
        stat("stat.duration", secs(a.duration), secs(b.duration));
    }
    if matches!(w.kind, WeaponKind::Projectile | WeaponKind::Boomerang | WeaponKind::Swing | WeaponKind::Bees | WeaponKind::Tornado) {
        let pierce = |n: u32| if n == 0 { lang.get("card.unlimited").to_string() } else { n.to_string() };
        stat("stat.pierce", pierce(a.pierce), pierce(b.pierce));
    }
    if b.knockback > 0.0 {
        stat("stat.knockback", num(a.knockback), num(b.knockback));
    }
    if b.crit_chance > 0.0 {
        let crit = |c: f32| format!("{} % x{}", (c * 100.0).round(), num(w.base.crit_mult));
        stat("stat.crit", crit(a.crit_chance), crit(b.crit_chance));
    }
    let mut notes = Vec::new();
    match w.status {
        Some(StatusDef::Burn { dps, time }) => notes.push(lang.get("card.burn").replace("{d}", &num(dps)).replace("{t}", &num(time))),
        Some(StatusDef::Freeze { time }) => notes.push(lang.get("card.freeze").replace("{t}", &num(time))),
        Some(StatusDef::Stun { time }) => notes.push(lang.get("card.stun").replace("{t}", &num(time))),
        None => {}
    }
    if let Some(z) = w.zone {
        notes.push(lang.get("card.zone").replace("{r}", &num(z.radius)).replace("{d}", &num(z.dps)).replace("{t}", &num(z.time)));
    }
    if let Some(bm) = &w.beam {
        notes.push(lang.get("card.beam").replace("{p}", &num(bm.damage * 100.0)));
    }
    if w.kind == WeaponKind::Somaria {
        notes.push(lang.get("card.rays").replace("{n}", &w.rays.to_string()));
    }
    if w.evolved {
        notes.extend(crate::records::fusion_recipe(content, lang, k));
    }
    // fusions possibles : le resultat reste cache tant qu'il n'a pas ete obtenu
    for e in w.evolution.iter().chain(w.ultimate.iter()) {
        let partner = crate::hud::hl(lang.get(&format!("passive.{}.name", e.passive)));
        let into = if p.evolutions.contains(&e.into) { crate::hud::hl_fusion(lang.get(&format!("weapon.{}.name", e.into))) } else { "???".into() };
        notes.push(lang.get("ui.evolves_into").replace("{a}", &partner).replace("{b}", &into));
    }
    Card { title: lang.get(&format!("weapon.{id}.name")).to_string(), fusion: w.evolved, desc: lang.get(&format!("weapon.{id}.desc")).to_string(), lines, notes }
}

/// Fiche d'un objet passif : bonus de chaque niveau, total, et les armes qu'il fait evoluer.
pub fn passive(content: &Content, lang: &Lang, p: &Profile, k: usize) -> Card {
    let id = &content.passive_ids[k];
    let def = &content.passives[k];
    let join = |v: &[(sim::defs::Stat, f32)]| v.iter().map(|&(s, x)| stat_text(lang, s, x)).collect::<Vec<_>>().join(", ");
    let mut lines: Vec<Line> = def.levels.iter().enumerate().map(|(i, lv)| Line::Text(lang.get("ui.level_of").replace("{a}", &(i + 1).to_string()).replace("{b}", &def.levels.len().to_string()), join(lv))).collect();
    // total au niveau maximal : bonus d'une meme statistique additionnes
    let mut total: Vec<(sim::defs::Stat, f32)> = Vec::new();
    for &(s, x) in def.levels.iter().flatten() {
        match total.iter_mut().find(|t| t.0 == s) {
            Some(t) => t.1 += x,
            None => total.push((s, x)),
        }
    }
    if def.levels.len() > 1 {
        lines.push(Line::Text(lang.get("card.total").to_string(), join(&total)));
    }
    let mut notes = Vec::new();
    for (wk, w) in content.weapons.iter().enumerate() {
        for e in w.evolution.iter().chain(w.ultimate.iter()).filter(|e| e.passive == *id) {
            let a = crate::hud::hl(lang.get(&format!("weapon.{}.name", content.weapon_ids[wk])));
            let b = if p.evolutions.contains(&e.into) { crate::hud::hl_fusion(lang.get(&format!("weapon.{}.name", e.into))) } else { "???".into() };
            notes.push(lang.get("card.partner").replace("{a}", &a).replace("{b}", &b));
        }
    }
    Card { title: lang.get(&format!("passive.{id}.name")).to_string(), fusion: false, desc: lang.get(&format!("passive.{id}.desc")).to_string(), lines, notes }
}

/// Fiche d'un personnage (statue de la galerie) : coeurs, vitesse, recharge du dash, arme de depart et
/// ses degats au niveau 1 (bonus commun et bonus du personnage compris), puis ses avantages (vert) et
/// defauts (rouge).
pub fn character(content: &Content, lang: &Lang, id: &str) -> Card {
    let Some(def) = content.characters.get(id) else {
        return Card { title: String::new(), fusion: false, desc: String::new(), lines: Vec::new(), notes: Vec::new() };
    };
    let mut lines = vec![
        Line::Text(lang.get("ui.char_health").to_string(), num(def.max_hp / 10.0)),
        Line::Text(lang.get("ui.char_speed").to_string(), num(def.speed)),
        Line::Text(lang.get("stat.dash_cd").to_string(), secs(def.dash.cooldown)),
        Line::Text(lang.get("ui.char_weapon").to_string(), lang.get(&format!("weapon.{}.name", def.weapon)).to_string()),
    ];
    if let Some(k) = content.weapon_index(&def.weapon) {
        let bonus: f32 = def.stats.iter().filter(|s| s.0 == sim::defs::Stat::Might).map(|s| s.1).sum();
        let st = Stats { might: 1.0 + content.rules.damage_bonus + bonus, ..Stats::default() };
        lines.push(Line::Text(lang.get("stat.damage").to_string(), num(resolve(&content.weapons[k as usize], 1, &st).damage)));
    }
    let notes = def.stats.iter().map(|&(s, v)| crate::chars::stat_colored(lang, s, v)).collect();
    Card { title: crate::chars::name(lang, id), fusion: false, desc: crate::chars::desc(lang, def), lines, notes }
}

/// Fiche d'un trophee (galerie) : ce qu'il demande pour le prochain palier, puis chaque palier, obtenu
/// ou avec l'avancement.
pub fn trophy(content: &Content, meta: &Meta, lang: &Lang, p: &Profile, k: usize) -> Card {
    use crate::trophies::{amount, families, TIER_KEYS};
    let fams = families(content, meta, p);
    let Some(f) = fams.get(k) else {
        return Card { title: String::new(), fusion: false, desc: String::new(), lines: Vec::new(), notes: Vec::new() };
    };
    let goals: Vec<(usize, u64)> = f.goals.iter().copied().enumerate().filter(|&(_, g)| g > 0).collect();
    let next = goals.iter().find(|&&(_, g)| f.value < g).or(goals.last()).map_or(0, |&(_, g)| g);
    let lines = goals
        .iter()
        .map(|&(r, g)| {
            let v = if f.value >= g { lang.get("ui.owned").to_string() } else { format!("{} / {}", amount(f, f.value), amount(f, g)) };
            Line::Text(lang.get(TIER_KEYS[r]).to_string(), v)
        })
        .collect();
    Card {
        title: lang.get(&format!("trophy.{}", f.id)).to_string(),
        fusion: false,
        desc: lang.get(&format!("trophy.{}.desc", f.id)).replace("{n}", &amount(f, next)),
        lines,
        notes: Vec::new(),
    }
}

/// Comportement et particularites d'un ennemi (fiche du bestiaire).
pub fn enemy_traits(content: &Content, lang: &Lang, k: usize) -> Vec<String> {
    let e = &content.enemies[k];
    let hearts = |d: f32| num((d / 10.0 * 2.0).round() / 2.0);
    let mut out = Vec::new();
    match &e.behavior {
        Behavior::Chase => out.push(lang.get("card.b.chase").to_string()),
        Behavior::Flutter { .. } => out.push(lang.get("card.b.flutter").to_string()),
        Behavior::Hop { .. } => out.push(lang.get("card.b.hop").to_string()),
        Behavior::Flee { .. } => out.push(lang.get("card.b.flee").to_string()),
        Behavior::Wander { .. } => out.push(lang.get("card.b.wander").to_string()),
        Behavior::Charger { .. } => out.push(lang.get("card.b.charger").to_string()),
        Behavior::Shooter { shot, .. } => out.push(lang.get("card.b.shooter").replace("{d}", &hearts(shot.damage))),
        Behavior::Boss { actions } => {
            let mut seen = Vec::new();
            for a in actions {
                let s = match a {
                    BossAction::Throw { shot, .. } => lang.get("card.b.shooter").replace("{d}", &hearts(shot.damage)),
                    BossAction::Summon { .. } => lang.get("card.b.summon").to_string(),
                    BossAction::Charge { .. } => lang.get("card.b.charger").to_string(),
                    BossAction::Leap { damage, .. } => lang.get("card.b.leap").replace("{d}", &hearts(*damage)),
                    BossAction::Teleport { .. } => lang.get("card.b.teleport").to_string(),
                    _ => continue,
                };
                if !seen.contains(&s) {
                    seen.push(s);
                }
            }
            if seen.is_empty() {
                seen.push(lang.get("card.b.chase").to_string());
            }
            out.extend(seen);
        }
    }
    if e.flying {
        out.push(lang.get("card.t.flying").to_string());
    }
    if e.mass >= 1.5 {
        out.push(lang.get("card.t.heavy").to_string());
    } else if e.mass <= 0.75 {
        out.push(lang.get("card.t.light").to_string());
    }
    if e.xp > 0 {
        out.push(lang.get("card.t.rupees").replace("{n}", &e.xp.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque arme, fusion, objet et ennemi a sa fiche, sans texte manquant.
    #[test]
    fn every_card_is_complete() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = crate::content::load_content(&root);
        let meta = Meta::load(&root);
        let p = Profile::new();
        for code in crate::lang::LANGS {
            let lang = Lang::load(&root, code);
            let mut texts = Vec::new();
            for k in 0..content.weapons.len() {
                let c = weapon(&content, &lang, &p, k);
                assert!(c.lines.len() >= 4, "fiche courte : {}", content.weapon_ids[k]);
                texts.push(c.title);
                texts.extend(c.notes);
                texts.extend(c.lines.into_iter().map(|l| match l {
                    Line::Stat(a, b, c) => format!("{a}{b}{c}"),
                    Line::Text(a, b) => format!("{a}{b}"),
                }));
            }
            for k in 0..content.passives.len() {
                let c = passive(&content, &lang, &p, k);
                texts.extend(c.notes);
                texts.extend(c.lines.into_iter().map(|l| if let Line::Text(a, b) = l { format!("{a}{b}") } else { String::new() }));
            }
            for k in 0..crate::trophies::families(&content, &meta, &p).len() {
                let c = trophy(&content, &meta, &lang, &p, k);
                assert!(!c.title.is_empty() && !c.desc.is_empty() && !c.lines.is_empty(), "fiche de trophee {k}");
                texts.extend(c.lines.into_iter().map(|l| if let Line::Text(a, b) = l { format!("{a}{b}") } else { String::new() }));
            }
            for k in 0..content.enemies.len() {
                assert!(!enemy_traits(&content, &lang, k).is_empty());
                texts.extend(enemy_traits(&content, &lang, k));
            }
            for id in crate::chars::roster(&content) {
                let c = character(&content, &lang, &id);
                assert!(!c.title.is_empty() && !c.desc.is_empty() && c.lines.len() == 5, "fiche de personnage {id}");
                texts.push(c.desc);
                texts.extend(c.lines.into_iter().map(|l| if let Line::Text(a, b) = l { format!("{a}{b}") } else { String::new() }));
            }
            assert!(texts.iter().all(|t| !t.contains('?') || t.contains("???")), "texte absent ({code})");
        }
        // epee : degats du niveau 1 avec le bonus commun
        let k = content.weapon_index("sword").unwrap() as usize;
        let c = weapon(&content, &Lang::load(&root, "fr"), &p, k);
        let want = num(content.weapons[k].base.damage * (1.0 + content.rules.damage_bonus));
        assert!(c.lines.iter().any(|l| matches!(l, Line::Stat(_, a, _) if *a == want)), "degats de l'epee : {want}");
    }
}
