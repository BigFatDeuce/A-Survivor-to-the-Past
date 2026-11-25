//! Comportements ennemis (poursuite, vol, bonds, flanerie, charge, tir, patrons de boss),
//! separation de la horde, contact avec le joueur et projectiles ennemis.
use crate::arsenal::EnemyShot;
use crate::defs::{Behavior, BossAction, Content, ShotDef};
use crate::horde::{face_from, ST_ACT, ST_MOVE, ST_REST, ST_WINDUP};
use crate::world::{Event, World, DT};
use glam::Vec2;

/// Tous les sprites de projectiles ennemis (pour l'affichage).
pub fn collect_shot_sprites(c: &Content) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &ShotDef| {
        if !out.contains(&s.sprite) {
            out.push(s.sprite.clone());
        }
    };
    for e in &c.enemies {
        match &e.behavior {
            Behavior::Shooter { shot, .. } => add(shot),
            Behavior::Boss { actions } => {
                for a in actions {
                    if let BossAction::Throw { shot, .. } = a {
                        add(shot);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Tir demande pendant la mise a jour de la horde (applique ensuite).
struct ShotRequest {
    from: Vec2,
    at: Vec2,
    kind: u16,
    /// Action de boss (sinon le tir du comportement Shooter).
    boss_step: Option<u8>,
    power: f32,
}

impl World {
    pub(crate) fn update_horde(&mut self) {
        let n = self.horde.len();
        self.accel.resize(n, Vec2::ZERO);
        let pp = self.player.pos;
        let t = self.time;
        let can_hurt = self.player.alive && self.player.invuln <= 0.0 && !self.player.god;
        let mut contact: Option<f32> = None;
        let mut shots: Vec<ShotRequest> = Vec::new();
        let mut summons: Vec<(u16, Vec2, u32, u8)> = Vec::new();
        let mut windups: Vec<(Vec2, u16)> = Vec::new();
        let mut slams: Vec<(Vec2, f32, f32)> = Vec::new();
        let mut teleports: Vec<(Vec2, Vec2, u16)> = Vec::new();
        {
            let h = &mut self.horde;
            let grid = &self.grid;
            let defs = &self.content.enemies;
            for i in 0..n {
                if h.hp[i] <= 0.0 {
                    self.accel[i] = Vec2::ZERO;
                    continue;
                }
                let kind = h.kind[i];
                let def = &defs[kind as usize];
                let x = h.pos[i];
                let to = pp - x;
                let dist = to.length().max(0.001);
                let dirp = to / dist;
                h.hurt[i] += DT;
                let held = h.held(i);
                let ph = (h.seed[i] & 1023) as f32 * 0.00614;
                let mut want = if held {
                    Vec2::ZERO
                } else {
                    match &def.behavior {
                        Behavior::Chase => dirp * def.speed,
                        Behavior::Flutter { amp, freq } => {
                            let a = (t * freq + ph).sin() * amp;
                            let (s, c) = a.sin_cos();
                            h.z[i] = 10.0 + (t * freq * 2.0 + ph).sin() * 3.0;
                            Vec2::new(dirp.x * c - dirp.y * s, dirp.x * s + dirp.y * c) * def.speed
                        }
                        Behavior::Hop { rest, jump, speed } => {
                            h.timer[i] += DT;
                            let cycle = rest + jump;
                            if h.timer[i] >= cycle {
                                h.timer[i] -= cycle;
                            }
                            if h.timer[i] < *rest {
                                h.z[i] = 0.0;
                                h.vel[i] = dirp;
                                Vec2::ZERO
                            } else {
                                let u = (h.timer[i] - rest) / jump;
                                h.z[i] = 4.0 * u * (1.0 - u) * 14.0;
                                h.vel[i] * *speed
                            }
                        }
                        Behavior::Flee { rest, jump, speed, range } => {
                            h.timer[i] += DT;
                            let cycle = rest + jump;
                            if h.timer[i] >= cycle {
                                h.timer[i] -= cycle;
                            }
                            if h.timer[i] < *rest {
                                h.z[i] = 0.0;
                                // direction du prochain bond : loin du joueur, avec un ecart qui change
                                let side = Vec2::from_angle((t * 0.9 + ph * 5.0).sin() * 0.9);
                                let away = -dirp;
                                let rot = Vec2::new(away.x * side.x - away.y * side.y, away.x * side.y + away.y * side.x);
                                h.vel[i] = if dist < *range { rot } else { Vec2::from_angle(ph * 6.0 + t * 0.4) };
                                Vec2::ZERO
                            } else {
                                let u = (h.timer[i] - rest) / jump;
                                h.z[i] = 4.0 * u * (1.0 - u) * 12.0;
                                let sp = if dist < *range { *speed } else { *speed * 0.35 };
                                h.vel[i] * sp
                            }
                        }
                        Behavior::Wander { turn } => {
                            let a = ph * 6.0 + (t * 0.55 + ph).sin() * turn + (t * 1.3 + ph * 3.0).sin() * turn * 0.5;
                            let drift = Vec2::from_angle(a);
                            (dirp * 0.55 + drift * 0.6).normalize_or(dirp) * def.speed
                        }
                        Behavior::Charger { range, windup, speed, time, rest } => {
                            h.timer[i] -= DT;
                            match h.state[i] {
                                ST_WINDUP => {
                                    if h.timer[i] <= 0.0 {
                                        h.state[i] = ST_ACT;
                                        h.timer[i] = *time;
                                        h.vel[i] = dirp;
                                    }
                                    Vec2::ZERO
                                }
                                ST_ACT => {
                                    if h.timer[i] <= 0.0 {
                                        h.state[i] = ST_REST;
                                        h.timer[i] = *rest;
                                    }
                                    h.vel[i] * *speed
                                }
                                ST_REST => {
                                    if h.timer[i] <= 0.0 {
                                        h.state[i] = ST_MOVE;
                                    }
                                    dirp * def.speed * 0.3
                                }
                                _ => {
                                    if dist < *range && h.timer[i] <= 0.0 {
                                        h.state[i] = ST_WINDUP;
                                        h.timer[i] = *windup;
                                        windups.push((x, kind));
                                    }
                                    dirp * def.speed
                                }
                            }
                        }
                        Behavior::Shooter { range, keep, cooldown, windup, .. } => {
                            h.timer[i] -= DT;
                            match h.state[i] {
                                ST_WINDUP => {
                                    if h.timer[i] <= 0.0 {
                                        h.state[i] = ST_MOVE;
                                        h.timer[i] = cooldown * (0.8 + (h.seed[i] % 41) as f32 / 100.0);
                                        shots.push(ShotRequest { from: x, at: pp, kind, boss_step: None, power: h.power[i] });
                                    }
                                    Vec2::ZERO
                                }
                                _ => {
                                    if dist < *range && h.timer[i] <= 0.0 {
                                        h.state[i] = ST_WINDUP;
                                        h.timer[i] = *windup;
                                    }
                                    if dist > *keep {
                                        dirp * def.speed
                                    } else if dist < keep * 0.65 {
                                        -dirp * def.speed * 0.6
                                    } else {
                                        // tourne autour du joueur
                                        let side = if h.seed[i] & 1 == 0 { 1.0 } else { -1.0 };
                                        dirp.perp() * side * def.speed * 0.5
                                    }
                                }
                            }
                        }
                        Behavior::Boss { actions } => {
                            if actions.is_empty() {
                                dirp * def.speed
                            } else {
                                let step = h.step[i] as usize % actions.len();
                                h.timer[i] -= DT;
                                let mut next = false;
                                let v = match &actions[step] {
                                    BossAction::Walk(secs) => {
                                        if h.state[i] == ST_MOVE {
                                            h.state[i] = ST_ACT;
                                            h.timer[i] = *secs;
                                        }
                                        if h.timer[i] <= 0.0 {
                                            next = true;
                                        }
                                        // rattrape le joueur s'il est loin
                                        let catch = if dist > 220.0 { 2.5 } else { 1.0 };
                                        dirp * def.speed * catch
                                    }
                                    BossAction::Charge { windup, speed, time } => match h.state[i] {
                                        ST_MOVE => {
                                            h.state[i] = ST_WINDUP;
                                            h.timer[i] = *windup;
                                            windups.push((x, kind));
                                            Vec2::ZERO
                                        }
                                        ST_WINDUP => {
                                            if h.timer[i] <= 0.0 {
                                                h.state[i] = ST_ACT;
                                                h.timer[i] = *time;
                                                h.vel[i] = dirp;
                                            }
                                            Vec2::ZERO
                                        }
                                        _ => {
                                            if h.timer[i] <= 0.0 {
                                                next = true;
                                            }
                                            h.vel[i] * *speed
                                        }
                                    },
                                    BossAction::Throw { windup, .. } => match h.state[i] {
                                        ST_MOVE => {
                                            h.state[i] = ST_WINDUP;
                                            h.timer[i] = *windup;
                                            windups.push((x, kind));
                                            Vec2::ZERO
                                        }
                                        _ => {
                                            if h.timer[i] <= 0.0 {
                                                shots.push(ShotRequest { from: x, at: pp, kind, boss_step: Some(step as u8), power: 1.0 });
                                                next = true;
                                            }
                                            Vec2::ZERO
                                        }
                                    },
                                    BossAction::Summon { count, .. } => {
                                        summons.push((kind, x, *count, step as u8));
                                        next = true;
                                        Vec2::ZERO
                                    }
                                    BossAction::Leap { windup, time, radius, damage } => match h.state[i] {
                                        ST_MOVE => {
                                            h.state[i] = ST_WINDUP;
                                            h.timer[i] = *windup;
                                            windups.push((x, kind));
                                            Vec2::ZERO
                                        }
                                        ST_WINDUP => {
                                            if h.timer[i] <= 0.0 {
                                                // vise la ou le joueur se trouve, sans le depasser de trop
                                                h.state[i] = ST_ACT;
                                                h.timer[i] = *time;
                                                h.vel[i] = to.clamp_length_max(260.0) / time.max(0.05);
                                            }
                                            Vec2::ZERO
                                        }
                                        _ => {
                                            let u = 1.0 - (h.timer[i] / time.max(0.05)).clamp(0.0, 1.0);
                                            h.z[i] = 4.0 * u * (1.0 - u) * 40.0;
                                            if h.timer[i] <= 0.0 {
                                                h.z[i] = 0.0;
                                                slams.push((x, *radius, damage * h.power[i]));
                                                next = true;
                                                Vec2::ZERO
                                            } else {
                                                h.vel[i]
                                            }
                                        }
                                    },
                                    BossAction::Teleport { windup, range } => match h.state[i] {
                                        ST_MOVE => {
                                            h.state[i] = ST_WINDUP;
                                            h.timer[i] = *windup;
                                            windups.push((x, kind));
                                            Vec2::ZERO
                                        }
                                        _ => {
                                            if h.timer[i] <= 0.0 {
                                                let a = (t * 1.7 + ph * 5.0).sin() * std::f32::consts::PI + dirp.y.atan2(dirp.x);
                                                let dest = pp + Vec2::from_angle(a + std::f32::consts::PI) * *range;
                                                teleports.push((x, dest, kind));
                                                h.pos[i] = dest;
                                                h.prev[i] = dest;
                                                next = true;
                                            }
                                            Vec2::ZERO
                                        }
                                    },
                                };
                                if next {
                                    h.step[i] = ((step + 1) % actions.len()) as u8;
                                    h.state[i] = ST_MOVE;
                                    h.timer[i] = 0.0;
                                }
                                v
                            }
                        }
                    }
                };
                want *= self.mods.enemy_speed;
                // sonne par un coup : ralenti
                let kl = h.knock[i].length();
                if kl > 25.0 {
                    want *= 0.15;
                }
                let mut a = want + h.knock[i];
                // separation (les volants l'ignorent)
                if !def.flying {
                    let r = h.radius[i];
                    let (cx, cy) = grid.coords(x);
                    let mut checked = 0;
                    'outer: for oy in -1..=1 {
                        for ox in -1..=1 {
                            let (nx, ny) = (cx + ox, cy + oy);
                            if nx < 0 || ny < 0 || nx >= grid.w || ny >= grid.h {
                                continue;
                            }
                            for &j in grid.cell(nx, ny) {
                                let j = j as usize;
                                if j == i {
                                    continue;
                                }
                                let s = x - h.pos[j];
                                let sep = r + h.radius[j];
                                let d2 = s.length_squared();
                                if d2 < sep * sep && d2 > 1e-4 {
                                    let d = d2.sqrt();
                                    // le boss pousse sans etre pousse
                                    let w = if def.boss { 0.1 } else { 11.0 };
                                    a += s * ((sep - d) * w / d);
                                }
                                checked += 1;
                                if checked >= 28 {
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
                self.accel[i] = a;
                if !held {
                    let look = if h.state[i] == ST_WINDUP || want.length_squared() < 1.0 { dirp } else { want };
                    h.face[i] = face_from(look, h.face[i]);
                }
                // contact avec le joueur
                if can_hurt && h.z[i] < 6.0 && h.freeze[i] <= 0.0 {
                    let rr = self.player.radius + h.radius[i] * 0.75;
                    let dmg = def.damage * h.power[i];
                    if dmg > 0.0 && dist < rr && contact.is_none_or(|d| dmg > d) {
                        contact = Some(dmg);
                    }
                }
            }
            let props = &self.map.props;
            let solid = props.iter().any(|p| p.solid);
            for i in 0..n {
                h.pos[i] += self.accel[i] * DT;
                h.knock[i] *= 0.84;
                if solid {
                    let def = &defs[h.kind[i] as usize];
                    // les volants passent au-dessus, les boss renversent tout
                    if !def.flying && !def.boss {
                        let side = if h.seed[i] & 2 == 0 { 1.0 } else { -1.0 };
                        h.pos[i] = crate::props::resolve(props, self.prop_seed, h.pos[i], h.radius[i] * 0.8, pp - h.pos[i], side);
                    }
                }
            }
        }
        for (pos, kind) in windups {
            self.events.push(Event::Windup { pos, kind });
        }
        for s in shots {
            self.enemy_fire(s);
        }
        for (kind, at, count, step) in summons {
            self.boss_summon(kind, at, count, step);
        }
        for (pos, radius, damage) in slams {
            self.events.push(Event::BossSlam { pos, radius });
            if can_hurt && (self.player.pos - pos).length() < radius + self.player.radius && !self.player.dashing() {
                contact = Some(contact.unwrap_or(0.0).max(damage));
            }
        }
        for (from, to, kind) in teleports {
            self.events.push(Event::Teleport { from, to, kind });
        }
        if let Some(dmg) = contact {
            self.hurt_player(dmg);
        }
    }

    fn shot_def(&self, kind: u16, boss_step: Option<u8>) -> Option<ShotDef> {
        match &self.content.enemies[kind as usize].behavior {
            Behavior::Shooter { shot, .. } => Some(shot.clone()),
            Behavior::Boss { actions } => match boss_step.and_then(|s| actions.get(s as usize)) {
                Some(BossAction::Throw { shot, .. }) => Some(shot.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn enemy_fire(&mut self, r: ShotRequest) {
        let Some(shot) = self.shot_def(r.kind, r.boss_step) else { return };
        let sprite = self.shot_sprites.iter().position(|s| *s == shot.sprite).unwrap_or(0) as u16;
        let from = r.from - Vec2::new(0.0, 8.0);
        let base = (r.at - Vec2::new(0.0, 6.0) - from).normalize_or(Vec2::Y);
        let n = shot.count.max(1);
        let spread = if shot.ring { std::f32::consts::TAU / n as f32 } else { shot.spread.to_radians() };
        // tirs un peu plus lents que les valeurs de enemies.ron (lisibilite), zone de contact
        // un peu plus petite que le sprite (regle shot_hitbox)
        let speed = shot.speed * self.content.rules.shot_speed;
        let hit = match self.shot_hit.get(sprite as usize).copied().filter(|&v| v > 0.0) {
            Some(v) => v * self.content.rules.shot_hitbox,
            None => shot.radius * self.content.rules.shot_hitbox,
        };
        for k in 0..n {
            let off = if shot.ring { k as f32 * spread } else { (k as f32 - (n - 1) as f32 * 0.5) * spread };
            let dir = Vec2::from_angle(base.y.atan2(base.x) + off);
            if shot.lob {
                // bombes : visent la position du joueur (+ dispersion)
                let target = r.at + dir.perp() * off * 40.0 + self.rng.dir() * self.rng.range(0.0, 14.0);
                let flight = ((target - r.from).length() / speed).clamp(0.5, 1.4);
                self.shots.push(EnemyShot {
                    pos: from,
                    prev: from,
                    vel: Vec2::ZERO,
                    damage: shot.damage * r.power,
                    radius: shot.radius,
                    life: flight + 0.55,
                    age: 0.0,
                    sprite,
                    lob: true,
                    z: 0.0,
                    flight,
                    start: r.from,
                    target,
                    dead: false,
                });
            } else {
                self.shots.push(EnemyShot {
                    pos: from,
                    prev: from,
                    vel: dir * speed,
                    damage: shot.damage * r.power,
                    radius: hit,
                    life: 4.0,
                    age: 0.0,
                    sprite,
                    lob: false,
                    z: 0.0,
                    flight: 0.0,
                    start: from,
                    target: from,
                    dead: false,
                });
            }
        }
        self.events.push(Event::EnemyShot { pos: r.from, kind: r.kind });
    }

    fn boss_summon(&mut self, boss_kind: u16, at: Vec2, count: u32, step: u8) {
        let enemy = match &self.content.enemies[boss_kind as usize].behavior {
            Behavior::Boss { actions } => match actions.get(step as usize) {
                Some(BossAction::Summon { enemy, .. }) => Some(enemy.clone()),
                _ => None,
            },
            _ => None,
        };
        let Some(k) = enemy.and_then(|e| self.content.enemy_index(&e)) else { return };
        let a0 = self.rng.f() * std::f32::consts::TAU;
        for j in 0..count {
            let a = a0 + j as f32 / count as f32 * std::f32::consts::TAU;
            self.spawn_enemy(k, at + Vec2::from_angle(a) * 40.0);
        }
        self.events.push(Event::Windup { pos: at, kind: boss_kind });
    }

    pub(crate) fn update_shots(&mut self) {
        let pp = self.player.pos;
        let pr = self.player.radius;
        let mut hurt: Option<f32> = None;
        let mut blasts: Vec<(Vec2, f32)> = Vec::new();
        for s in self.shots.iter_mut() {
            s.prev = s.pos;
            s.age += DT;
            if s.lob {
                if s.age < s.flight {
                    let u = s.age / s.flight;
                    s.pos = s.start.lerp(s.target, u);
                    s.z = 4.0 * u * (1.0 - u) * 48.0;
                } else {
                    s.z = 0.0;
                    s.pos = s.target;
                    if s.age >= s.life {
                        s.dead = true;
                        blasts.push((s.pos, s.radius));
                        if (pp - s.pos).length() < s.radius + pr {
                            hurt = Some(hurt.unwrap_or(0.0).max(s.damage));
                        }
                    }
                }
            } else {
                s.pos += s.vel * DT;
                if (pp - Vec2::new(0.0, 6.0) - s.pos).length() < s.radius + pr {
                    s.dead = true;
                    hurt = Some(hurt.unwrap_or(0.0).max(s.damage));
                }
                if s.age >= s.life {
                    s.dead = true;
                }
            }
        }
        self.shots.retain(|s| !s.dead);
        for (pos, radius) in blasts {
            self.events.push(Event::ShotBlast { pos, radius });
        }
        if let Some(d) = hurt {
            if !self.player.dashing() {
                self.hurt_player(d);
            }
        }
    }
}
