//! Les armes du joueur : declenchement selon la famille, projectiles, zones au sol,
//! orbites, explosions. Toutes les touches passent par `World::strike`.
use crate::defs::{StatusDef, WeaponKind};
use crate::horde::{SLOT_MISC, SLOT_ZONE};
use crate::world::{Event, PlayerInput, Swing, World, DT};
use glam::Vec2;
use std::f32::consts::{PI, TAU};

/// Trajectoire d'un projectile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjKind {
    /// Ligne droite (fleches, boules de feu, faisceaux).
    Straight,
    /// Aller puis retour vers le joueur.
    Boomerang,
    /// Lancer en cloche, puis meche, puis explosion.
    Lob,
    /// Bloc pose au sol (Somaria) : blesse au contact puis eclate en rayons.
    Block,
    /// Poursuit l'ennemi le plus proche en zigzaguant (abeilles) ; `flight` = vitesse.
    Homing,
    /// Tornade : erre lentement en ondulant, traverse tout (Ocarina) ; `flight` = phase de l'ondulation.
    Tornado,
}

#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub kind: ProjKind,
    pub weapon: u16,
    /// Emplacement de degats (intervalle entre deux touches).
    pub slot: u8,
    pub damage: f32,
    pub crit: (f32, f32),
    pub knockback: f32,
    pub interval: f32,
    /// Rayon de touche.
    pub radius: f32,
    /// Touches restantes (u32::MAX = illimite).
    pub pierce: u32,
    /// Vie (s) : trajet, aller du boomerang, meche de la bombe.
    pub life: f32,
    pub age: f32,
    /// Angle d'affichage.
    pub rot: f32,
    /// Hauteur (bombes) et duree du vol.
    pub z: f32,
    pub flight: f32,
    pub start: Vec2,
    pub target: Vec2,
    /// Rayon d'explosion (bombes).
    pub blast: f32,
    pub returning: bool,
    /// Faisceau d'epee (sprite propre).
    pub beam: bool,
    pub dead: bool,
}

/// Zone au sol : flammes persistantes, colonnes de feu (avec delai et frappe initiale).
#[derive(Clone, Copy, Debug)]
pub struct Zone {
    pub pos: Vec2,
    pub radius: f32,
    /// Duree apres activation.
    pub life: f32,
    pub age: f32,
    pub delay: f32,
    pub dps: f32,
    /// Degats a l'activation (colonnes).
    pub blast: f32,
    pub knockback: f32,
    pub weapon: u16,
    pub slot: u8,
    pub status: Option<StatusDef>,
    pub fired: bool,
    /// Colonne (Bombos) plutot que flamme au sol.
    pub pillar: bool,
}

/// Projectile ennemi (pierre, fleche, lance, bombe).
#[derive(Clone, Copy, Debug)]
pub struct EnemyShot {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    pub damage: f32,
    pub radius: f32,
    pub life: f32,
    pub age: f32,
    /// Index dans `World::shot_sprites`.
    pub sprite: u16,
    /// Lance en cloche : hauteur, duree du vol, cible, explosion.
    pub lob: bool,
    pub z: f32,
    pub flight: f32,
    pub start: Vec2,
    pub target: Vec2,
    pub dead: bool,
}

/// Angle (radians) d'un vecteur.
#[inline]
fn ang(v: Vec2) -> f32 {
    libm::atan2f(v.y, v.x)
}

impl World {
    /// Les `n` ennemis les plus proches de `from` a moins de `range` (du plus proche au plus loin).
    pub(crate) fn nearest_enemies(&self, from: Vec2, range: f32, n: usize) -> Vec<usize> {
        let mut best: Vec<(f32, usize)> = Vec::with_capacity(n + 1);
        let r2 = range * range;
        let horde = &self.horde;
        self.grid.query(from, range, |i| {
            if horde.hp[i] <= 0.0 {
                return;
            }
            let d = (horde.pos[i] - from).length_squared();
            if d > r2 {
                return;
            }
            if best.len() < n || d < best[best.len() - 1].0 {
                let at = best.partition_point(|b| b.0 < d);
                best.insert(at, (d, i));
                if best.len() > n {
                    best.pop();
                }
            }
        });
        best.into_iter().map(|b| b.1).collect()
    }

    /// Ennemis a moins de `range` du joueur, `n` au hasard.
    fn random_enemies(&mut self, range: f32, n: usize) -> Vec<usize> {
        let pp = self.player.pos;
        let r2 = range * range;
        let mut all = Vec::new();
        let horde = &self.horde;
        self.grid.query(pp, range, |i| {
            if horde.hp[i] > 0.0 && (horde.pos[i] - pp).length_squared() < r2 {
                all.push(i);
            }
        });
        let mut out = Vec::new();
        while out.len() < n && !all.is_empty() {
            let k = self.rng.below(all.len() as u32) as usize;
            out.push(all.swap_remove(k));
        }
        out
    }

    /// Tous les ennemis a moins de `r` d'un point.
    fn enemies_in(&self, at: Vec2, r: f32) -> Vec<usize> {
        let mut out = Vec::new();
        let horde = &self.horde;
        self.grid.query(at, r + 12.0, |i| {
            if horde.hp[i] > 0.0 && (horde.pos[i] - at).length() < r + horde.radius[i] {
                out.push(i);
            }
        });
        out
    }

    pub(crate) fn update_weapons(&mut self, input: &PlayerInput) {
        // a terre, ou en plein choix (coop) : aucune attaque, orbites rangees
        if !self.player.alive || self.player.choosing() {
            self.player.swing = None;
            self.weapons.iter_mut().for_each(|w| w.active = 0.0);
            return;
        }
        self.update_swing();
        self.update_jump();
        for slot in 0..self.weapons.len() {
            let w = self.weapons[slot];
            let kind = self.content.weapons[w.id as usize].kind;
            if kind == WeaponKind::Orbit {
                let ws = &mut self.weapons[slot];
                ws.phase = (ws.phase + w.stats.speed * DT) % TAU;
                ws.active = (ws.active - DT).max(0.0);
            }
            self.weapons[slot].cd -= DT;
            if self.weapons[slot].cd > 0.0 {
                continue;
            }
            let fired = match kind {
                WeaponKind::Swing => self.try_swing(slot, input),
                WeaponKind::Projectile => self.fire_projectiles(slot),
                WeaponKind::Boomerang => self.fire_boomerangs(slot),
                WeaponKind::Lob => self.fire_bombs(slot),
                WeaponKind::Slam => self.fire_slam(slot),
                WeaponKind::Orbit => {
                    self.weapons[slot].active = w.stats.duration;
                    true
                }
                WeaponKind::Pillars => self.fire_pillars(slot),
                WeaponKind::Storm => self.fire_storm(slot),
                WeaponKind::Aura => self.fire_aura(slot),
                WeaponKind::Somaria => self.fire_somaria(slot),
                WeaponKind::Bees => self.fire_bees(slot),
                WeaponKind::Jump => self.fire_jump(slot),
                WeaponKind::Tornado => self.fire_tornadoes(slot),
            };
            if fired {
                self.weapons[slot].cd = w.stats.cooldown;
                if kind != WeaponKind::Swing {
                    self.events.push(Event::Fire { pos: self.player.pos, weapon: w.id });
                }
            }
        }
        self.update_orbits();
    }

    /// Epee : choisit la direction (cible automatique ou visee) et lance le coup.
    fn try_swing(&mut self, slot: usize, input: &PlayerInput) -> bool {
        if self.player.swing.is_some() {
            return false;
        }
        let w = self.weapons[slot];
        let def = &self.content.weapons[w.id as usize];
        let p = &self.player;
        let dir = if p.auto_attack {
            let range = def.range.max(w.stats.area);
            let near = self.nearest_enemies(p.pos, range, 1);
            near.first().map(|&i| self.horde.pos[i] - p.pos)
        } else if input.attack {
            Some(input.aim.filter(|a| a.length_squared() > 0.01).unwrap_or(p.facing))
        } else {
            None
        };
        let Some(dir) = dir else { return false };
        let dir = if dir.length_squared() > 1e-6 { dir.normalize() } else { self.player.facing };
        self.player.swing_queue = w.stats.amount.saturating_sub(1);
        self.start_swing(slot, ang(dir));
        true
    }

    fn start_swing(&mut self, slot: usize, angle: f32) {
        let w = self.weapons[slot];
        let def = &self.content.weapons[w.id as usize];
        let dir = Vec2::from_angle(angle);
        let p = &mut self.player;
        p.facing = dir;
        p.face = crate::horde::face_from(dir, p.face);
        p.swing = Some(Swing {
            id: p.next_swing,
            slot: slot as u8,
            weapon: w.id,
            t: 0.0,
            dur: w.stats.duration.max(0.06),
            dir: angle,
            arc: def.arc.to_radians(),
            radius: w.stats.area,
            sign: p.swing_sign,
            hits: 0,
        });
        p.next_swing = p.next_swing.wrapping_add(1);
        p.swing_sign = -p.swing_sign;
        let pos = p.pos;
        self.events.push(Event::Swing { pos, angle, weapon: w.id });
        // faisceau de l'Epee de legende
        if let Some(b) = def.beam.clone() {
            let speed = b.speed * self.player.stats.speed;
            self.projectiles.push(Projectile {
                pos: pos - Vec2::new(0.0, 8.0) + dir * 10.0,
                prev: pos - Vec2::new(0.0, 8.0),
                vel: dir * speed,
                kind: ProjKind::Straight,
                weapon: w.id,
                slot: SLOT_MISC as u8,
                damage: w.stats.damage * b.damage,
                crit: (w.stats.crit_chance, w.stats.crit_mult),
                knockback: w.stats.knockback * 0.5,
                interval: 0.25,
                radius: 9.0 * self.player.stats.area,
                pierce: u32::MAX,
                life: b.life * self.player.stats.duration,
                age: 0.0,
                rot: angle,
                z: 0.0,
                flight: 0.0,
                start: pos,
                target: pos,
                blast: 0.0,
                returning: false,
                beam: true,
                dead: false,
            });
        }
    }

    /// Progression du coup d'epee : balayage et touches.
    fn update_swing(&mut self) {
        let Some(mut sw) = self.player.swing else { return };
        sw.t += DT;
        let slot = sw.slot as usize;
        let Some(w) = self.weapons.get(slot).copied() else {
            self.player.swing = None;
            return;
        };
        let a_start = sw.angle_at(0.0);
        let a_now = sw.angle_at(sw.t);
        let (lo, hi) = if a_start < a_now { (a_start, a_now) } else { (a_now, a_start) };
        let tol = 0.2;
        let pp = self.player.pos;
        let reach = sw.radius + 12.0;
        let mut cands: Vec<(f32, usize)> = Vec::new();
        {
            let horde = &self.horde;
            self.grid.query(pp, reach, |i| {
                if horde.hp[i] <= 0.0 {
                    return;
                }
                let d = horde.pos[i] - pp;
                let dist = d.length();
                if dist > sw.radius + horde.radius[i] {
                    return;
                }
                let close = dist < 9.0;
                let mut rel = ang(d) - sw.dir;
                while rel > PI {
                    rel -= TAU;
                }
                while rel < -PI {
                    rel += TAU;
                }
                let aa = sw.dir + rel;
                if close || (aa >= lo - tol && aa <= hi + tol) {
                    cands.push((dist, i));
                }
            });
        }
        // les plus proches d'abord : la limite de cibles protege le devant de Link
        cands.sort_by(|a, b| a.0.total_cmp(&b.0));
        let max_hits = if w.stats.pierce == 0 { u32::MAX } else { w.stats.pierce };
        let interval = sw.dur + 0.02;
        for (_, i) in cands {
            if sw.hits >= max_hits {
                break;
            }
            let d = self.horde.pos[i] - pp;
            let dir = if d.length_squared() > 1e-6 { d.normalize() } else { Vec2::from_angle(sw.dir) };
            let status = self.content.weapons[w.id as usize].status;
            if self.strike(i, slot, w.id, w.stats.damage, (w.stats.crit_chance, w.stats.crit_mult), dir, w.stats.knockback, interval, status) {
                sw.hits += 1;
            }
        }
        if sw.t >= sw.dur {
            self.player.swing = None;
            if self.player.swing_queue > 0 {
                self.player.swing_queue -= 1;
                // coup suivant : derriere Link
                self.start_swing(slot, sw.dir + PI);
            }
        } else {
            self.player.swing = Some(sw);
        }
    }

    fn base_projectile(&self, slot: usize, pos: Vec2, vel: Vec2, kind: ProjKind) -> Projectile {
        let w = self.weapons[slot];
        Projectile {
            pos,
            prev: pos,
            vel,
            kind,
            weapon: w.id,
            slot: slot as u8,
            damage: w.stats.damage,
            crit: (w.stats.crit_chance, w.stats.crit_mult),
            knockback: w.stats.knockback,
            interval: w.stats.hit_interval,
            radius: w.stats.area,
            pierce: if w.stats.pierce == 0 { u32::MAX } else { w.stats.pierce },
            life: w.stats.duration,
            age: 0.0,
            rot: ang(vel),
            z: 0.0,
            flight: 0.0,
            start: pos,
            target: pos,
            blast: 0.0,
            returning: false,
            beam: false,
            dead: false,
        }
    }

    /// Arc, baguettes : un projectile par cible proche, les surplus en eventail.
    fn fire_projectiles(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let def = &self.content.weapons[w.id as usize];
        let (range, spread) = (def.range, def.arc.to_radians());
        let origin = self.player.pos - Vec2::new(0.0, 8.0);
        let n = w.stats.amount.max(1) as usize;
        let targets = self.nearest_enemies(self.player.pos, range, n);
        let Some(&first) = targets.first() else { return false };
        let base_dir = (self.horde.pos[first] - origin).normalize_or(self.player.facing);
        let fan = def.fan;
        for k in 0..n {
            // lance-pierre : tout l'eventail vers la cible la plus proche
            let fan_dir = Vec2::from_angle(ang(base_dir) + (k as f32 - (n as f32 - 1.0) * 0.5) * spread);
            let dir = if fan { Some(fan_dir) } else { None };
            let dir = dir.unwrap_or_else(|| match targets.get(k) {
                Some(&i) => (self.horde.pos[i] - Vec2::new(0.0, 4.0) - origin).normalize_or(base_dir),
                None => {
                    let side = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let step = k.div_ceil(2) as f32;
                    Vec2::from_angle(ang(base_dir) + side * step * spread.max(0.15))
                }
            });
            let p = self.base_projectile(slot, origin + dir * 6.0, dir * w.stats.speed, ProjKind::Straight);
            self.projectiles.push(p);
        }
        true
    }

    /// Boomerangs : vers les ennemis proches (ou devant), retour au joueur.
    fn fire_boomerangs(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let range = self.content.weapons[w.id as usize].range;
        let n = w.stats.amount.max(1) as usize;
        let targets = self.nearest_enemies(self.player.pos, range, n);
        if targets.is_empty() {
            return false;
        }
        let origin = self.player.pos - Vec2::new(0.0, 8.0);
        for k in 0..n {
            let dir = match targets.get(k) {
                Some(&i) => (self.horde.pos[i] - origin).normalize_or(self.player.facing),
                None => Vec2::from_angle(ang(self.player.facing) + k as f32 * TAU / n as f32),
            };
            let p = self.base_projectile(slot, origin, dir * w.stats.speed, ProjKind::Boomerang);
            self.projectiles.push(p);
        }
        true
    }

    /// Bombes : lancees en cloche sur des ennemis proches.
    fn fire_bombs(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let range = self.content.weapons[w.id as usize].range;
        let n = w.stats.amount.max(1) as usize;
        let targets = self.random_enemies(range, n);
        if targets.is_empty() {
            return false;
        }
        let origin = self.player.pos;
        for k in 0..n {
            let target = match targets.get(k) {
                Some(&i) => self.horde.pos[i] + self.rng.dir() * self.rng.range(0.0, 10.0),
                None => origin + self.rng.dir() * self.rng.range(30.0, range * 0.6),
            };
            let flight = 0.42 + self.rng.range(0.0, 0.12);
            let mut p = self.base_projectile(slot, origin - Vec2::new(0.0, 4.0), (target - origin) / flight, ProjKind::Lob);
            p.flight = flight;
            p.start = origin;
            p.target = target;
            p.blast = w.stats.area;
            p.radius = 0.0;
            p.interval = 0.05;
            p.rot = 0.0;
            self.projectiles.push(p);
        }
        true
    }

    /// Marteau, Quake : frappe autour du joueur (si un ennemi est a portee).
    fn fire_slam(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let pp = self.player.pos;
        let r = w.stats.area;
        let hit = self.enemies_in(pp, r);
        if hit.is_empty() {
            return false;
        }
        self.slam_at(slot, pp, hit);
        true
    }

    /// Frappe au sol : degats pleins au centre, 60 % au bord ; flammes au sol si l'arme en laisse (Feu de Din).
    fn slam_at(&mut self, slot: usize, pp: Vec2, hit: Vec<usize>) {
        let w = self.weapons[slot];
        let r = w.stats.area;
        let (status, zone) = {
            let def = &self.content.weapons[w.id as usize];
            (def.status, def.zone)
        };
        let mut flames = 0;
        for i in hit {
            let d = self.horde.pos[i] - pp;
            let dir = d.normalize_or(Vec2::Y);
            let f = 1.0 - 0.4 * (d.length() / r).min(1.0);
            if self.strike(i, slot, w.id, w.stats.damage * f, (w.stats.crit_chance, w.stats.crit_mult), dir, w.stats.knockback * f, 0.1, status) {
                // au plus 8 flammes par frappe (la foule en laisserait des dizaines)
                if let Some(z) = zone.filter(|_| flames < 8) {
                    flames += 1;
                    self.add_flame(self.horde.pos[i], z.radius, z.time, z.dps, w.id);
                }
            }
        }
        self.events.push(Event::Slam { pos: pp, radius: r, weapon: w.id });
    }

    /// Plume de Roc : saute (invincible) quand un ennemi est proche.
    fn fire_jump(&mut self, slot: usize) -> bool {
        if self.player.jump > 0.0 {
            return false;
        }
        let w = self.weapons[slot];
        let range = self.content.weapons[w.id as usize].range;
        if self.nearest_enemies(self.player.pos, range, 1).is_empty() {
            return false;
        }
        let dur = w.stats.duration.max(0.2);
        let p = &mut self.player;
        p.jump = dur;
        p.jump_dur = dur;
        p.jump_slot = slot as u8;
        p.invuln = p.invuln.max(dur + 0.05);
        true
    }

    /// Saut en cours : a l'atterrissage, onde de choc autour du joueur.
    fn update_jump(&mut self) {
        if self.player.jump <= 0.0 {
            return;
        }
        self.player.jump -= DT;
        if self.player.jump > 0.0 {
            return;
        }
        self.player.jump = 0.0;
        let slot = self.player.jump_slot as usize;
        if slot >= self.weapons.len() || self.content.weapons[self.weapons[slot].id as usize].kind != WeaponKind::Jump {
            return;
        }
        let pp = self.player.pos;
        let hit = self.enemies_in(pp, self.weapons[slot].stats.area);
        self.slam_at(slot, pp, hit);
    }

    /// Ocarina : tornades lancees vers des ennemis a l'ecran, qui errent ensuite.
    fn fire_tornadoes(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let range = self.content.weapons[w.id as usize].range;
        let n = w.stats.amount.max(1) as usize;
        let targets = self.random_enemies(range, n);
        if targets.is_empty() {
            return false;
        }
        let origin = self.player.pos - Vec2::new(0.0, 6.0);
        for k in 0..n {
            let dir = match targets.get(k) {
                Some(&i) => (self.horde.pos[i] - origin).normalize_or(self.player.facing),
                None => self.rng.dir(),
            };
            let mut p = self.base_projectile(slot, origin + dir * 8.0, dir * w.stats.speed, ProjKind::Tornado);
            p.pierce = u32::MAX;
            p.flight = self.rng.f() * TAU;
            p.rot = 0.0;
            self.projectiles.push(p);
        }
        true
    }

    /// Bombos : colonnes de feu sur des ennemis a l'ecran, en cascade.
    fn fire_pillars(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let def = &self.content.weapons[w.id as usize];
        let (range, status, zone) = (def.range, def.status, def.zone);
        let n = w.stats.amount.max(1) as usize;
        let targets = self.random_enemies(range, n);
        if targets.is_empty() {
            return false;
        }
        for (k, &i) in targets.iter().enumerate() {
            let pos = self.horde.pos[i];
            self.zones.push(Zone {
                pos,
                radius: w.stats.area,
                life: zone.map(|z| z.time).unwrap_or(0.6) * self.player.stats.duration,
                age: 0.0,
                delay: 0.07 * k as f32,
                dps: zone.map(|z| z.dps).unwrap_or(0.0) * self.player.stats.might,
                blast: w.stats.damage,
                knockback: w.stats.knockback,
                weapon: w.id,
                slot: slot as u8,
                status,
                fired: false,
                pillar: true,
            });
        }
        true
    }

    /// Ether : tout l'ecran est frappe et gele.
    fn fire_storm(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let def = &self.content.weapons[w.id as usize];
        let (range, status) = (def.range, def.status);
        let pp = self.player.pos;
        let hit = self.enemies_in(pp, range);
        if hit.is_empty() {
            return false;
        }
        for i in hit {
            let dir = (self.horde.pos[i] - pp).normalize_or(Vec2::Y);
            self.strike(i, slot, w.id, w.stats.damage, (w.stats.crit_chance, w.stats.crit_mult), dir, w.stats.knockback, 0.1, status);
        }
        self.events.push(Event::Storm { pos: pp, radius: range, weapon: w.id });
        true
    }

    /// Poudre magique : blesse tout ce qui est dans le nuage autour du joueur.
    fn fire_aura(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let pp = self.player.pos - Vec2::new(0.0, 4.0);
        let status = self.content.weapons[w.id as usize].status;
        for i in self.enemies_in(pp, w.stats.area) {
            let dir = (self.horde.pos[i] - pp).normalize_or(Vec2::Y);
            self.strike(i, slot, w.id, w.stats.damage, (w.stats.crit_chance, w.stats.crit_mult), dir, w.stats.knockback, w.stats.hit_interval, status);
        }
        true
    }

    /// Canne de Somaria : pose des blocs sur des ennemis proches.
    fn fire_somaria(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let range = self.content.weapons[w.id as usize].range;
        let n = w.stats.amount.max(1) as usize;
        let targets = self.random_enemies(range, n);
        if targets.is_empty() {
            return false;
        }
        for k in 0..n {
            let at = match targets.get(k) {
                Some(&i) => self.horde.pos[i],
                None => self.player.pos + self.rng.dir() * self.rng.range(30.0, 70.0),
            };
            // le point de touche est 6 px sous la position : le bloc couvre l'ennemi
            let mut p = self.base_projectile(slot, at - Vec2::new(0.0, 6.0), Vec2::ZERO, ProjKind::Block);
            p.rot = 0.0;
            p.radius = 9.0 * self.player.stats.area.sqrt();
            self.projectiles.push(p);
        }
        true
    }

    /// Le bloc eclate : rayons dans toutes les directions.
    fn somaria_burst(&mut self, p: &Projectile) {
        let def = &self.content.weapons[p.weapon as usize];
        let Some(b) = def.beam.clone() else { return };
        let n = def.rays.max(1);
        let st = self.player.stats;
        for k in 0..n {
            let a = k as f32 * TAU / n as f32;
            let dir = Vec2::from_angle(a);
            self.projectiles.push(Projectile {
                pos: p.pos + dir * 4.0,
                prev: p.pos,
                vel: dir * b.speed * st.speed,
                kind: ProjKind::Straight,
                weapon: p.weapon,
                slot: p.slot,
                damage: p.damage * b.damage,
                crit: p.crit,
                knockback: p.knockback * 0.6,
                interval: 0.3,
                radius: 7.0 * st.area,
                pierce: u32::MAX,
                life: b.life * st.duration,
                age: 0.0,
                rot: a,
                z: 0.0,
                flight: 0.0,
                start: p.pos,
                target: p.pos,
                blast: 0.0,
                returning: false,
                beam: true,
                dead: false,
            });
        }
        self.events.push(Event::Burst { pos: p.pos, weapon: p.weapon });
    }

    /// Abeilles : s'envolent autour du joueur puis piquent les ennemis.
    fn fire_bees(&mut self, slot: usize) -> bool {
        let w = self.weapons[slot];
        let range = self.content.weapons[w.id as usize].range;
        if self.nearest_enemies(self.player.pos, range, 1).is_empty() {
            return false;
        }
        let origin = self.player.pos - Vec2::new(0.0, 8.0);
        for _ in 0..w.stats.amount.max(1) {
            let dir = self.rng.dir();
            let mut p = self.base_projectile(slot, origin + dir * 4.0, dir * w.stats.speed * 0.7, ProjKind::Homing);
            p.flight = w.stats.speed;
            // phase du zigzag
            p.target = Vec2::new(self.rng.f() * TAU, 0.0);
            self.projectiles.push(p);
        }
        true
    }


    fn update_orbits(&mut self) {
        for slot in 0..self.weapons.len() {
            let w = self.weapons[slot];
            let def = &self.content.weapons[w.id as usize];
            if def.kind != WeaponKind::Orbit || w.active <= 0.0 {
                continue;
            }
            let status = def.status;
            let center = self.player.pos - Vec2::new(0.0, 6.0);
            let pts = w.orbit_points(center, w.phase);
            let hit_r = 7.0 * self.player.stats.area.sqrt();
            for pt in pts {
                for i in self.enemies_in(pt, hit_r) {
                    let dir = (self.horde.pos[i] - center).normalize_or(Vec2::Y);
                    self.strike(i, slot, w.id, w.stats.damage, (w.stats.crit_chance, w.stats.crit_mult), dir, w.stats.knockback, w.stats.hit_interval, status);
                }
            }
        }
    }

    pub(crate) fn update_projectiles(&mut self) {
        let mut list = std::mem::take(&mut self.projectiles);
        let pp = self.player.pos - Vec2::new(0.0, 8.0);
        for p in list.iter_mut() {
            p.prev = p.pos;
            p.age += DT;
            match p.kind {
                ProjKind::Straight => {
                    p.pos += p.vel * DT;
                    if p.age >= p.life {
                        p.dead = true;
                        self.projectile_end(p);
                    }
                }
                ProjKind::Boomerang => {
                    let def = &self.content.weapons[p.weapon as usize];
                    p.rot += def.spin * DT;
                    if !p.returning {
                        // ralentit jusqu'a l'arret, puis revient
                        let u = (p.age / p.life.max(0.05)).min(1.0);
                        p.pos += p.vel * (1.0 - u) * 1.6 * DT;
                        if u >= 1.0 {
                            p.returning = true;
                            p.age = 0.0;
                        }
                    } else {
                        let to = pp - p.pos;
                        let d = to.length();
                        let sp = p.vel.length() * (0.6 + p.age * 2.5).min(1.6);
                        p.pos += to / d.max(0.001) * sp * DT;
                        if d < 8.0 || p.age > 4.0 {
                            p.dead = true;
                        }
                    }
                }
                ProjKind::Lob => {
                    if p.age < p.flight {
                        let u = p.age / p.flight;
                        p.pos = p.start.lerp(p.target, u) - Vec2::new(0.0, 4.0);
                        p.z = 4.0 * u * (1.0 - u) * 36.0;
                    } else {
                        p.z = 0.0;
                        p.pos = p.target;
                        if p.age >= p.flight + p.life {
                            p.dead = true;
                            let (pos, r, dmg, kb, w, slot) = (p.pos, p.blast, p.damage, p.knockback, p.weapon, p.slot);
                            self.explode(pos, r, dmg, kb, w, slot as usize, p.crit);
                        }
                    }
                    continue;
                }
                ProjKind::Block => {
                    if p.age >= p.life {
                        p.dead = true;
                        self.somaria_burst(p);
                    }
                }
                ProjKind::Tornado => {
                    // ondule autour de sa direction et tourne sur elle-meme
                    let a = libm::sinf(p.age * 1.7 + p.flight) * 1.4 * DT;
                    p.vel = Vec2::from_angle(ang(p.vel) + a) * p.vel.length();
                    p.pos += p.vel * DT;
                    p.rot += 9.0 * DT;
                    if p.age >= p.life {
                        p.dead = true;
                    }
                }
                ProjKind::Homing => {
                    let near = self.nearest_enemies(p.pos, 150.0, 1);
                    let fwd = p.vel.normalize_or(Vec2::X);
                    let want = match near.first() {
                        Some(&i) => (self.horde.pos[i] - Vec2::new(0.0, 6.0) - p.pos).normalize_or(fwd),
                        None => fwd,
                    };
                    let wob = Vec2::from_angle(ang(want) + libm::sinf(p.age * 9.0 + p.target.x) * 0.8);
                    p.vel += (wob * p.flight - p.vel) * (6.0 * DT);
                    p.pos += p.vel * DT;
                    p.rot = ang(p.vel);
                    if p.age >= p.life {
                        p.dead = true;
                    }
                }
            }
            if p.dead || p.radius <= 0.0 {
                continue;
            }
            // touches
            let mut hits: Vec<usize> = Vec::new();
            {
                let horde = &self.horde;
                let (pos, r) = (p.pos + Vec2::new(0.0, 6.0), p.radius);
                self.grid.query(pos, r + 12.0, |i| {
                    if horde.hp[i] > 0.0 && (horde.pos[i] - pos).length() < r + horde.radius[i] {
                        hits.push(i);
                    }
                });
            }
            let (status, zone) = {
                let def = &self.content.weapons[p.weapon as usize];
                (if p.beam { None } else { def.status }, if p.beam { None } else { def.zone })
            };
            for i in hits {
                let dir = p.vel.normalize_or(Vec2::Y);
                if self.strike(i, p.slot as usize, p.weapon, p.damage, p.crit, dir, p.knockback, p.interval, status) {
                    if let Some(z) = zone {
                        self.add_flame(self.horde.pos[i], z.radius, z.time, z.dps, p.weapon);
                    }
                    if p.pierce != u32::MAX {
                        p.pierce = p.pierce.saturating_sub(1);
                        if p.pierce == 0 {
                            p.dead = true;
                            break;
                        }
                    }
                }
            }
        }
        list.retain(|p| !p.dead);
        // projectiles tires pendant la mise a jour (rayons de Somaria) + liste traitee
        list.append(&mut self.projectiles);
        self.projectiles = list;
    }

    /// Fin de course d'un projectile droit (boule de feu : laisse une flamme).
    fn projectile_end(&mut self, p: &Projectile) {
        if p.beam {
            return;
        }
        if let Some(z) = self.content.weapons[p.weapon as usize].zone {
            self.add_flame(p.pos + Vec2::new(0.0, 6.0), z.radius, z.time * 0.6, z.dps, p.weapon);
        }
    }

    fn add_flame(&mut self, pos: Vec2, radius: f32, time: f32, dps: f32, weapon: u16) {
        if self.zones.len() >= 160 {
            return;
        }
        let st = self.player.stats;
        self.zones.push(Zone {
            pos,
            radius: radius * st.area,
            life: time * st.duration,
            age: 0.0,
            delay: 0.0,
            dps: dps * st.might,
            blast: 0.0,
            knockback: 0.0,
            weapon,
            slot: SLOT_ZONE as u8,
            status: None,
            fired: true,
            pillar: false,
        });
    }

    /// Explosion : degats et recul dans un rayon.
    #[allow(clippy::too_many_arguments)]
    fn explode(&mut self, pos: Vec2, r: f32, dmg: f32, kb: f32, weapon: u16, slot: usize, crit: (f32, f32)) {
        let status = self.content.weapons[weapon as usize].status;
        for i in self.enemies_in(pos, r) {
            let d = self.horde.pos[i] - pos;
            let dir = d.normalize_or(Vec2::Y);
            let f = 1.0 - 0.35 * (d.length() / r).min(1.0);
            self.strike(i, slot, weapon, dmg * f, crit, dir, kb * f, 0.05, status);
        }
        self.events.push(Event::Explosion { pos, radius: r, weapon });
    }

    pub(crate) fn update_zones(&mut self) {
        let mut zones = std::mem::take(&mut self.zones);
        let tick_burn = self.ticks.is_multiple_of(15);
        for z in zones.iter_mut() {
            z.age += DT;
            if !z.fired && z.age >= z.delay {
                z.fired = true;
                if z.blast > 0.0 {
                    for i in self.enemies_in(z.pos, z.radius) {
                        let dir = (self.horde.pos[i] - z.pos).normalize_or(Vec2::Y);
                        self.strike(i, z.slot as usize, z.weapon, z.blast, (0.0, 1.0), dir, z.knockback, 0.05, z.status);
                    }
                    self.events.push(Event::Pillar { pos: z.pos, radius: z.radius, weapon: z.weapon });
                }
            }
            if z.fired && z.dps > 0.0 && tick_burn {
                for i in self.enemies_in(z.pos, z.radius * 0.8) {
                    self.strike(i, SLOT_ZONE, z.weapon, z.dps * 0.25, (0.0, 1.0), Vec2::ZERO, 0.0, 0.24, None);
                }
            }
        }
        zones.retain(|z| z.age < z.delay + z.life);
        zones.append(&mut self.zones);
        self.zones = zones;
    }

    /// Effets d'etat : brulure (degats echelonnes), gel et etourdissement (minuteries).
    pub(crate) fn update_statuses(&mut self) {
        let t = self.ticks;
        for i in 0..self.horde.len() {
            let h = &mut self.horde;
            h.freeze[i] = (h.freeze[i] - DT).max(0.0);
            h.stun[i] = (h.stun[i] - DT).max(0.0);
            if h.burn[i] > 0.0 {
                h.burn[i] -= DT;
                if (t + h.uid[i] as u64).is_multiple_of(30) && h.hp[i] > 0.0 {
                    let dmg = h.burn_dps[i] * 0.5;
                    if h.burn[i] <= 0.0 {
                        h.burn_dps[i] = 0.0;
                    }
                    self.damage_enemy(i, dmg, false, Vec2::ZERO, 0.0, u16::MAX);
                }
            }
        }
    }
}
