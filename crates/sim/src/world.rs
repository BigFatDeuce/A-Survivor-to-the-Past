//! Le monde d'une partie et son pas de simulation (1/60 s).
//! Les armes sont dans `arsenal.rs`, les comportements ennemis dans `enemies.rs`,
//! les choix de niveau et coffres dans `upgrades.rs`.
use crate::arsenal::{EnemyShot, Projectile, Zone};
use crate::defs::{CharacterDef, Content, DashDef, DropDef, MapDef, Pattern, Stat, StatusDef, WeaponKind};
use crate::grid::Grid;
use crate::horde::{face_from, Horde, FACE_DOWN, HIT_SLOTS, MAX_HEROES};
use crate::pickups::{PickupKind, Pickups};
use crate::rng::Rng;
use crate::upgrades::{PassiveSlot, Stats, WeaponSlot};
use glam::Vec2;
use std::collections::HashSet;

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
    PlayerHurt { pos: Vec2, damage: f32, slot: u8 },
    PlayerDied { pos: Vec2, slot: u8 },
    Pickup { pos: Vec2, kind: PickupKind, value: u32 },
    LevelUp { level: u32, pos: Vec2, slot: u8 },
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
    /// Une fee (`fairy`) ou un allie, en coop, releve le joueur.
    Revived { pos: Vec2, slot: u8, fairy: bool },
    /// La porte de sortie apparait.
    PortalOpened { pos: Vec2 },
    /// Le joueur a franchi la porte : stage gagne.
    Victory { pos: Vec2 },
    ChestDropped { pos: Vec2, big: bool },
    Frozen { pos: Vec2 },
    /// Une Bob-omb arrive (annonce).
    BombSpawned { pos: Vec2 },
    /// Rencontre : un champion errant attend quelque part (annonce et fleche).
    EncounterSpawned { pos: Vec2, kind: u16 },
    /// Le champion est reparti sans etre vaincu.
    EncounterFled { pos: Vec2 },
    /// Champion vaincu : une recompense est a choisir.
    EncounterWon { pos: Vec2 },
    /// Bob-omb vaincue : les ennemis a l'ecran sont souffles.
    ScreenClear { pos: Vec2, count: u32 },
    /// Element de decor casse (indice dans `MapDef::props`).
    PropBroken { pos: Vec2, kind: u16 },
    /// Bloc de Somaria qui eclate en rayons.
    Burst { pos: Vec2, weapon: u16 },
    /// Medaillon ramasse : ennemis a l'ecran geles.
    Medallion { pos: Vec2 },
    /// Pikit voleur : arrive (annonce et fleche), repart sans etre vaincu, ou vaincu (rubis rendus).
    ThiefSpawned { pos: Vec2 },
    ThiefFled { pos: Vec2 },
    ThiefLooted { pos: Vec2, count: u32 },
    /// PNJ de passage (diseuse de bonne aventure, sorciere) : arrive, touche par le joueur, repart.
    NpcArrived { kind: crate::npc::NpcKind, pos: Vec2 },
    NpcMet { kind: crate::npc::NpcKind },
    NpcLeft { kind: crate::npc::NpcKind, pos: Vec2 },
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

#[derive(Clone)]
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
    /// Saut (Plume de Roc) : temps restant, duree totale et arme qui le donne.
    pub jump: f32,
    pub jump_dur: f32,
    pub(crate) jump_slot: u8,
    /// Place dans la partie (coop : 0 = hote ; solo : 0).
    pub slot: u8,
    /// Tirages des choix de ce joueur (niveaux, coffres) en coop : chaque jeu rejoue les choix des autres
    /// avec le meme generateur, sans toucher au hasard commun de la partie.
    pub menu_rng: Rng,
    /// Coop : temps passe a etre releve par un allie (a terre), voir `REVIVE_TIME`.
    pub revive: f32,
    /// Coop : joueur parti (deconnecte) ; il ne se releve plus.
    pub gone: bool,
    /// Coop : secondes depuis le debut de ses choix en attente (niveau, coffre) ; 0 sans choix.
    pub choose_t: f32,
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
            jump: 0.0,
            jump_dur: 0.0,
            jump_slot: 0,
            slot: 0,
            menu_rng: Rng::new(0),
            revive: 0.0,
            gone: false,
            choose_t: 0.0,
        }
    }

    /// Coop : choix en cours (la partie continue pour les autres) : il n'attaque pas.
    pub fn choosing(&self) -> bool {
        self.choose_t > 0.0
    }

    /// Coop : ennemis et tirs l'ignorent pendant les `CHOICE_SHIELD` premieres secondes de son choix.
    pub fn shielded(&self) -> bool {
        self.choosing() && self.choose_t <= CHOICE_SHIELD
    }

    pub fn dashing(&self) -> bool {
        self.dash_t > 0.0
    }

    /// Recharge effective du dash.
    pub fn dash_cooldown(&self) -> f32 {
        self.dash.cooldown * self.stats.dash_cd
    }
}

/// Un autre joueur de la partie (coop) : son personnage et tout ce qui lui est propre. Le joueur local
/// est toujours dans `World::player` et les champs voisins ; les autres attendent ici. Le pas de
/// simulation fait passer chaque joueur au premier plan, dans l'ordre des places : tous les jeux
/// calculent la meme chose, quel que soit le joueur local.
pub struct Hero {
    pub player: Player,
    pub weapons: Vec<WeaponSlot>,
    pub passives: Vec<PassiveSlot>,
    pub projectiles: Vec<Projectile>,
    pub zones: Vec<Zone>,
    pub char_stats: Vec<(Stat, f32)>,
    pub mods: crate::mods::RunMods,
    pub pending_levels: u32,
    pub pending_chests: Vec<bool>,
    pub used_partners: Vec<u16>,
    pub locked_weapons: Vec<u16>,
}

/// Coop : secondes aupres d'un allie a terre pour le relever, et distance maximale entre deux joueurs.
pub const REVIVE_TIME: f32 = 3.0;
pub const REVIVE_RANGE: f32 = 26.0;
pub const LEASH: f32 = 520.0;
/// Coop : secondes de protection d'un joueur qui choisit (niveau, coffre).
pub const CHOICE_SHIELD: f32 = 10.0;
/// Coups des joueurs sur le decor pendant un pas : (avant les evenements, apres), (centre, rayon).
type PropHits = (Vec<(Vec2, f32)>, Vec<(Vec2, f32)>);
/// Coop : vie et flux d'apparition des ennemis en plus, par joueur au-dela du premier.
pub const COOP_HP: f32 = 0.5;
pub const COOP_SPAWN: f32 = 0.3;

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
    /// Demi-taille visible de chaque sprite de tir (pixels, 0 = inconnue : rayon de enemies.ron),
    /// fournie par le jeu d'apres l'atlas ; la zone de contact en est une part (`shot_hitbox`).
    pub shot_hit: Vec<f32>,
    /// Niveaux gagnes en attente de choix.
    pub pending_levels: u32,
    /// Objets partenaires deja utilises par une fusion : chacun ne sert qu'a une seule par partie.
    pub used_partners: Vec<u16>,
    /// Armes pas encore debloquees par le joueur : jamais proposees (rempli par le jeu).
    pub locked_weapons: Vec<u16>,
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
    /// Porte annoncee : position et secondes avant qu'elle s'ouvre (apres le boss final).
    pub portal_pending: Option<(Vec2, f32)>,
    /// Stage gagne (porte franchie).
    pub won: bool,
    /// Graine du decor (placement des arbres, rochers...).
    pub prop_seed: u32,
    /// Cases du decor dont l'element a ete casse.
    pub broken: HashSet<(i32, i32)>,
    /// La carte a du decor cassable.
    breakables: bool,
    /// Bonus du personnage (appliques avec les objets passifs).
    pub char_stats: Vec<(Stat, f32)>,
    /// Bob-omb : type d'ennemi et temps avant la prochaine.
    pub bomb_kind: Option<u16>,
    bomb_t: f32,
    /// Rencontre en cours (champion errant) et temps avant la prochaine.
    pub encounter: Option<crate::encounter::Encounter>,
    pub encounter_t: f32,
    /// Champions vaincus dont la recompense reste a choisir.
    pub pending_rewards: u32,
    /// Pikit voleur de rubis : type d'ennemi, Pikit en cours et temps avant le prochain.
    pub thief_kind: Option<u16>,
    pub thief: Option<crate::thief::Thief>,
    pub thief_t: f32,
    /// PNJ presents, et heure d'arrivee de la sorciere (si elle doit passer).
    pub npcs: Vec<crate::npc::Npc>,
    pub witch_at: Option<f32>,
    /// Premier evenement du pas en cours.
    ev0: usize,
    wave_acc: Vec<f32>,
    wave_kind: Vec<Option<u16>>,
    bursts_done: Vec<bool>,
    elites_done: Vec<bool>,
    boss_done: bool,
    pub(crate) accel: Vec<Vec2>,
    /// Coop : les autres joueurs (vide en solo) et la place du joueur local.
    pub others: Vec<Hero>,
    pub local: u8,
    /// Coop : rubis ramasses pendant le pas, partages entre tous les joueurs a la fin du pas.
    shared_xp: u32,
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
            shot_hit: Vec::new(),
            pending_levels: 0,
            used_partners: Vec::new(),
            locked_weapons: Vec::new(),
            mods: crate::mods::RunMods::default(),
            pending_chests: Vec::new(),
            boss_uid: None,
            boss_defeated: false,
            final_uid: None,
            final_done: false,
            portal: None,
            portal_t: 0.0,
            portal_pending: None,
            won: false,
            prop_seed: (seed as u32) ^ ((seed >> 32) as u32).wrapping_mul(0x9e37_79b9) ^ 0x2545_f491,
            broken: HashSet::new(),
            breakables: false,
            char_stats: c.stats.clone(),
            bomb_kind: None,
            bomb_t: 0.0,
            encounter: None,
            encounter_t: 0.0,
            pending_rewards: 0,
            thief_kind: None,
            thief: None,
            thief_t: 0.0,
            npcs: Vec::new(),
            witch_at: None,
            ev0: 0,
            accel: Vec::new(),
            others: Vec::new(),
            local: 0,
            shared_xp: 0,
            content,
        };
        w.breakables = w.map.props.iter().any(|p| p.breakable);
        let bomb = w.content.rules.bomb_enemy.clone();
        if !bomb.is_empty() {
            w.bomb_kind = w.content.enemy_index(&bomb);
            if w.bomb_kind.is_none() {
                eprintln!("bob-omb : ennemi inconnu {bomb}");
            }
        }
        let (a, b) = w.content.rules.bomb_every;
        w.bomb_t = w.rng.range(a, b) * 0.6;
        w.encounter_t = w.content.rules.encounter.first;
        let thief = w.content.rules.thief.enemy.clone();
        if !thief.is_empty() {
            w.thief_kind = w.content.enemy_index(&thief);
            if w.thief_kind.is_none() {
                eprintln!("pikit : ennemi inconnu {thief}");
            }
        }
        w.thief_t = w.content.rules.thief.first;
        w.recompute_stats();
        w.player.hp = w.player.max_hp;
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
        // difficulte du stage : ennemis ordinaires plus resistants et plus dangereux
        let stage = if def.boss { 1.0 } else { self.map.power };
        let i = self.horde.spawn(kind, pos, def.hp * grow * self.mods.enemy_hp * stage, def.radius, seed);
        self.horde.power[i] = self.mods.enemy_damage * stage;
        i
    }

    /// Boss du stage : vie de base fixee par la carte (sinon celle de la fiche).
    fn spawn_boss(&mut self, b: &crate::defs::SpawnAt, kind: u16, pos: Vec2) -> usize {
        let i = self.spawn_enemy(kind, pos);
        if let Some(hp) = b.hp {
            self.horde.hp[i] = hp * self.mods.boss_hp * self.mods.enemy_hp;
            self.horde.max_hp[i] = self.horde.hp[i];
        }
        i
    }

    /// Applique la progression du joueur (a faire juste apres `World::new`).
    pub fn set_mods(&mut self, m: crate::mods::RunMods) {
        let (a, b) = crate::npc::WITCH_WINDOW;
        self.witch_at = m.witch.then(|| self.rng.range(a, b));
        self.mods = m;
        self.recompute_stats();
        self.player.hp = self.player.max_hp;
    }

    /// Point aleatoire juste hors de l'ecran (dans la direction `bias` si fournie).
    pub(crate) fn offscreen_point(&mut self, bias: Option<Vec2>) -> Vec2 {
        let h = self.view_half + Vec2::splat(24.0);
        let d = match bias {
            Some(b) if b.length_squared() > 0.01 && self.rng.chance(0.7) => {
                let a = libm::atan2f(b.y, b.x) + self.rng.range(-0.9, 0.9);
                Vec2::new(libm::cosf(a), libm::sinf(a))
            }
            _ => self.rng.dir(),
        };
        // projection du rayon sur le bord du rectangle
        let tx = if d.x.abs() > 1e-4 { h.x / d.x.abs() } else { f32::MAX };
        let ty = if d.y.abs() > 1e-4 { h.y / d.y.abs() } else { f32::MAX };
        self.spawn_center() + d * tx.min(ty) + self.rng.dir() * self.rng.range(0.0, 12.0)
    }

    /// Centre des apparitions : le joueur (solo) ou un joueur debout tire au sort (coop).
    fn spawn_center(&mut self) -> Vec2 {
        if self.others.is_empty() {
            return self.player.pos;
        }
        let alive: Vec<Vec2> = self.heroes().into_iter().filter(|h| h.2).map(|h| h.1).collect();
        if alive.is_empty() {
            return self.player.pos;
        }
        alive[self.rng.below(alive.len() as u32) as usize]
    }

    /// Fait apparaitre `n` ennemis selon un motif.
    pub fn spawn_burst(&mut self, kind: u16, n: u32, pattern: Pattern) {
        match pattern {
            Pattern::Ring => {
                let r = self.view_half.length() * 0.9;
                let a0 = self.rng.f() * std::f32::consts::TAU;
                for i in 0..n {
                    let a = a0 + i as f32 / n as f32 * std::f32::consts::TAU;
                    let p = self.hero_center() + Vec2::new(libm::cosf(a), libm::sinf(a)) * (r + self.rng.range(0.0, 10.0));
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
        self.tick_all(std::slice::from_ref(input));
    }

    /// Pas de simulation de tous les joueurs : `inputs[place]` (coop : memes commandes dans tous les jeux).
    pub fn tick_all(&mut self, inputs: &[PlayerInput]) {
        let n = self.hero_count();
        self.time += DT;
        self.ticks += 1;
        self.ev0 = self.events.len();
        for s in 0..n {
            self.front(s as u8);
            self.player.prev = self.player.pos;
            for p in &mut self.projectiles {
                p.prev = p.pos;
            }
        }
        self.front(0);
        self.horde.prev.copy_from_slice(&self.horde.pos);
        self.pickups.prev.copy_from_slice(&self.pickups.pos);
        for s in &mut self.shots {
            s.prev = s.pos;
        }
        for cd in &mut self.horde.hit_cd {
            for c in cd.iter_mut() {
                *c = c.saturating_sub(1);
            }
        }

        let center = self.hero_center();
        self.grid.rebuild(center, &self.horde.pos);
        let mut hits = (Vec::new(), Vec::new());
        for s in 0..n {
            self.front(s as u8);
            if n > 1 {
                let busy = self.pending_levels > 0 || !self.pending_chests.is_empty();
                self.player.choose_t = if busy { self.player.choose_t + DT } else { 0.0 };
            }
            let input = inputs.get(s).copied().unwrap_or_default();
            self.update_player(&input);
            self.leash();
            self.update_weapons(&input);
            self.update_projectiles();
            self.update_zones();
            if self.breakables {
                self.hero_prop_hits(&mut hits);
            }
        }
        self.front(0);
        if self.breakables {
            self.update_props(hits);
        }
        self.update_horde();
        self.update_statuses();
        self.update_shots();
        self.update_pickups();
        let coop = n > 1;
        if self.spawning && self.any_alive() {
            self.update_spawner();
            // rencontres, voleur et visiteurs ouvrent des menus : solo seulement
            if !coop {
                self.update_encounter();
                self.update_thief();
            }
        }
        if !coop {
            self.update_npcs();
        }
        if coop {
            self.share_xp();
            self.update_revives();
            self.front(0);
        }
        self.update_portal();
        self.recycle_far();
        self.remove_dead();
        self.front(self.local);
    }

    /// Joueurs de la partie (1 en solo).
    pub fn hero_count(&self) -> usize {
        1 + self.others.len()
    }

    /// Partie a plusieurs.
    pub fn coop(&self) -> bool {
        !self.others.is_empty()
    }

    /// Au moins un joueur debout (coop : la partie continue tant qu'un joueur tient).
    pub fn any_alive(&self) -> bool {
        self.player.alive || self.others.iter().any(|h| h.player.alive)
    }

    /// Joueurs dans l'ordre des places : (place, position, debout).
    pub fn heroes(&self) -> Vec<(u8, Vec2, bool)> {
        let mut v: Vec<(u8, Vec2, bool)> = std::iter::once(&self.player).chain(self.others.iter().map(|h| &h.player)).map(|p| (p.slot, p.pos, p.alive)).collect();
        v.sort_by_key(|h| h.0);
        v
    }

    /// Centre des joueurs debout (solo : le joueur).
    fn hero_center(&self) -> Vec2 {
        if self.others.is_empty() {
            return self.player.pos;
        }
        let alive: Vec<Vec2> = self.heroes().into_iter().filter(|h| h.2).map(|h| h.1).collect();
        if alive.is_empty() {
            return self.player.pos;
        }
        alive.iter().copied().sum::<Vec2>() / alive.len() as f32
    }

    /// Met le joueur de cette place au premier plan (`player`, armes, objets...). Sans effet en solo.
    pub fn front(&mut self, slot: u8) {
        if self.player.slot == slot {
            return;
        }
        let Some(k) = self.others.iter().position(|h| h.player.slot == slot) else { return };
        let h = &mut self.others[k];
        std::mem::swap(&mut self.player, &mut h.player);
        std::mem::swap(&mut self.weapons, &mut h.weapons);
        std::mem::swap(&mut self.passives, &mut h.passives);
        std::mem::swap(&mut self.projectiles, &mut h.projectiles);
        std::mem::swap(&mut self.zones, &mut h.zones);
        std::mem::swap(&mut self.char_stats, &mut h.char_stats);
        std::mem::swap(&mut self.mods, &mut h.mods);
        std::mem::swap(&mut self.pending_levels, &mut h.pending_levels);
        std::mem::swap(&mut self.pending_chests, &mut h.pending_chests);
        std::mem::swap(&mut self.used_partners, &mut h.used_partners);
        std::mem::swap(&mut self.locked_weapons, &mut h.locked_weapons);
    }

    /// Coop : ajoute un joueur (place, personnage, modificateurs de sa progression). Les ennemis gardent les
    /// reglages de la partie (`mods` de l'hote) ; le joueur arrive a cote des autres.
    pub fn add_hero(&mut self, slot: u8, character: &str, mods: crate::mods::RunMods, seed: u64) {
        let c = self.content.characters.get(character).or_else(|| self.content.characters.get("link")).cloned();
        let Some(c) = c else { return };
        let mut player = Player::new(&c);
        player.slot = slot;
        player.xp_next = self.content.rules.xp_first;
        player.menu_rng = Rng::new(seed ^ (slot as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        player.pos = Vec2::new(20.0 * slot as f32, 0.0);
        player.prev = player.pos;
        let mut m = mods;
        // la partie (difficulte, Sceaux) est celle de l'hote
        m.enemy_hp = self.mods.enemy_hp;
        m.enemy_damage = self.mods.enemy_damage;
        m.enemy_speed = self.mods.enemy_speed;
        m.boss_hp = self.mods.boss_hp;
        m.spawn_rate = self.mods.spawn_rate;
        m.heal = self.mods.heal;
        m.witch = false;
        self.others.push(Hero {
            player,
            weapons: Vec::new(),
            passives: Vec::new(),
            projectiles: Vec::new(),
            zones: Vec::new(),
            char_stats: c.stats.clone(),
            mods: m,
            pending_levels: 0,
            pending_chests: Vec::new(),
            used_partners: Vec::new(),
            locked_weapons: Vec::new(),
        });
        let back = self.player.slot;
        self.front(slot);
        self.recompute_stats();
        self.player.hp = self.player.max_hp;
        if let Some(id) = self.content.weapon_index(&c.weapon) {
            self.add_weapon(id);
        }
        self.front(back);
    }

    /// Coop : le joueur au premier plan ne s'eloigne pas a plus de `LEASH` du centre des autres.
    fn leash(&mut self) {
        if self.others.is_empty() || !self.player.alive {
            return;
        }
        let others: Vec<Vec2> = self.others.iter().filter(|h| h.player.alive).map(|h| h.player.pos).collect();
        if others.is_empty() {
            return;
        }
        let c = others.iter().copied().sum::<Vec2>() / others.len() as f32;
        let d = self.player.pos - c;
        if d.length() > LEASH {
            self.player.pos = c + d.normalize() * LEASH;
        }
    }

    /// Coop : chaque joueur gagne les rubis ramasses par tous (son bonus d'experience compris).
    fn share_xp(&mut self) {
        let v = std::mem::take(&mut self.shared_xp);
        if v == 0 {
            return;
        }
        for s in 0..self.hero_count() {
            self.front(s as u8);
            self.gain_xp(v);
        }
    }

    /// Coop : un joueur a terre se releve (moitie de sa vie) si un allie debout reste pres de lui.
    fn update_revives(&mut self) {
        let heroes = self.heroes();
        for &(s, pos, alive) in &heroes {
            if alive || self.gone(s) {
                continue;
            }
            let helped = heroes.iter().any(|h| h.2 && h.0 != s && (h.1 - pos).length() < REVIVE_RANGE);
            self.front(s);
            let p = &mut self.player;
            p.revive = if helped { p.revive + DT } else { (p.revive - DT * 2.0).max(0.0) };
            if p.revive >= REVIVE_TIME {
                p.revive = 0.0;
                p.alive = true;
                p.hp = p.max_hp * 0.5;
                p.invuln = 2.5;
                self.events.push(Event::Revived { pos, slot: s, fairy: false });
            }
        }
    }

    fn gone(&self, slot: u8) -> bool {
        std::iter::once(&self.player).chain(self.others.iter().map(|h| &h.player)).any(|p| p.slot == slot && p.gone)
    }

    /// Coop : un joueur quitte la partie (meme pas dans tous les jeux) : il tombe pour de bon et ses
    /// choix en attente sont abandonnes.
    pub fn retire(&mut self, slot: u8) {
        let back = self.player.slot;
        self.front(slot);
        if self.player.slot == slot {
            let p = &mut self.player;
            p.gone = true;
            p.alive = false;
            p.swing = None;
            self.pending_levels = 0;
            self.pending_chests.clear();
        }
        self.front(back);
    }

    /// Coop : plus de joueurs, des ennemis plus resistants et plus nombreux (`COOP_HP`, `COOP_SPAWN` par
    /// joueur en plus). A appeler une fois tous les joueurs ajoutes.
    pub fn coop_scale(&mut self) {
        let extra = (self.hero_count() - 1) as f32;
        let (hp, spawn) = (1.0 + COOP_HP * extra, 1.0 + COOP_SPAWN * extra);
        for m in std::iter::once(&mut self.mods).chain(self.others.iter_mut().map(|h| &mut h.mods)) {
            m.enemy_hp *= hp;
            m.spawn_rate *= spawn;
        }
    }

    /// Empreinte de l'etat commun de la partie (coop : comparee entre les jeux pour reperer une
    /// desynchronisation). Les tirages prives des menus n'en font pas partie.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        mix(self.ticks);
        mix(self.time.to_bits() as u64);
        mix(self.horde.len() as u64);
        for i in 0..self.horde.len() {
            mix(self.horde.pos[i].x.to_bits() as u64 | (self.horde.pos[i].y.to_bits() as u64) << 32);
            mix(self.horde.hp[i].to_bits() as u64);
        }
        mix(self.pickups.len() as u64);
        let mut heroes: Vec<&Player> = std::iter::once(&self.player).chain(self.others.iter().map(|h| &h.player)).collect();
        heroes.sort_by_key(|p| p.slot);
        for p in heroes {
            mix(p.pos.x.to_bits() as u64 | (p.pos.y.to_bits() as u64) << 32);
            mix(p.hp.to_bits() as u64);
            mix(p.level as u64);
            mix(p.xp.to_bits() as u64);
        }
        h
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
        if let Some((at, t)) = self.portal_pending {
            let t = t - DT;
            self.portal_pending = if t <= 0.0 { None } else { Some((at, t)) };
            if t <= 0.0 {
                self.open_portal(at);
            }
        }
        let Some(at) = self.portal else { return };
        self.portal_t += DT;
        let reached = self.heroes().iter().any(|h| h.2 && (h.1 - at).length() < 14.0);
        if reached && self.portal_t > 1.2 {
            self.won = true;
            self.player.invuln = f32::MAX;
            for h in &mut self.others {
                h.player.invuln = f32::MAX;
            }
            self.events.push(Event::Victory { pos: at });
        }
    }

    /// Secondes entre la chute du boss final et l'ouverture de la porte de sortie.
    pub const PORTAL_DELAY: f32 = 5.0;

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
        // chaque joueur a ses propres delais de touche (sinon les armes d'un allie bloqueraient les siennes)
        let cd = slot.min(HIT_SLOTS - 1) + (self.player.slot as usize % MAX_HEROES) * HIT_SLOTS;
        if self.horde.hp[i] <= 0.0 || self.horde.hit_cd[i][cd] > 0 {
            return false;
        }
        self.horde.hit_cd[i][cd] = (interval / DT).round().clamp(1.0, 255.0) as u8;
        // la chance ajoute des points de critique aux armes qui peuvent en faire
        let luck = ((self.player.stats.luck - 1.0) * self.content.rules.luck_crit).max(0.0);
        let chance = if crit.1 > 1.0 { crit.0 + luck } else { 0.0 };
        let is_crit = chance > 0.0 && self.rng.chance(chance);
        let dmg = if is_crit { damage * crit.1 } else { damage };
        if let Some(st) = status {
            self.apply_status(i, st);
        }
        self.damage_enemy(i, dmg, is_crit, dir, knockback, weapon);
        true
    }

    pub fn apply_status(&mut self, i: usize, st: StatusDef) {
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
                // en cours : la plus forte reste ; terminee : la nouvelle remplace l'ancienne (la
                // force n'etait remise a zero que si la brulure finissait sur un pas de degats)
                let dps = dps * self.player.stats.might;
                let h = &mut self.horde;
                h.burn_dps[i] = if h.burn[i] > 0.0 { h.burn_dps[i].max(dps) } else { dps };
                h.burn[i] = h.burn[i].max(time * dur);
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
            self.on_death(i, dir);
        }
    }

    /// Mort d'un ennemi : butin, coffres, fin de boss, souffle de la Bob-omb.
    fn on_death(&mut self, i: usize, dir: Vec2) {
        let k = self.horde.kind[i];
        let pos = self.horde.pos[i];
        let boss = self.content.enemies[k as usize].boss;
        {
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
                    // boss final : la diseuse de bonne aventure remplace le grand coffre ; elle se tient
                    // devant (plus bas a l'ecran), la porte s'ouvre derriere elle quelques secondes apres
                    self.final_uid = None;
                    self.spawning = false;
                    self.add_npc(crate::npc::NpcKind::Fortune, pos + Vec2::new(0.0, 12.0));
                    self.portal_pending = Some((pos - Vec2::new(0.0, 30.0), Self::PORTAL_DELAY));
                } else {
                    self.pickups.spawn(PickupKind::BigChest, 0, pos, Vec2::ZERO, 120.0);
                    self.events.push(Event::ChestDropped { pos, big: true });
                }
                // pluie de rubis
                for _ in 0..12 {
                    let v = self.rng.dir() * self.rng.range(40.0, 110.0);
                    self.pickups.spawn(PickupKind::RupeeRed, 20, pos, v, self.rng.range(80.0, 140.0));
                }
            } else if self.is_champion(i) {
                self.encounter = None;
                let (a, b) = self.content.rules.encounter.every;
                self.encounter_t = self.rng.range(a, b);
                self.pending_rewards += 1;
                self.events.push(Event::EncounterWon { pos });
                for _ in 0..6 {
                    let v = self.rng.dir() * self.rng.range(40.0, 100.0);
                    self.pickups.spawn(PickupKind::RupeeBlue, 5, pos, v, self.rng.range(80.0, 130.0));
                }
            } else if elite {
                self.pickups.spawn(PickupKind::Chest, 0, pos, dir * 20.0, 110.0);
                self.events.push(Event::ChestDropped { pos, big: false });
                let xp = self.content.rules.elite_xp;
                self.pickups.spawn(PickupKind::rupee_for(xp), xp, pos, self.rng.dir() * 40.0, 90.0);
            } else if Some(k) == self.bomb_kind {
                self.drop_loot(k, pos, dir);
                self.screen_clear(pos);
            } else if Some(k) == self.thief_kind {
                self.drop_loot(k, pos, dir);
                self.thief_defeated(pos);
            } else {
                self.drop_loot(k, pos, dir);
            }
        }
    }

    /// Souffle de la Bob-omb : tous les ennemis visibles meurent (sauf les boss) ; en coop, visibles
    /// par au moins un joueur debout.
    pub fn screen_clear(&mut self, at: Vec2) {
        let lim = self.view_half + Vec2::splat(16.0);
        let mut eyes: Vec<Vec2> = self.heroes().into_iter().filter(|h| h.2).map(|h| h.1).collect();
        if eyes.is_empty() {
            eyes.push(self.player.pos);
        }
        let mut count = 0;
        for j in 0..self.horde.len() {
            if self.horde.hp[j] <= 0.0 {
                continue;
            }
            let k = self.horde.kind[j];
            if self.content.enemies[k as usize].boss || Some(k) == self.bomb_kind || self.is_champion(j) {
                continue;
            }
            let e = self.horde.pos[j];
            if !eyes.iter().any(|&p| (e - p).x.abs() <= lim.x && (e - p).y.abs() <= lim.y) {
                continue;
            }
            self.horde.hp[j] = 0.0;
            let dir = (self.horde.pos[j] - at).normalize_or(Vec2::Y);
            self.on_death(j, dir);
            count += 1;
        }
        self.events.push(Event::ScreenClear { pos: at, count });
    }

    /// Medaillon : gele les ennemis visibles (les boss un instant seulement).
    fn medallion(&mut self, at: Vec2) {
        let lim = self.view_half + Vec2::splat(16.0);
        let pp = self.player.pos;
        for j in 0..self.horde.len() {
            let d = (self.horde.pos[j] - pp).abs();
            if self.horde.hp[j] <= 0.0 || d.x > lim.x || d.y > lim.y {
                continue;
            }
            let boss = self.content.enemies[self.horde.kind[j] as usize].boss;
            self.horde.freeze[j] = self.horde.freeze[j].max(if boss { 1.5 } else { 6.0 });
        }
        self.events.push(Event::Medallion { pos: at });
    }

    /// Decor cassable : touche par l'epee, les projectiles, les explosions, les orbites et le dash.
    /// Coups du joueur au premier plan qui cassent le decor : (epee, dash, projectiles), (armes en orbite
    /// et auras). Ordre du solo garde : premiers coups, explosions, puis orbites.
    fn hero_prop_hits(&self, hits: &mut PropHits) {
        let pp = self.player.pos;
        if let Some(sw) = self.player.swing {
            let d = Vec2::from_angle(sw.angle_at(sw.t));
            hits.0.push((pp + d * sw.radius * 0.6, sw.radius * 0.45));
        }
        if self.player.dashing() {
            hits.0.push((pp, self.player.radius + 4.0));
        }
        for p in &self.projectiles {
            if p.radius > 0.0 && p.z < 4.0 {
                hits.0.push((p.pos + Vec2::new(0.0, 6.0), p.radius));
            }
        }
        for slot in 0..self.weapons.len() {
            let w = self.weapons[slot];
            match self.content.weapons[w.id as usize].kind {
                WeaponKind::Orbit if w.active > 0.0 => {
                    for pt in w.orbit_points(pp - Vec2::new(0.0, 6.0), w.phase) {
                        hits.1.push((pt, 8.0));
                    }
                }
                WeaponKind::Aura => hits.1.push((pp, w.stats.area * 0.8)),
                _ => {}
            }
        }
    }

    fn update_props(&mut self, (first, last): PropHits) {
        let mut hits = first;
        for e in &self.events[self.ev0..] {
            match *e {
                Event::Explosion { pos, radius, .. } | Event::Slam { pos, radius, .. } | Event::Pillar { pos, radius, .. } => {
                    hits.push((pos, radius))
                }
                _ => {}
            }
        }
        hits.extend(last);
        for (at, r) in hits {
            self.hit_props(at, r.min(120.0));
        }
    }

    /// Casse les elements cassables touches par un cercle.
    pub fn hit_props(&mut self, at: Vec2, r: f32) {
        use crate::props::{prop_at, CELL};
        let (x0, y0) = (((at.x - r - 8.0) / CELL).floor() as i32, ((at.y - r - 8.0) / CELL).floor() as i32);
        let (x1, y1) = (((at.x + r + 8.0) / CELL).floor() as i32, ((at.y + r + 8.0) / CELL).floor() as i32);
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                if self.broken.contains(&(cx, cy)) {
                    continue;
                }
                let Some(p) = prop_at(&self.map.props, self.prop_seed, cx, cy) else { continue };
                let def = &self.map.props[p.kind as usize];
                if !def.breakable {
                    continue;
                }
                // le centre visuel est un peu au-dessus du pied
                let c = p.pos - Vec2::new(0.0, 4.0);
                if (c - at).length() > r + def.radius.max(6.0) {
                    continue;
                }
                self.broken.insert((cx, cy));
                self.events.push(Event::PropBroken { pos: p.pos, kind: p.kind });
                self.prop_drop(c);
            }
        }
    }

    /// Butin d'un element casse (la chance rend le « rien » plus rare).
    fn prop_drop(&mut self, pos: Vec2) {
        let luck = self.player.stats.luck.max(0.1);
        let weight = |d: DropDef, w: f32| if d == DropDef::Nothing { w / luck } else { w };
        let drops = self.content.rules.prop_drops.clone();
        let total: f32 = drops.iter().map(|&(d, w)| weight(d, w)).sum();
        if total <= 0.0 {
            return;
        }
        let mut u = self.rng.f() * total;
        let mut pick = DropDef::Nothing;
        for &(d, w) in &drops {
            let w = weight(d, w);
            if u < w {
                pick = d;
                break;
            }
            u -= w;
        }
        let (kind, value) = match pick {
            DropDef::Nothing => return,
            DropDef::RupeeGreen => (PickupKind::RupeeGreen, 1),
            DropDef::RupeeBlue => (PickupKind::RupeeBlue, 5),
            DropDef::RupeeRed => (PickupKind::RupeeRed, 20),
            DropDef::Heart => (PickupKind::Heart, 0),
            DropDef::Magnet => (PickupKind::Magnet, 0),
            DropDef::Fairy => (PickupKind::Fairy, 0),
            DropDef::Medallion => (PickupKind::Medallion, 0),
        };
        let v = self.rng.dir() * 14.0;
        self.pickups.spawn(kind, value, pos, v, 95.0);
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
        let slot = p.slot;
        self.events.push(Event::PlayerHurt { pos, damage: dmg, slot });
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
            self.events.push(Event::Revived { pos, slot, fairy: true });
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
            self.events.push(Event::PlayerDied { pos, slot });
        }
    }

    fn update_pickups(&mut self) {
        // joueurs : (place, position, aimant, debout) ; chaque objet va au plus proche (solo : le joueur)
        let mut heroes: Vec<(u8, Vec2, f32, bool)> = std::iter::once(&self.player)
            .chain(self.others.iter().map(|h| &h.player))
            .map(|p| (p.slot, p.pos, p.magnet, p.alive))
            .collect();
        heroes.sort_by_key(|h| h.0);
        let mut i = 0;
        while i < self.pickups.len() {
            let at = self.pickups.pos[i];
            let near = heroes
                .iter()
                .filter(|h| h.3)
                .min_by(|a, b| (a.1 - at).length_squared().total_cmp(&(b.1 - at).length_squared()))
                .or(heroes.first())
                .copied();
            let Some((slot, pp, magnet, alive)) = near else { break };
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
                    self.front(slot);
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
                self.front(slot);
                self.collect(kind, value, pos);
                continue;
            }
            i += 1;
        }
        self.front(0);
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
            PickupKind::Magnet => self.vacuum(),
            PickupKind::Fairy => {
                let p = &mut self.player;
                p.hp = p.max_hp;
            }
            PickupKind::Medallion => self.medallion(pos),
            // coop : tous les joueurs en profitent, a la fin du pas (`share_xp`)
            _ if self.coop() => self.shared_xp += value,
            _ => self.gain_xp(value),
        }
    }

    pub(crate) fn gain_xp(&mut self, value: u32) {
        self.player.rupees += value;
        self.player.xp += value as f32 * self.mods.xp * self.player.stats.growth;
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
        self.events.push(Event::LevelUp { level: p.level, pos, slot: p.slot });
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
        if let Some(bk) = self.bomb_kind {
            self.bomb_t -= DT;
            if self.bomb_t <= 0.0 {
                let (a, b) = self.content.rules.bomb_every;
                self.bomb_t = self.rng.range(a, b);
                if self.horde.alive[bk as usize] == 0 && self.horde.len() < max {
                    let bias = Some(self.player.vel);
                    let p = self.offscreen_point(bias);
                    let i = self.spawn_enemy(bk, p);
                    self.toughen_encounter(i);
                    self.events.push(Event::BombSpawned { pos: p });
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
                self.horde.power[i] = r.elite_damage * self.mods.enemy_damage * self.map.power;
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
                        let i = self.spawn_boss(&b, k, p);
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
                        let i = self.spawn_boss(&b, k, p);
                        let uid = self.horde.uid[i];
                        self.boss_uid = Some(uid);
                        self.final_uid = Some(uid);
                        self.events.push(Event::BossSpawned { pos: p, kind: k, last: true });
                    }
                }
            }
        }
    }

    /// Rencontre (Bob-omb, voleur) : trois fois la vie d'un ennemi ordinaire du stage (moyenne des
    /// vagues), avec la meme progression (temps, difficulte, puissance du stage).
    pub(crate) fn toughen_encounter(&mut self, i: usize) {
        let base = self.content.enemies[self.horde.kind[i] as usize].hp;
        let hps: Vec<f32> = self.map.waves.iter().filter_map(|w| self.content.enemy_index(&w.enemy)).map(|k| self.content.enemies[k as usize].hp).collect();
        let mob = if hps.is_empty() { base } else { hps.iter().sum::<f32>() / hps.len() as f32 };
        self.horde.hp[i] *= 3.0 * mob / base.max(0.1);
        self.horde.max_hp[i] = self.horde.hp[i];
    }

    /// Les ennemis distances sont replaces devant le joueur (la pression reste constante).
    fn recycle_far(&mut self) {
        let lim = self.view_half + Vec2::splat(140.0);
        let pp = self.player.pos;
        let heroes: Vec<Vec2> = self.heroes().into_iter().map(|h| h.1).collect();
        // ecart au joueur le plus proche, axe par axe (solo : au joueur)
        let gap = |p: Vec2| heroes.iter().map(|&h| (p - h).abs()).fold(Vec2::splat(f32::MAX), |m, d| if d.x.max(d.y) < m.x.max(m.y) { d } else { m });
        for i in 0..self.horde.len() {
            let d = gap(self.horde.pos[i]);
            if d.x > lim.x || d.y > lim.y {
                if self.content.enemies[self.horde.kind[i] as usize].boss || self.is_champion(i) || Some(self.horde.kind[i]) == self.thief_kind {
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
        let _ = pp;
        while i < self.pickups.len() {
            let d = gap(self.pickups.pos[i]);
            if (d.x > lim2.x || d.y > lim2.y) && !self.pickups.kind[i].is_chest() && !self.pickups.kind[i].is_special() {
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
