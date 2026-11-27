//! Progression d'une partie : statistiques du joueur, armes et objets passifs possedes,
//! choix de niveau (1 parmi 3) et coffres (evolutions).
use crate::defs::{Mod, Stat, WeaponDef, WeaponStats};
use crate::world::World;

/// Statistiques du joueur (somme des objets passifs).
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub might: f32,
    pub area: f32,
    pub cooldown: f32,
    pub amount: u32,
    pub duration: f32,
    pub speed: f32,
    pub move_speed: f32,
    pub max_hp: f32,
    pub armor: f32,
    pub magnet: f32,
    pub dash_cd: f32,
    pub regen: f32,
    pub luck: f32,
    pub growth: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            might: 1.0,
            area: 1.0,
            cooldown: 1.0,
            amount: 0,
            duration: 1.0,
            speed: 1.0,
            move_speed: 1.0,
            max_hp: 0.0,
            armor: 0.0,
            magnet: 1.0,
            dash_cd: 1.0,
            regen: 0.0,
            luck: 1.0,
            growth: 1.0,
        }
    }
}

impl Stats {
    pub fn add(&mut self, s: Stat, v: f32) {
        match s {
            Stat::Might => self.might += v,
            Stat::Area => self.area += v,
            Stat::Cooldown => self.cooldown += v,
            Stat::Amount => self.amount += v.max(0.0) as u32,
            Stat::Duration => self.duration += v,
            Stat::Speed => self.speed += v,
            Stat::MoveSpeed => self.move_speed += v,
            Stat::MaxHp => self.max_hp += v,
            Stat::Armor => self.armor += v,
            Stat::Magnet => self.magnet += v,
            Stat::DashCooldown => self.dash_cd += v,
            Stat::Regen => self.regen += v,
            Stat::Luck => self.luck += v,
            Stat::Growth => self.growth += v,
        }
    }
}

/// Amelioration speciale d'une arme (recompense de rencontre) : suit l'arme quand elle evolue.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponBoost {
    /// Degats en plus (+0.25 = +25 %).
    pub damage: f32,
    /// Projectiles / coups en plus.
    pub amount: u32,
    /// Zone en plus (+0.2 = +20 %).
    pub area: f32,
    /// Recharge en moins (0.15 = -15 %).
    pub cooldown: f32,
}

impl WeaponBoost {
    pub fn apply(&self, w: &mut WeaponStats) {
        w.damage *= 1.0 + self.damage;
        w.amount += self.amount;
        w.area *= 1.0 + self.area;
        w.cooldown *= (1.0 - self.cooldown).max(0.3);
    }

    pub fn add(&mut self, o: WeaponBoost) {
        self.damage += o.damage;
        self.amount += o.amount;
        self.area += o.area;
        self.cooldown += o.cooldown;
    }
}

/// Arme possedee.
#[derive(Clone, Copy, Debug)]
pub struct WeaponSlot {
    pub id: u16,
    /// Ameliorations speciales cumulees (rencontres).
    pub boost: WeaponBoost,
    pub level: u32,
    /// Recharge restante (s).
    pub cd: f32,
    /// Caracteristiques effectives (niveau + statistiques du joueur).
    pub stats: WeaponStats,
    /// Orbite : temps actif restant et angle.
    pub active: f32,
    pub phase: f32,
}

/// Objet passif possede.
#[derive(Clone, Copy, Debug)]
pub struct PassiveSlot {
    pub id: u16,
    pub level: u32,
}

/// Poids d'une fusion parmi les choix de niveau (x poids de l'arme) : plus rare qu'une amelioration.
pub const EVOLVE_WEIGHT: f32 = 0.35;

/// Une proposition de niveau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    NewWeapon(u16),
    UpgradeWeapon(u16),
    NewPassive(u16),
    UpgradePassive(u16),
    /// Tout est au maximum : soin.
    Heal,
    /// Tout est au maximum : rubis.
    Rupees,
    /// Fusion prete (arme et objet partenaire au maximum) : l'arme evoluee (proposee plus rarement).
    Evolve(u16),
}

/// Contenu d'un coffre ouvert.
#[derive(Clone, Debug, Default)]
pub struct ChestReward {
    /// (arme d'origine, arme evoluee)
    pub evolution: Option<(u16, u16)>,
    pub upgrades: Vec<Offer>,
}

/// Caracteristiques d'une arme a un niveau donne, avec les statistiques du joueur.
pub fn resolve(def: &WeaponDef, level: u32, st: &Stats) -> WeaponStats {
    let mut w = def.base;
    let (mut cdm, mut aream, mut spm, mut durm, mut kbm) = (1.0f32, 1.0f32, 1.0f32, 1.0f32, 1.0f32);
    for lv in def.levels.iter().take(level.saturating_sub(1) as usize) {
        for m in lv {
            match *m {
                Mod::Damage(d) => w.damage += d,
                Mod::Cooldown(c) => cdm += c,
                Mod::Amount(n) => w.amount += n,
                Mod::Area(a) => aream += a,
                Mod::Speed(s) => spm += s,
                Mod::Duration(d) => durm += d,
                Mod::Pierce(p) => w.pierce += p,
                Mod::Knockback(k) => kbm += k,
                Mod::Crit(c) => w.crit_chance += c,
            }
        }
    }
    w.damage *= st.might;
    w.cooldown *= cdm.max(0.2) * st.cooldown.max(0.35);
    w.amount += st.amount;
    w.area *= aream * st.area;
    w.speed *= spm * st.speed;
    w.duration *= durm * st.duration;
    w.knockback *= kbm;
    w
}

impl World {
    pub fn add_weapon(&mut self, id: u16) {
        let def = &self.content.weapons[id as usize];
        let stats = resolve(def, 1, &self.player.stats);
        self.weapons.push(WeaponSlot { id, boost: WeaponBoost::default(), level: 1, cd: 0.4, stats, active: 0.0, phase: 0.0 });
    }

    fn add_passive(&mut self, id: u16) {
        self.passives.push(PassiveSlot { id, level: 1 });
        self.recompute_stats();
    }

    /// Recalcule les statistiques du joueur et des armes.
    pub fn recompute_stats(&mut self) {
        let mut st = Stats::default();
        st.might += self.content.rules.damage_bonus;
        for &(s, v) in &self.char_stats {
            st.add(s, v);
        }
        for &(s, v) in &self.mods.stats {
            st.add(s, v);
        }
        for p in &self.passives {
            let def = &self.content.passives[p.id as usize];
            for lv in def.levels.iter().take(p.level as usize) {
                for &(s, v) in lv {
                    st.add(s, v);
                }
            }
        }
        let pl = &mut self.player;
        let old_max = pl.max_hp;
        pl.stats = st;
        pl.max_hp = pl.base_hp + st.max_hp;
        if pl.max_hp > old_max {
            pl.hp += pl.max_hp - old_max;
        }
        pl.hp = pl.hp.min(pl.max_hp);
        pl.speed = pl.base_speed * st.move_speed;
        pl.magnet = pl.base_magnet * st.magnet;
        for w in &mut self.weapons {
            w.stats = resolve(&self.content.weapons[w.id as usize], w.level, &st);
            w.boost.apply(&mut w.stats);
        }
    }

    /// Bonus de statistiques gagnes pendant la partie (benedictions, reliques de rencontre).
    pub fn add_run_stats(&mut self, stats: &[(Stat, f32)]) {
        self.mods.stats.extend_from_slice(stats);
        self.recompute_stats();
    }

    /// Amelioration speciale d'une arme possedee.
    pub fn boost_weapon(&mut self, slot: usize, b: WeaponBoost) {
        if let Some(w) = self.weapons.get_mut(slot) {
            w.boost.add(b);
        }
        self.recompute_stats();
    }

    pub fn weapon_level(&self, id: u16) -> Option<u32> {
        self.weapons.iter().find(|w| w.id == id).map(|w| w.level)
    }

    pub fn passive_level(&self, id: u16) -> Option<u32> {
        self.passives.iter().find(|p| p.id == id).map(|p| p.level)
    }

    /// Toutes les propositions possibles avec leur poids.
    fn candidates(&self) -> Vec<(Offer, f32)> {
        let c = &self.content;
        let mut out = Vec::new();
        for w in &self.weapons {
            let def = &c.weapons[w.id as usize];
            if w.level < def.max_level() {
                out.push((Offer::UpgradeWeapon(w.id), def.weight * 1.3));
            }
        }
        if self.weapons.len() < c.rules.max_weapons {
            for (i, def) in c.weapons.iter().enumerate() {
                let id = i as u16;
                if def.evolved || self.locked_weapons.contains(&id) || self.weapons.iter().any(|w| w.id == id) {
                    continue;
                }
                // une arme dont l'evolution est deja possedee n'est plus proposee
                if self.weapons.iter().any(|w| def.recipes().any(|e| c.weapon_ids[w.id as usize] == e.into)) {
                    continue;
                }
                out.push((Offer::NewWeapon(id), def.weight));
            }
        }
        for p in &self.passives {
            let def = &c.passives[p.id as usize];
            if p.level < def.max_level() {
                out.push((Offer::UpgradePassive(p.id), def.weight * 1.1));
            }
        }
        if self.passives.len() < c.rules.max_passives {
            for (i, def) in c.passives.iter().enumerate() {
                let id = i as u16;
                if !self.passives.iter().any(|p| p.id == id) {
                    out.push((Offer::NewPassive(id), def.weight * 0.9));
                }
            }
        }
        // fusions pretes : plus rares qu'une amelioration ; seules en lice quand tout est au maximum
        // (elles passent donc avant les soins et les rubis)
        for (slot, into, _) in self.evolutions_ready() {
            if !out.iter().any(|(o, _)| *o == Offer::Evolve(into)) {
                out.push((Offer::Evolve(into), c.weapons[self.weapons[slot].id as usize].weight * EVOLVE_WEIGHT));
            }
        }
        out
    }

    /// Tire `n` propositions differentes (ponderees).
    fn draw_offers(&mut self, n: usize, owned_only: bool) -> Vec<Offer> {
        let mut pool = self.candidates();
        if owned_only {
            pool.retain(|(o, _)| matches!(o, Offer::UpgradeWeapon(_) | Offer::UpgradePassive(_)));
        }
        let mut out = Vec::new();
        while out.len() < n && !pool.is_empty() {
            let total: f32 = pool.iter().map(|p| p.1).sum();
            let mut r = self.rng.f() * total;
            let mut pick = pool.len() - 1;
            for (k, p) in pool.iter().enumerate() {
                if r < p.1 {
                    pick = k;
                    break;
                }
                r -= p.1;
            }
            out.push(pool.swap_remove(pick).0);
        }
        out
    }

    /// Propositions pour un niveau gagne.
    pub fn roll_offers(&mut self) -> Vec<Offer> {
        let n = self.content.rules.choices;
        let mut out = self.draw_offers(n, false);
        if out.is_empty() {
            out = vec![Offer::Heal, Offer::Rupees];
        }
        out
    }

    pub fn apply_offer(&mut self, o: Offer) {
        match o {
            Offer::NewWeapon(id) => {
                if self.weapons.len() < self.content.rules.max_weapons {
                    self.add_weapon(id);
                }
            }
            Offer::UpgradeWeapon(id) => {
                if let Some(w) = self.weapons.iter_mut().find(|w| w.id == id) {
                    w.level += 1;
                }
                self.recompute_stats();
            }
            Offer::NewPassive(id) => self.add_passive(id),
            Offer::UpgradePassive(id) => {
                if let Some(p) = self.passives.iter_mut().find(|p| p.id == id) {
                    p.level += 1;
                }
                self.recompute_stats();
            }
            Offer::Heal => {
                let h = self.content.rules.fallback_heal;
                self.player.hp = (self.player.hp + h).min(self.player.max_hp);
            }
            Offer::Rupees => {
                let r = self.content.rules.fallback_rupees;
                self.player.rupees += r;
            }
            Offer::Evolve(into) => {
                if let Some((slot, _, _)) = self.evolutions_ready().into_iter().find(|e| e.1 == into) {
                    self.evolve_slot(slot, into);
                }
            }
        }
    }

    /// Fait evoluer l'arme de l'emplacement `slot` en `into` (l'objet partenaire est alors utilise).
    /// Rend l'arme d'origine.
    fn evolve_slot(&mut self, slot: usize, into: u16) -> u16 {
        let from = self.weapons[slot].id;
        let boost = self.weapons[slot].boost;
        let def = &self.content.weapons[into as usize];
        let mut stats = resolve(def, 1, &self.player.stats);
        boost.apply(&mut stats);
        self.weapons[slot] = WeaponSlot { id: into, boost, level: 1, cd: 0.2, stats, active: 0.0, phase: 0.0 };
        let into_id = &self.content.weapon_ids[into as usize];
        let recipe = self.content.weapons[from as usize].recipes().find(|e| &e.into == into_id);
        if let Some(pid) = recipe.and_then(|e| self.content.passive_index(&e.passive)) {
            self.used_partners.push(pid);
        }
        from
    }

    /// Fusions possibles pour l'arme `k` dans cette partie : la normale, et l'ultime si elle est
    /// debloquee.
    pub fn recipes_of(&self, k: u16) -> Vec<&crate::defs::EvolutionDef> {
        let c = &self.content;
        c.weapons[k as usize]
            .recipes()
            .filter(|e| c.weapon_index(&e.into).is_some_and(|i| !self.locked_weapons.contains(&i)))
            .collect()
    }

    /// L'objet partenaire `pid` peut-il encore servir a une fusion (pas deja utilise dans cette partie) ?
    pub fn partner_free(&self, pid: u16) -> bool {
        !self.used_partners.contains(&pid)
    }

    /// Evolution possible : (emplacement, arme evoluee). Retour 0.6.1 : l'arme ET son objet
    /// partenaire doivent etre tous deux au niveau maximal.
    pub fn evolution_ready(&self) -> Option<(usize, u16)> {
        self.evolutions_ready().first().map(|&(slot, into, _)| (slot, into))
    }

    /// Choix a proposer au joueur : les armes pretes qui partagent l'objet partenaire de la
    /// premiere (un seul fusionnera). Moins de deux : rien a choisir.
    pub fn evolution_choices(&self) -> Vec<(usize, u16)> {
        let all = self.evolutions_ready();
        let Some(&(_, _, pid)) = all.first() else { return Vec::new() };
        all.into_iter().filter(|e| e.2 == pid).map(|(slot, into, _)| (slot, into)).collect()
    }

    /// Toutes les evolutions possibles : (emplacement, arme evoluee).
    pub fn evolution_choices_all(&self) -> Vec<(usize, u16)> {
        self.evolutions_ready().into_iter().map(|(slot, into, _)| (slot, into)).collect()
    }

    /// Toutes les evolutions possibles : (emplacement, arme evoluee, objet partenaire).
    fn evolutions_ready(&self) -> Vec<(usize, u16, u16)> {
        let c = &self.content;
        let mut out = Vec::new();
        for (slot, w) in self.weapons.iter().enumerate() {
            let def = &c.weapons[w.id as usize];
            if w.level < def.max_level() {
                continue;
            }
            // fusion ultime d'abord : elle passe avant la fusion normale quand les deux sont pretes
            let mut recipes = self.recipes_of(w.id);
            recipes.reverse();
            for evo in recipes {
                let Some(pid) = c.passive_index(&evo.passive) else { continue };
                // un objet partenaire ne sert qu'a une seule fusion par partie
                if self.used_partners.contains(&pid) {
                    continue;
                }
                if self.passive_level(pid).is_none_or(|l| l < c.passives[pid as usize].max_level()) {
                    continue;
                }
                if let Some(into) = c.weapon_index(&evo.into) {
                    out.push((slot, into, pid));
                }
            }
        }
        out
    }

    /// Ouvre un coffre : evolution si possible, sinon des ameliorations.
    pub fn open_chest(&mut self, big: bool) -> ChestReward {
        self.open_chest_choice(big, None)
    }

    /// Ouvre un coffre en faisant evoluer l'arme de l'emplacement `pick` (choisie par le joueur
    /// parmi `evolution_choices`) ; sans choix, la premiere arme prete.
    pub fn open_chest_choice(&mut self, big: bool, pick: Option<usize>) -> ChestReward {
        let mut r = ChestReward::default();
        let chosen = pick.and_then(|s| self.evolution_choices().into_iter().find(|e| e.0 == s));
        if let Some((slot, into)) = chosen.or_else(|| self.evolution_ready()) {
            let from = self.evolve_slot(slot, into);
            r.evolution = Some((from, into));
        }
        let (lo, hi) = self.content.rules.chest_upgrades;
        let n = if big {
            hi
        } else if r.evolution.is_some() {
            0
        } else {
            // 1 le plus souvent, parfois plus
            let roll = self.rng.f();
            let k = if roll < 0.7 {
                lo
            } else if roll < 0.93 {
                lo + 1
            } else {
                hi
            };
            k.clamp(lo, hi)
        };
        for _ in 0..n {
            // une amelioration a la fois : la suivante tient compte de la precedente
            let pick = self.draw_offers(1, true);
            match pick.first() {
                Some(&o) => {
                    self.apply_offer(o);
                    r.upgrades.push(o);
                }
                None => {
                    self.apply_offer(Offer::Rupees);
                    r.upgrades.push(Offer::Rupees);
                }
            }
        }
        self.recompute_stats();
        r
    }
}
