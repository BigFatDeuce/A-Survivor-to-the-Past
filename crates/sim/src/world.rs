//! Le monde d'une partie et son pas de simulation (1/60 s).
//! Les armes sont dans `arsenal.rs`, les comportements ennemis dans `enemies.rs`,
//! les choix de niveau et coffres dans `upgrades.rs`.
use crate::arsenal::{EnemyShot, Projectile, Zone};
use crate::defs::{CharacterDef, Content, DashDef, MapDef, Pattern, StatusDef};
use crate::grid::Grid;
use crate::horde::{face_from, Horde, FACE_DOWN, HIT_SLOTS};
use crate::pickups::{PickupKind, Pickups};
use crate::rng::Rng;
use crate::upgrades::{PassiveSlot, Stats, WeaponSlot};
use glam::Vec2;

pub const DT: f32 = 1.0 / 60.0;

/// Commandes du joueur pour un pas.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInput {
    /// Deplacement (longueur <= 1).
    pub move_dir: Vec2,
    /// Visee explicite (souris, stick droit), sinon la direction du regard.
    pub aim: Option<Vec2>,
    /// Bouton d'attaque maintenu.
    pub attack: bool,
    /// Dash demande.
    pub dash: bool,
    /// Bascule attaque manuelle / automatique.
    pub toggle_auto: bool,
}

/// Ce qui s'est passe pendant le pas (pour le son et les effets).
#[derive(Clone, Copy, Debug)]
pub enum Event {
    Swing { pos: Vec2, angle: f32, weapon: u16 },
    /// Une arme s'est declenchee (son).
    Fire { pos: Vec2, weapon: u16 },
    Dash { pos: Vec2, dir: Vec2 },
    EnemyHit { pos: Vec2, kind: u16, damage: f32, crit: bool, dir: Vec2, weapon: u16 },
    EnemyKilled { pos: Vec2, kind: u16, dir: Vec2, elite: bool },
    PlayerHurt { pos: Vec2, damage: f32 },
    PlayerDied { pos: Vec2 },
    Pickup { pos: Vec2, kind: PickupKind, value: u32 },
    LevelUp { level: u32, pos: Vec2 },
    AutoToggled { auto: bool },
    Explosion { pos: Vec2, radius: f32, weapon: u16 },
    Slam { pos: Vec2, radius: f32, weapon: u16 },
    Storm { pos: Vec2, radius: f32, weapon: u16 },
    Pillar { pos: Vec2, radius: f32, weapon: u16 },
    /// Un ennemi se prepare (annonce visuelle et sonore).
    Windup { pos: Vec2, kind: u16 },
    /// Un ennemi tire.
    EnemyShot { pos: Vec2, kind: u16 },
    /// Bombe ennemie qui explose.
    ShotBlast { pos: Vec2, radius: f32 },
    /// Atterrissage d'un boss apres un bond (onde de choc).
    BossSlam { pos: Vec2, radius: f32 },
    /// Boss qui disparait puis reapparait ailleurs.
    Teleport { from: Vec2, to: Vec2, kind: u16 },
    EliteSpawned { pos: Vec2 },
    /// `last` : boss final de la carte.
    BossSpawned { pos: Vec2, kind: u16, last: bool },
    BossDied { pos: Vec2, kind: u16, last: bool },
    /// Une fee releve Link.
    Revived { pos: Vec2 },
    /// La porte de sortie apparait.
    PortalOpened { pos: Vec2 },
    /// Le joueur a franchi la porte : stage gagne.
    Victory { pos: Vec2 },
    ChestDropped { pos: Vec2, big: bool },
    Frozen { pos: Vec2 },
}

/// Coup d'epee en cours.
#[derive(Clone, Copy, Debug)]
pub struct Swing {
    pub id: u32,
    /// Emplacement d'arme et arme.
    pub slot: u8,
    pub weapon: u16,
    pub t: f32,
    pub dur: f32,
    /// Direction centrale (radians).
    pub dir: f32,
    pub arc: f32,
    pub radius: f32,
    /// Sens du balayage (+1 / -1), alterne a chaque coup.
    pub sign: f32,
    /// Ennemis deja touches par ce coup.
    pub hits: u32,
}

impl Swing {
    /// Angle de la lame au temps t.
    pub fn angle_at(&self, t: f32) -> f32 {
        let u = (t / self.dur).clamp(0.0, 1.0);
        // depart vif, fin amortie
        let e = 1.0 - (1.0 - u) * (1.0 - u);
        self.dir + self.sign * (-self.arc * 0.5 + self.arc * e)
    }
}

pub struct Player {
    pub pos: Vec2,
    pub prev: Vec2,
    pub vel: Vec2,
    /// Direction du regard (unitaire).
    pub facing: Vec2,
    pub face: u8,
    pub hp: f32,
    pub max_hp: f32,
    pub base_hp: f32,
    pub speed: f32,
    pub base_speed: f32,
    pub radius: f32,
    pub magnet: f32,
    pub base_magnet: f32,
    pub dash: DashDef,
    pub dash_t: f32,
    pub dash_cd: f32,
    pub dash_dir: Vec2,
    pub invuln: f32,
    /// Temps depuis le dernier coup recu.
    pub hurt_t: f32,
    pub auto_attack: bool,
    pub swing: Option<Swing>,
    /// Coups supplementaires a enchainer (Carquois).
    pub(crate) swing_queue: u32,
    pub(crate) next_swing: u32,
    pub(crate) swing_sign: f32,
    pub level: u32,
    pub xp: f32,
    pub xp_next: f32,
    pub rupees: u32,
    pub kills: u32,
    pub alive: bool,
    /// Invincible (tests de charge).
    pub god: bool,
    /// Temps de marche cumule (animation).
    pub walk_t: f32,
    pub moving: bool,
    pub stats: Stats,
}

impl Player {
    fn new(c: &CharacterDef) -> Player {
        Player {
            pos: Vec2::ZERO,
            prev: Vec2::ZERO,
            vel: Vec2::ZERO,
            facing: Vec2::Y,
            face: FACE_DOWN,
            hp: c.max_hp,
            max_hp: c.max_hp,
            base_hp: c.max_hp,
            speed: c.speed,
            base_speed: c.speed,
            radius: c.radius,
            magnet: c.magnet,
            base_magnet: c.magnet,
            dash: c.dash,
            dash_t: 0.0,
            dash_cd: 0.0,
            dash_dir: Vec2::Y,
            invuln: 0.0,
            hurt_t: 99.0,
            auto_attack: true,
            swing: None,
            swing_queue: 0,
            next_swing: 0,
            swing_sign: 1.0,
            level: 1,
            xp: 0.0,
            xp_next: 1.0,
            rupees: 0,
            kills: 0,
            alive: true,
            god: false,
            walk_t: 0.0,
            moving: false,
            stats: Stats::default(),
        }
    }

    pub fn dashing(&self) -> bool {
        self.dash_t > 0.0
    }

    /// Recharge effective du dash.
    pub fn dash_cooldown(&self) -> f32 {
        self.dash.cooldown * self.stats.dash_cd
    }
}

pub struct World {
    pub content: Content,
    /// Identifiant de la carte (nom du fichier).
    pub map_id: String,
    pub map: MapDef,
    pub time: f32,
    pub ticks: u64,
    pub player: Player,
    pub horde: Horde,
    pub pickups: Pickups,
    pub grid: Grid,
    pub rng: Rng,
    pub events: Vec<Event>,
    /// Demi-taille de la vue (pixels logiques) : apparition juste hors ecran.
    pub view_half: Vec2,
    /// Apparitions automatiques actives.
    pub spawning: bool,
    /// Armes et objets passifs possedes.
    pub weapons: Vec<WeaponSlot>,
    pub passives: Vec<PassiveSlot>,
    pub projectiles: Vec<Projectile>,
    pub zones: Vec<Zone>,
    pub shots: Vec<EnemyShot>,
    /// Sprites des projectiles ennemis (index de `EnemyShot::sprite`).
    pub shot_sprites: Vec<String>,
    /// Niveaux gagnes en attente de choix.
    pub pending_levels: u32,
    /// Progression du joueur appliquee a la partie (voir `set_mods`).
    pub mods: crate::mods::RunMods,
    /// Coffres ramasses en attente d'ouverture (vrai = grand coffre).
    pub pending_chests: Vec<bool>,
    /// Boss en vie (identifiant unique).
    pub boss_uid: Option<u32>,
    pub boss_defeated: bool,
    /// Boss final (identifiant) et etat de la fin de partie.
    pub final_uid: Option<u32>,
    final_done: bool,
    /// Porte de sortie et temps depuis son apparition.
    pub portal: Option<Vec2>,
    pub portal_t: f32,
    /// Stage gagne (porte franchie).
    pub won: bool,
    /// Graine du decor (placement des arbres, rochers...).
    pub prop_seed: u32,
    wave_acc: Vec<f32>,
    wave_kind: Vec<Option<u16>>,
    bursts_done: Vec<bool>,
    elites_done: Vec<bool>,
    boss_done: bool,
    pub(crate) accel: Vec<Vec2>,
}

impl World {
    pub fn new(content: Content, character: &str, map_id: &str, seed: u64) -> World {
        let c = content.characters.get(character).unwrap_or_else(|| panic!("personnage inconnu : {character}")).clone();
        let map = content.maps.get(map_id).unwrap_or_else(|| panic!("carte inconnue : {map_id}")).clone();
        let mut player = Player::new(&c);
        player.xp_next = content.rules.xp_first;
        let wave_kind = map
            .waves
            .iter()
            .map(|wv| {
                let k = content.enemy_index(&wv.enemy);
                if k.is_none() {
                    eprintln!("vague : ennemi inconnu {}", wv.enemy);
                }
                k
            })
            .collect();
        let n_kinds = content.enemies.len();
        let shot_sprites = crate::enemies::collect_shot_sprites(&content);
        let mut w = World {
            wave_acc: vec![0.0; map.waves.len()],
            wave_kind,
            bursts_done: vec![false; map.bursts.len()],
            elites_done: vec![false; map.elites.len()],
            boss_done: false,
            map_id: map_id.to_string(),
            map,
            time: 0.0,
            ticks: 0,
            player,
            horde: Horde::with_kinds(n_kinds),
            pickups: Pickups::default(),
            grid: Grid::new(192, 192),
            rng: Rng::new(seed),
            events: Vec::with_capacity(1024),
            view_half: Vec2::new(320.0, 180.0),
            spawning: true,
            weapons: Vec::new(),
            passives: Vec::new(),
            projectiles: Vec::new(),
            zones: Vec::new(),
            shots: Vec::new(),
            shot_sprites,
            pending_levels: 0,
            mods: crate::mods::RunMods::default(),
            pending_chests: Vec::new(),
            boss_uid: None,
            boss_defeated: false,
            final_uid: None,
            final_done: false,
            portal: None,
            portal_t: 0.0,
            won: false,
            prop_seed: (seed as u32) ^ ((seed >> 32) as u32).wrapping_mul(0x9e37_79b9) ^ 0x2545_f491,
            accel: Vec::new(),
            content,
        };
        match w.content.weapon_index(&c.weapon) {
            Some(id) => w.add_weapon(id),
            None => eprintln!("arme de depart inconnue : {}", c.weapon),
        }
        w
    }

    /// Avance l'horloge de partie sans simuler (essais : commencer a une minute donnee).
    pub fn skip_to(&mut self, t: f32) {
        self.time = t;
        for (i, b) in self.map.bursts.iter().enumerate() {
            self.bursts_done[i] = b.at < t;
        }
        for (i, e) in self.map.elites.iter().enumerate() {
            self.elites_done[i] = e.at < t;
        }
        self.boss_done = self.map.boss.as_ref().is_some_and(|b| b.at < t);
    }

    /// Experience requise pour passer du niveau `level` au suivant.
    pub fn xp_for(&self, level: u32) -> f32 {
        let r = &self.content.rules;
        let l = level as f32;
        r.xp_first + r.xp_step * (l - 1.0) + r.xp_step_late * (l - 20.0).max(0.0)
    }

    /// Fait apparaitre un ennemi (vie croissante avec le temps).
    pub fn spawn_enemy(&mut self, kind: u16, pos: Vec2) -> usize {
        let def = &self.content.enemies[kind as usize];
        let grow = if def.boss { self.mods.boss_hp } else { 1.0 + self.time / 60.0 * self.content.rules.hp_growth };
        let seed = self.rng.next_u32();
        let i = self.horde.spawn(kind, pos, def.hp * grow * self.mods.enemy_hp, def.radius, seed);
        self.horde.power[i] = self.mods.enemy_damage;
        i
    }

    /// Applique la progression du joueur (a faire juste apres `World::new`).
    pub fn set_mods(&mut self, m: crate::mods::RunMods) {
        self.mods = m;
        self.recompute_stats();
        self.player.hp = self.player.max_hp;
    }

    /// Point aleatoire juste hors de l'ecran (dans la direction `bias` si fournie).
    pub(crate) fn offscreen_point(&mut self, bias: Option<Vec2>) -> Vec2 {
        let h = self.view_half + Vec2::splat(24.0);
        let d = match bias {
            Some(b) if b.length_squared() > 0.01 && self.rng.chance(0.7) => {
                let a = b.y.atan2(b.x) + self.rng.range(-0.9, 0.9);
                Vec2::new(a.cos(), a.sin())
            }
            _ => self.rng.dir(),
        };
        // projection du rayon sur le bord du rectangle
        let tx = if d.x.abs() > 1e-4 { h.x / d.x.abs() } else { f32::MAX };
        let ty = if d.y.abs() > 1e-4 { h.y / d.y.abs() } else { f32::MAX };
        self.player.pos + d * tx.min(ty) + self.rng.dir() * self.rng.range(0.0, 12.0)
    }

    /// Fait apparaitre `n` ennemis selon un motif.
    pub fn spawn_burst(&mut self, kind: u16, n: u32, pattern: Pattern) {
        match pattern {
            Pattern::Ring => {
                let r = self.view_half.length() * 0.9;
                let a0 = self.rng.f() * std::f32::consts::TAU;
                for i in 0..n {
                    let a = a0 + i as f32 / n as f32 * std::f32::consts::TAU;
                    let p = self.player.pos + Vec2::new(a.cos(), a.sin()) * (r + self.rng.range(0.0, 10.0));
                    self.spawn_enemy(kind, p);
                }
            }
            Pattern::Pack => {
                let c = self.offscreen_point(None);
                let spread = 8.0 + (n as f32).sqrt() * 6.0;
                for _ in 0..n {
                    let p = c + self.rng.dir() * self.rng.range(0.0, spread);
                    self.spawn_enemy(kind, p);
                }
            }
        }
    }

    pub fn tick(&mut self, input: &PlayerInput) {
        self.time += DT;
        self.ticks += 1;
        self.player.prev = self.player.pos;
        self.horde.prev.copy_from_slice(&self.horde.pos);
        self.pickups.prev.copy_from_slice(&self.pickups.pos);
        for p in &mut self.projectiles {
            p.prev = p.pos;
        }
        for s in &mut self.shots {
            s.prev = s.pos;
        }
        for cd in &mut self.horde.hit_cd {
            for c in cd.iter_mut() {
                *c = c.saturating_sub(1);
            }
        }

        self.grid.rebuild(self.player.pos, &self.horde.pos);
        self.update_player(input);
        self.update_weapons(input);
        self.update_projectiles();
        self.update_zones();
        self.update_horde();
        self.update_statuses();
        self.update_shots();
        self.update_pickups();
        if self.spawning && self.player.alive {
            self.update_spawner();
        }
        self.update_portal();
        self.recycle_far();
        self.remove_dead();
    }

    /// Porte de sortie : apparait a la mort du boss final (ou a la fin du temps s'il n'y en a pas) ;
    /// l'atteindre gagne le stage.
    fn update_portal(&mut self) {
        if self.won {
            return;
        }
        if self.portal.is_none() && self.map.final_boss.is_none() && self.time >= self.map.duration {
            self.open_portal(self.player.pos + self.player.facing * 90.0);
        }
        let Some(at) = self.portal else { return };
        self.portal_t += DT;
        let p = &self.player;
        if p.alive && self.portal_t > 1.2 && (p.pos - at).length() < 14.0 {
            self.won = true;
            self.player.invuln = f32::MAX;
            self.events.push(Event::Victory { pos: at });
        }
    }

    /// Ouvre la porte de sortie (hors des obstacles) et arrete les apparitions.
    pub fn open_portal(&mut self, at: Vec2) {
        let at = crate::props::resolve(&self.map.props, self.prop_seed, at, 16.0, Vec2::ZERO, 1.0);
        self.portal = Some(at);
        self.portal_t = 0.0;
        self.spawning = false;
        self.events.push(Event::PortalOpened { pos: at });
    }

    fn update_player(&mut self, input: &PlayerInput) {
        let dash_cd = self.player.dash_cooldown();
        let p = &mut self.player;
        p.invuln = (p.invuln - DT).max(0.0);
        p.hurt_t += DT;
        p.dash_cd = (p.dash_cd - DT).max(0.0);
        if !p.alive {
            p.vel = Vec2::ZERO;
            p.dash_t = 0.0;
            return;
        }
        // regeneration
        if p.stats.regen > 0.0 && p.hp < p.max_hp {
            p.hp = (p.hp + p.stats.regen * DT).min(p.max_hp);
        }
        if input.toggle_auto {
            p.auto_attack = !p.auto_attack;
            self.events.push(Event::AutoToggled { auto: p.auto_attack });
        }
        let mv = input.move_dir.clamp_length_max(1.0);
        if input.dash && p.dash_cd <= 0.0 && p.dash_t <= 0.0 {
            p.dash_dir = if mv.length_squared() > 0.01 { mv.normalize() } else { p.facing };
            p.dash_t = p.dash.duration;
            p.dash_cd = dash_cd;
            p.invuln = p.invuln.max(p.dash.duration + 0.06);
            self.events.push(Event::Dash { pos: p.pos, dir: p.dash_dir });
        }
        if p.dash_t > 0.0 {
            p.dash_t -= DT;
            p.vel = p.dash_dir * p.dash.speed;
        } else {
            p.vel = mv * p.speed;
        }
        p.pos += p.vel * DT;
        p.pos = crate::props::resolve(&self.map.props, self.prop_seed, p.pos, p.radius, p.vel, 1.0);
        p.moving = mv.length_squared() > 0.01 || p.dash_t > 0.0;
        if p.moving {
            p.walk_t += DT * (p.vel.length() / p.speed).clamp(0.5, 1.6);
        } else {
            p.walk_t = 0.0;
        }
        if p.swing.is_none() && mv.length_squared() > 0.01 {
            p.facing = mv.normalize();
            p.face = face_from(mv, p.face);
        }
    }

    /// Touche un ennemi avec une source (emplacement) : respecte l'intervalle entre deux
    /// touches, tire le critique, applique recul et effet d'etat. Renvoie vrai si touche.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn strike(
        &mut self,
        i: usize,
        slot: usize,
        weapon: u16,
        damage: f32,
        crit: (f32, f32),
        dir: Vec2,
        knockback: f32,
        interval: f32,
        status: Option<StatusDef>,
    ) -> bool {
        if self.horde.hp[i] <= 0.0 || self.horde.hit_cd[i][slot.min(HIT_SLOTS - 1)] > 0 {
            return false;
        }
        self.horde.hit_cd[i][slot.min(HIT_SLOTS - 1)] = (interval / DT).round().clamp(1.0, 255.0) as u8;
        let is_crit = crit.0 > 0.0 && self.rng.chance(crit.0);
        let dmg = if is_crit { damage * crit.1 } else { damage };
        if let Some(st) = status {
            self.apply_status(i, st);
        }
        self.damage_enemy(i, dmg, is_crit, dir, knockback, weapon);
        true
    }

    pub(crate) fn apply_status(&mut self, i: usize, st: StatusDef) {
        let boss = self.content.enemies[self.horde.kind[i] as usize].boss;
        let resist = if boss {
            0.25
        } else if self.horde.elite[i] {
            0.6
        } else {
            1.0
        };
        let dur = self.player.stats.duration;
        match st {
            StatusDef::Burn { dps, time } => {
                self.horde.burn[i] = self.horde.burn[i].max(time * dur);
                self.horde.burn_dps[i] = self.horde.burn_dps[i].max(dps * self.player.stats.might);
            }
            StatusDef::Freeze { time } => {
                if self.horde.freeze[i] <= 0.0 {
                    self.events.push(Event::Frozen { pos: self.horde.pos[i] });
                }
                self.horde.freeze[i] = self.horde.freeze[i].max(time * dur * resist);
            }
            StatusDef::Stun { time } => {
                self.horde.stun[i] = self.horde.stun[i].max(time * dur * resist);
            }
        }
    }

    /// Inflige des degats ; la mort est traitee en fin de pas.
    pub fn damage_enemy(&mut self, i: usize, dmg: f32, crit: bool, dir: Vec2, knockback: f32, weapon: u16) {
        if self.horde.hp[i] <= 0.0 {
            return;
        }
        let k = self.horde.kind[i];
        let def = &self.content.enemies[k as usize];
        let boss = def.boss;
        self.horde.hp[i] -= dmg;
        self.horde.hurt[i] = 0.0;
        let mass = if boss { def.mass * 8.0 } else { def.mass * if self.horde.elite[i] { 2.5 } else { 1.0 } };
        self.horde.knock[i] += dir * knockback / mass;
        let pos = self.horde.pos[i];
        self.events.push(Event::EnemyHit { pos, kind: k, damage: dmg, crit, dir, weapon });
        if self.horde.hp[i] <= 0.0 {
            let elite = self.horde.elite[i];
            self.events.push(Event::EnemyKilled { pos, kind: k, dir, elite });
            self.player.kills += 1;
            if boss {
                let uid = self.horde.uid[i];
                let last = self.final_uid == Some(uid);
                self.events.push(Event::BossDied { pos, kind: k, last });
                if self.boss_uid == Some(uid) {
                    self.boss_uid = None;
                }
                self.boss_defeated = true;
                if last {
                    self.final_uid = None;
                    self.open_portal(pos + Vec2::new(0.0, 24.0));
                }
                self.pickups.spawn(PickupKind::BigChest, 0, pos, Vec2::ZERO, 120.0);
                self.events.push(Event::ChestDropped { pos, big: true });
                // pluie de rubis
                for _ in 0..12 {
                    let v = self.rng.dir() * self.rng.range(40.0, 110.0);
                    self.pickups.spawn(PickupKind::RupeeRed, 20, pos, v, self.rng.range(80.0, 140.0));
                }
            } else if elite {
                self.pickups.spawn(PickupKind::Chest, 0, pos, dir * 20.0, 110.0);
                self.events.push(Event::ChestDropped { pos, big: false });
                let xp = self.content.rules.elite_xp;
                self.pickups.spawn(PickupKind::rupee_for(xp), xp, pos, self.rng.dir() * 40.0, 90.0);
            } else {
                self.drop_loot(k, pos, dir);
            }
        }
    }

    fn drop_loot(&mut self, kind: u16, pos: Vec2, dir: Vec2) {
        let xp = self.content.enemies[kind as usize].xp;
        let rules = &self.content.rules;
        if self.rng.chance(rules.heart_drop) {
            let v = dir * 30.0 + self.rng.dir() * 15.0;
            self.pickups.spawn(PickupKind::Heart, 0, pos, v, 70.0);
        }
        if xp == 0 {
            return;
        }
        if self.pickups.len() as u32 >= rules.max_pickups {
            // fusion : la valeur rejoint un rubis existant
            let j = self.rng.below(self.pickups.len() as u32) as usize;
            if self.pickups.kind[j].is_rupee() {
                self.pickups.value[j] += xp;
                self.pickups.kind[j] = PickupKind::rupee_for(self.pickups.value[j]);
                return;
            }
        }
        let v = dir * 25.0 + self.rng.dir() * 20.0;
        self.pickups.spawn(PickupKind::rupee_for(xp), xp, pos, v, self.rng.range(50.0, 80.0));
    }

    /// Degats subis par le joueur (armure deduite).
    pub(crate) fn hurt_player(&mut self, dmg: f32) {
        let p = &mut self.player;
        if !p.alive || p.invuln > 0.0 || p.god {
            return;
        }
        let dmg = (dmg - p.stats.armor).max(1.0);
        p.hp = (p.hp - dmg).max(0.0);
        p.invuln = self.content.rules.hurt_iframes;
        p.hurt_t = 0.0;
        let pos = p.pos;
        self.events.push(Event::PlayerHurt { pos, damage: dmg });
        // repousse les ennemis proches pour laisser respirer
        for i in 0..self.horde.len() {
            let d = self.horde.pos[i] - pos;
            let l = d.length();
            if l < 40.0 && l > 0.01 {
                self.horde.knock[i] += d / l * 160.0 * (1.0 - l / 40.0);
            }
        }
        if p.hp <= 0.0 && self.mods.revives > 0 {
            // fee en bouteille : Link se releve
            self.mods.revives -= 1;
            p.hp = p.max_hp * 0.5;
            p.invuln = 2.5;
            self.events.push(Event::Revived { pos });
            for i in 0..self.horde.len() {
                let d = self.horde.pos[i] - pos;
                let l = d.length();
                if l < 90.0 && l > 0.01 {
                    self.horde.knock[i] += d / l * 420.0 * (1.0 - l / 90.0);
                }
            }
        } else if p.hp <= 0.0 {
            p.alive = false;
            p.swing = None;
            self.events.push(Event::PlayerDied { pos });
        }
    }

    fn update_pickups(&mut self) {
        let pp = self.player.pos;
        let magnet = self.player.magnet;
        let alive = self.player.alive;
        let mut i = 0;
        while i < self.pickups.len() {
            let pk = &mut self.pickups;
            pk.age[i] += DT;
            // petit rebond
            if pk.z[i] > 0.0 || pk.vz[i] > 0.0 {
                pk.vz[i] -= 420.0 * DT;
                pk.z[i] += pk.vz[i] * DT;
                if pk.z[i] <= 0.0 {
                    pk.z[i] = 0.0;
                    pk.vz[i] = if pk.vz[i] < -50.0 { -pk.vz[i] * 0.35 } else { 0.0 };
                }
            }
            let d = pp - pk.pos[i];
            let dist = d.length();
            let chest = pk.kind[i].is_chest();
            if chest {
                pk.vel[i] *= 0.9;
                pk.pos[i] += pk.vel[i] * DT;
                if alive && dist < 12.0 && pk.age[i] > 0.4 {
                    let (kind, value, pos) = (pk.kind[i], pk.value[i], pk.pos[i]);
                    pk.remove(i);
                    self.collect(kind, value, pos);
                    continue;
                }
                i += 1;
                continue;
            }
            if alive && pk.pull[i] < 0.0 && dist < magnet && pk.age[i] > 0.25 {
                pk.pull[i] = 0.0;
            }
            if pk.pull[i] >= 0.0 {
                pk.pull[i] += DT;
                let sp = 40.0 + pk.pull[i] * 520.0;
                let dir = d / dist.max(0.001);
                // depart en arriere (anticipation), puis aspiration
                let back = if pk.pull[i] < 0.08 { -60.0 } else { 0.0 };
                pk.vel[i] = dir * (sp + back);
            } else {
                pk.vel[i] *= 0.9;
            }
            pk.pos[i] += pk.vel[i] * DT;
            if alive && pk.pull[i] >= 0.0 && (pp - pk.pos[i]).length() < 6.0 {
                let (kind, value, pos) = (pk.kind[i], pk.value[i], pk.pos[i]);
                pk.remove(i);
                self.collect(kind, value, pos);
                continue;
            }
            i += 1;
        }
    }

    /// Attire tous les rubis vers le joueur (fin de boss).
    pub fn vacuum(&mut self) {
        for i in 0..self.pickups.len() {
            if self.pickups.kind[i].is_rupee() && self.pickups.pull[i] < 0.0 {
                self.pickups.pull[i] = 0.0;
            }
        }
    }

    fn collect(&mut self, kind: PickupKind, value: u32, pos: Vec2) {
        self.events.push(Event::Pickup { pos, kind, value });
        match kind {
            PickupKind::Heart => {
                let p = &mut self.player;
                p.hp = (p.hp + self.content.rules.heart_heal * self.mods.heal).min(p.max_hp);
            }
            PickupKind::Chest => self.pending_chests.push(false),
            PickupKind::BigChest => self.pending_chests.push(true),
            _ => self.gain_xp(value),
        }
    }

    pub(crate) fn gain_xp(&mut self, value: u32) {
        self.player.rupees += value;
        self.player.xp += value as f32 * self.mods.xp;
        while self.player.xp >= self.player.xp_next {
            self.player.xp -= self.player.xp_next;
            self.player.level += 1;
            self.player.xp_next = self.xp_for(self.player.level);
            self.pending_levels += 1;
            self.level_wave();
        }
    }

    /// Montee de niveau : onde qui repousse (sans blesser) et courte invulnerabilite.
    fn level_wave(&mut self) {
        let r = self.content.rules.clone();
        let p = &mut self.player;
        p.invuln = p.invuln.max(0.6);
        let pos = p.pos;
        self.events.push(Event::LevelUp { level: p.level, pos });
        let rad = r.levelup_wave_radius;
        for i in 0..self.horde.len() {
            let d = self.horde.pos[i] - pos;
            let l = d.length();
            if l < rad && l > 0.01 {
                let f = 1.0 - l / rad;
                let mass = self.content.enemies[self.horde.kind[i] as usize].mass;
                self.horde.knock[i] += d / l * r.levelup_wave_force * f / mass;
            }
        }
    }

    fn update_spawner(&mut self) {
        let t = self.time;
        let max = self.map.max_enemies as usize;
        // pendant le boss, la horde se calme
        let calm = if self.boss_uid.is_some() { 0.35 } else { 1.0 };
        for w in 0..self.map.waves.len() {
            let (from, to, rate, cap) = {
                let wv = &self.map.waves[w];
                (wv.from, wv.to, wv.rate, wv.cap)
            };
            let Some(kind) = self.wave_kind[w] else { continue };
            if t < from || t >= to {
                continue;
            }
            let u = ((t - from) / (to - from).max(0.001)).clamp(0.0, 1.0);
            self.wave_acc[w] += (rate.0 + (rate.1 - rate.0) * u) * DT * calm * self.mods.spawn_rate;
            while self.wave_acc[w] >= 1.0 {
                self.wave_acc[w] -= 1.0;
                if self.horde.alive[kind as usize] < cap && self.horde.len() < max {
                    let bias = Some(self.player.vel);
                    let p = self.offscreen_point(bias);
                    self.spawn_enemy(kind, p);
                }
            }
        }
        for b in 0..self.map.bursts.len() {
            if self.bursts_done[b] || t < self.map.bursts[b].at {
                continue;
            }
            self.bursts_done[b] = true;
            let (enemy, count, pattern) = {
                let bd = &self.map.bursts[b];
                (bd.enemy.clone(), bd.count, bd.pattern)
            };
            if let Some(k) = self.content.enemy_index(&enemy) {
                let n = (count as f32 * self.mods.spawn_rate).round() as u32;
                self.spawn_burst(k, n, pattern);
            }
        }
        for e in 0..self.map.elites.len() {
            if self.elites_done[e] || t < self.map.elites[e].at {
                continue;
            }
            self.elites_done[e] = true;
            let enemy = self.map.elites[e].enemy.clone();
            if let Some(k) = self.content.enemy_index(&enemy) {
                let p = self.offscreen_point(None);
                let i = self.spawn_enemy(k, p);
                let r = &self.content.rules;
                self.horde.elite[i] = true;
                self.horde.hp[i] *= r.elite_hp;
                self.horde.max_hp[i] = self.horde.hp[i];
                self.horde.power[i] = r.elite_damage * self.mods.enemy_damage;
                self.events.push(Event::EliteSpawned { pos: p });
            }
        }
        if !self.boss_done {
            if let Some(b) = self.map.boss.clone() {
                if t >= b.at {
                    self.boss_done = true;
                    if let Some(k) = self.content.enemy_index(&b.enemy) {
                        let dir = if self.player.facing.length_squared() > 0.1 { self.player.facing } else { Vec2::Y };
                        let p = self.player.pos + dir * 150.0;
                        let i = self.spawn_enemy(k, p);
                        self.boss_uid = Some(self.horde.uid[i]);
                        self.events.push(Event::BossSpawned { pos: p, kind: k, last: false });
                    }
                }
            }
        }
        if !self.final_done {
            if let Some(b) = self.map.final_boss.clone() {
                if t >= b.at {
                    self.final_done = true;
                    if let Some(k) = self.content.enemy_index(&b.enemy) {
                        let dir = if self.player.facing.length_squared() > 0.1 { self.player.facing } else { Vec2::Y };
                        let p = self.player.pos + dir * 160.0;
                        let i = self.spawn_enemy(k, p);
                        let uid = self.horde.uid[i];
                        self.boss_uid = Some(uid);
                        self.final_uid = Some(uid);
                        self.events.push(Event::BossSpawned { pos: p, kind: k, last: true });
                    }
                }
            }
        }
    }

    /// Les ennemis distances sont replaces devant le joueur (la pression reste constante).
    fn recycle_far(&mut self) {
        let lim = self.view_half + Vec2::splat(140.0);
        let pp = self.player.pos;
        for i in 0..self.horde.len() {
            let d = (self.horde.pos[i] - pp).abs();
            if d.x > lim.x || d.y > lim.y {
                if self.content.enemies[self.horde.kind[i] as usize].boss {
                    continue;
                }
                let bias = Some(self.player.vel);
                let p = self.offscreen_point(bias);
                self.horde.pos[i] = p;
                self.horde.prev[i] = p;
                self.horde.knock[i] = Vec2::ZERO;
            }
        }
        // objets trop loin : oublies (sauf coffres)
        let lim2 = lim * 2.5;
        let mut i = 0;
        while i < self.pickups.len() {
            let d = (self.pickups.pos[i] - pp).abs();
            if (d.x > lim2.x || d.y > lim2.y) && !self.pickups.kind[i].is_chest() {
                self.pickups.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn remove_dead(&mut self) {
        let mut i = 0;
        while i < self.horde.len() {
            if self.horde.hp[i] <= 0.0 {
                self.horde.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Index du boss en vie.
    pub fn boss_index(&self) -> Option<usize> {
        let uid = self.boss_uid?;
        self.horde.uid.iter().position(|&u| u == uid)
    }
}
