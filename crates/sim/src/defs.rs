//! Definitions de contenu, lues depuis `data/*.ron`. Ajouter un ennemi, une arme, un objet
//! ou une vague = modifier ces fichiers, sans toucher au code.
use serde::Deserialize;
use std::collections::BTreeMap;

fn one() -> f32 {
    1.0
}
fn one_u() -> u32 {
    1
}

/// Personnage jouable.
#[derive(Clone, Debug, Deserialize)]
pub struct CharacterDef {
    pub name: String,
    /// Nom du sprite dans l'atlas (animations idle_*/walk_*).
    pub sprite: String,
    /// Points de vie (1 coeur = 10).
    pub max_hp: f32,
    /// Vitesse de marche (pixels/s).
    pub speed: f32,
    /// Rayon de collision.
    #[serde(default = "default_player_radius")]
    pub radius: f32,
    /// Rayon d'attraction des objets.
    pub magnet: f32,
    pub dash: DashDef,
    /// Arme de depart (cle de `weapons.ron`).
    pub weapon: String,
    /// Sprite de l'epee pour l'affichage (ex. "sword1").
    #[serde(default)]
    pub weapon_sprite: String,
    /// Les images de profil regardent a gauche (retournees pour aller a droite).
    #[serde(default)]
    pub side_faces_left: bool,
    /// Bonus de depart (meme format que les objets passifs).
    #[serde(default)]
    pub stats: Vec<(Stat, f32)>,
    /// Rang dans le choix du personnage.
    #[serde(default)]
    pub order: u32,
    /// Condition de deblocage (aucune : disponible des le depart).
    #[serde(default)]
    pub unlock: Option<UnlockDef>,
    /// Couleurs au choix (meme personnage, planches identiques) : (sprite, couleur) ; la cle de texte
    /// est `skin.<couleur>`. Le choix est garde dans le profil et remplace `sprite`.
    #[serde(default)]
    pub skins: Vec<(String, String)>,
    /// Univers (page de l'ecran des personnages, cle `group.<univers>`) ; vide : Zelda.
    #[serde(default)]
    pub group: String,
    /// Auteur du sprite (personnages du Randomizer, 0.6.5).
    #[serde(default)]
    pub credit: String,
    /// Identifiant (nom du fichier ou cle), rempli au chargement.
    #[serde(skip)]
    pub id: String,
}

/// Condition de deblocage d'un personnage (verifiee par le jeu, d'apres le profil).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub enum UnlockDef {
    /// Gagner ce stage.
    Win(String),
    /// Vaincre n ennemis au total.
    Kills(u64),
    /// Vaincre n boss au total.
    Bosses(u32),
    /// Atteindre ce niveau dans une partie.
    Level(u32),
    /// Vaincre cet ennemi (identifiant de `enemies.ron`) au moins une fois.
    Slain(String),
    /// Debloquer tous les autres personnages d'origine (rang < 1000, hors boutique) : reste le dernier.
    AllOthers,
    /// Acheter a la boutique (onglet des personnages) pour ce prix en rubis rouges.
    Shop(u64),
}

fn default_player_radius() -> f32 {
    5.0
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct DashDef {
    pub speed: f32,
    pub duration: f32,
    pub cooldown: f32,
}

/// Statistiques du joueur modifiees par les objets passifs.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stat {
    /// Degats (+0.1 = +10 %).
    Might,
    /// Taille des attaques (+0.1 = +10 %).
    Area,
    /// Temps de recharge (-0.08 = -8 %).
    Cooldown,
    /// Projectiles supplementaires.
    Amount,
    /// Duree des effets (+0.1 = +10 %).
    Duration,
    /// Vitesse des projectiles (+0.1 = +10 %).
    Speed,
    /// Vitesse de marche (+0.1 = +10 %).
    MoveSpeed,
    /// Vie maximale (points ; 10 = 1 coeur).
    MaxHp,
    /// Degats recus en moins (points).
    Armor,
    /// Rayon d'attraction des objets (+0.3 = +30 %).
    Magnet,
    /// Recharge du dash (-0.1 = -10 %).
    DashCooldown,
    /// Vie rendue par seconde.
    Regen,
    /// Chance (+0.1 = +10 %) : coups critiques, butin du decor.
    Luck,
    /// Croissance (+0.1 = +10 % d'experience).
    Growth,
}

/// Objet passif : bonus de statistiques par niveau.
#[derive(Clone, Debug, Deserialize)]
pub struct PassiveDef {
    /// Icone (sprite de l'atlas).
    pub icon: String,
    /// Bonus ajoutes a chaque niveau (entree 0 = niveau 1).
    pub levels: Vec<Vec<(Stat, f32)>>,
    /// Poids dans le tirage des choix.
    #[serde(default = "one")]
    pub weight: f32,
}

impl PassiveDef {
    pub fn max_level(&self) -> u32 {
        self.levels.len() as u32
    }
}

/// Famille d'arme (comportement).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum WeaponKind {
    /// Coup en arc autour du joueur (epee).
    Swing,
    /// Projectiles vers les ennemis proches (arc, baguettes).
    Projectile,
    /// Aller-retour (boomerang).
    Boomerang,
    /// Lancer en cloche qui explose (bombes).
    Lob,
    /// Frappe au sol autour du joueur (marteau, Quake).
    Slam,
    /// Objets en orbite (Canne de Byrna, fees).
    Orbit,
    /// Colonnes qui frappent des ennemis a l'ecran (Bombos).
    Pillars,
    /// Frappe tout l'ecran (Ether).
    Storm,
    /// Nuage autour du joueur qui blesse en continu (Poudre magique).
    Aura,
    /// Bloc pose au sol qui eclate en rayons (Canne de Somaria).
    Somaria,
    /// Projectiles qui poursuivent les ennemis (abeilles).
    Bees,
    /// Saut invincible, onde de choc a l'atterrissage (Plume de Roc). `duration` = duree du saut.
    Jump,
    /// Tornades qui errent lentement et traversent les ennemis (Ocarina).
    Tornado,
}

/// Effet d'etat inflige au contact.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum StatusDef {
    /// Brulure : degats par seconde pendant `time`.
    Burn { dps: f32, time: f32 },
    /// Gel : immobile (et teinte bleue).
    Freeze { time: f32 },
    /// Etourdi : immobile.
    Stun { time: f32 },
}

/// Zone au sol laissee par un projectile (flammes).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ZoneDef {
    pub radius: f32,
    pub time: f32,
    pub dps: f32,
}

/// Caracteristiques de base d'une arme (niveau 1).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct WeaponStats {
    pub damage: f32,
    /// Temps entre deux declenchements (s).
    pub cooldown: f32,
    #[serde(default = "one_u")]
    pub amount: u32,
    /// Taille : portee du coup, rayon d'explosion, d'orbite, de colonne.
    pub area: f32,
    /// Vitesse des projectiles (pixels/s) ou de rotation (radians/s).
    #[serde(default)]
    pub speed: f32,
    /// Duree : balayage, vie du projectile, meche, orbite active.
    #[serde(default)]
    pub duration: f32,
    /// Ennemis traverses (projectile) ou touches par coup (epee) ; 0 = illimite.
    #[serde(default)]
    pub pierce: u32,
    #[serde(default)]
    pub knockback: f32,
    #[serde(default)]
    pub crit_chance: f32,
    #[serde(default = "crit_mult_default")]
    pub crit_mult: f32,
    /// Delai minimal entre deux touches de cette arme sur un meme ennemi (s).
    #[serde(default = "hit_interval_default")]
    pub hit_interval: f32,
}

fn crit_mult_default() -> f32 {
    1.5
}
fn hit_interval_default() -> f32 {
    0.4
}

/// Amelioration apportee par un niveau d'arme.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Mod {
    /// Degats de base (+valeur).
    Damage(f32),
    /// Recharge (-0.1 = -10 %).
    Cooldown(f32),
    /// Projectiles (+n).
    Amount(u32),
    /// Taille (+0.1 = +10 %).
    Area(f32),
    /// Vitesse (+0.1 = +10 %).
    Speed(f32),
    /// Duree (+0.1 = +10 %).
    Duration(f32),
    /// Ennemis traverses ou touches (+n).
    Pierce(u32),
    /// Recul (+0.1 = +10 %).
    Knockback(f32),
    /// Chance de critique (+0.05 = +5 points).
    Crit(f32),
}

/// Evolution : arme au niveau maximal + objet passif possede -> nouvelle arme (dans un coffre).
#[derive(Clone, Debug, Deserialize)]
pub struct EvolutionDef {
    pub passive: String,
    pub into: String,
}

/// Faisceau tire a chaque coup (Epee de legende).
#[derive(Clone, Debug, Deserialize)]
pub struct BeamDef {
    pub sprite: String,
    /// Part des degats du coup.
    pub damage: f32,
    pub speed: f32,
    pub life: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeaponDef {
    pub kind: WeaponKind,
    /// Icone (sprite de l'atlas).
    pub icon: String,
    /// Sprite du projectile ou de l'effet.
    #[serde(default)]
    pub sprite: String,
    pub base: WeaponStats,
    /// Ameliorations des niveaux 2, 3...
    #[serde(default)]
    pub levels: Vec<Vec<Mod>>,
    #[serde(default)]
    pub evolution: Option<EvolutionDef>,
    /// Fusion ultime (Master Sword : epee + bouclier) : possible seulement une fois l'arme evoluee
    /// debloquee (son `unlock`).
    #[serde(default)]
    pub ultimate: Option<EvolutionDef>,
    /// Arme evoluee : obtenue seulement par evolution.
    #[serde(default)]
    pub evolved: bool,
    /// Arme a debloquer : identifiant de l'ennemi (mini-boss) a vaincre une premiere fois. Avant,
    /// elle n'est jamais proposee ; ensuite, elle peut l'etre dans toutes les parties.
    #[serde(default)]
    pub unlock: Option<String>,
    #[serde(default)]
    pub status: Option<StatusDef>,
    #[serde(default)]
    pub zone: Option<ZoneDef>,
    #[serde(default)]
    pub beam: Option<BeamDef>,
    /// Epee : ouverture de l'arc (degres). Projectiles : ecart entre projectiles (degres).
    #[serde(default)]
    pub arc: f32,
    /// Portee de visee automatique.
    #[serde(default = "range_default")]
    pub range: f32,
    /// Rotation du sprite (radians/s ; 0 = oriente selon la vitesse).
    #[serde(default)]
    pub spin: f32,
    #[serde(default = "one")]
    pub weight: f32,
    /// Son de declenchement (cle d'evenement de `audio.ron`).
    #[serde(default)]
    pub sfx: String,
    /// Chaine dessinee entre le joueur et le projectile (grappin).
    #[serde(default)]
    pub chain: bool,
    /// Canne de Somaria : nombre de rayons a l'eclatement du bloc.
    #[serde(default = "rays_default")]
    pub rays: u32,
    /// Projectiles : tous en eventail vers la cible la plus proche (lance-pierre), ecart = `arc`.
    #[serde(default)]
    pub fan: bool,
    /// Teinte de l'effet (nuage des armes Aura : grains et bord), RVB 0..1.
    #[serde(default)]
    pub tint: Option<(f32, f32, f32)>,
}

fn rays_default() -> u32 {
    4
}

fn range_default() -> f32 {
    140.0
}

impl WeaponDef {
    pub fn max_level(&self) -> u32 {
        self.levels.len() as u32 + 1
    }

    /// Toutes les fusions de cette arme : la normale puis l'ultime.
    pub fn recipes(&self) -> impl Iterator<Item = &EvolutionDef> {
        self.evolution.iter().chain(self.ultimate.iter())
    }
}

/// Projectile ennemi.
#[derive(Clone, Debug, Deserialize)]
pub struct ShotDef {
    pub sprite: String,
    pub speed: f32,
    pub damage: f32,
    /// Lance en cloche, explose a l'arrivee (rayon `radius`).
    #[serde(default)]
    pub lob: bool,
    #[serde(default = "shot_radius_default")]
    pub radius: f32,
    #[serde(default = "one_u")]
    pub count: u32,
    /// Ecart entre les tirs (degres).
    #[serde(default)]
    pub spread: f32,
    /// Tir en couronne : `count` projectiles repartis tout autour (ignore `spread`).
    #[serde(default)]
    pub ring: bool,
}

fn shot_radius_default() -> f32 {
    4.0
}

/// Action d'un patron de boss.
#[derive(Clone, Debug, Deserialize)]
pub enum BossAction {
    /// Poursuite pendant n secondes.
    Walk(f32),
    /// Arret (annonce), puis charge en ligne droite.
    Charge { windup: f32, speed: f32, time: f32 },
    /// Arret (annonce), puis tir.
    Throw { windup: f32, shot: ShotDef },
    /// Appelle des renforts autour de lui.
    Summon { enemy: String, count: u32 },
    /// Bond sur la position du joueur ; onde de choc a l'atterrissage (rayon, degats).
    Leap { windup: f32, time: f32, radius: f32, damage: f32 },
    /// Disparait (annonce) puis reapparait a `range` pixels du joueur.
    Teleport { windup: f32, range: f32 },
}

#[derive(Clone, Debug, Deserialize)]
pub enum Behavior {
    /// Fonce droit sur le joueur.
    Chase,
    /// Vole en zigzag vers le joueur (Keese) : ignore la separation.
    Flutter { amp: f32, freq: f32 },
    /// Bonds periodiques vers le joueur (Tektite).
    Hop { rest: f32, jump: f32, speed: f32 },
    /// Bonds periodiques pour fuir le joueur quand il est a moins de `range`, sinon flane (Pikit).
    Flee { rest: f32, jump: f32, speed: f32, range: f32 },
    /// Avance en flanant (Popo, Buzz Blob) : direction qui derive.
    Wander { turn: f32 },
    /// S'approche, s'arrete (annonce), puis charge en ligne droite.
    Charger { range: f32, windup: f32, speed: f32, time: f32, rest: f32 },
    /// Garde ses distances et tire.
    Shooter { range: f32, keep: f32, cooldown: f32, windup: f32, shot: ShotDef },
    /// Patron de boss : enchaine les actions en boucle.
    Boss { actions: Vec<BossAction> },
}

/// Facon d'animer un ennemi (rendu uniquement).
#[derive(Clone, Debug, Deserialize)]
pub enum EnemyAnims {
    /// Animations directionnelles `<prefix>_down/_side/_up` (le cote est retourne pour l'autre sens).
    /// `attack` : prefixe des animations d'attaque (annonce, tir).
    Dir4 {
        prefix: String,
        side_faces_left: bool,
        #[serde(default)]
        attack: Option<String>,
    },
    /// Une seule animation.
    Single(String),
    /// Repos / saut.
    Hop { idle: String, jump: String },
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemyDef {
    pub sprite: String,
    pub anims: EnemyAnims,
    pub hp: f32,
    pub speed: f32,
    /// Degats au contact (1 coeur = 10).
    pub damage: f32,
    pub radius: f32,
    /// Resistance au recul (1 = normal).
    #[serde(default = "one")]
    pub mass: f32,
    /// Valeur en rubis lachee a la mort.
    pub xp: u32,
    pub behavior: Behavior,
    /// Vole (pas de separation).
    #[serde(default)]
    pub flying: bool,
    /// Taille de l'ombre (0 petite, 1 moyenne, 2 grande).
    #[serde(default)]
    pub shadow: u8,
    /// Boss : barre de vie, jamais recycle, grand coffre.
    #[serde(default)]
    pub boss: bool,
    /// Agrandissement a l'affichage (entier pour rester net : 2 = boss geant).
    #[serde(default = "one")]
    pub scale: f32,
    /// Faux : absent du bestiaire tant qu'il n'a pas ete vaincu (ennemi qu'aucun stage n'utilise).
    #[serde(default = "yes")]
    pub bestiary: bool,
}

/// Flux continu d'apparition entre deux instants.
#[derive(Clone, Debug, Deserialize)]
pub struct WaveDef {
    pub enemy: String,
    /// Debut et fin (secondes de partie).
    pub from: f32,
    pub to: f32,
    /// Ennemis par seconde au debut et a la fin (interpolation lineaire).
    pub rate: (f32, f32),
    /// Maximum d'ennemis de ce type en vie.
    pub cap: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Pattern {
    /// Cercle complet autour du joueur.
    Ring,
    /// Groupe compact hors de l'ecran.
    Pack,
}

/// Evenement ponctuel : une vague d'un coup.
#[derive(Clone, Debug, Deserialize)]
pub struct BurstDef {
    pub at: f32,
    pub enemy: String,
    pub count: u32,
    pub pattern: Pattern,
}

/// Apparition unique a un instant donne.
#[derive(Clone, Debug, Deserialize)]
pub struct SpawnAt {
    pub at: f32,
    pub enemy: String,
    /// Boss : vie de base pour ce stage (remplace celle de la fiche ; un meme boss peut revenir plus
    /// fort dans un stage plus avance).
    #[serde(default)]
    pub hp: Option<f32>,
}

/// Element de decor place au hasard (voir `props.rs`).
#[derive(Clone, Debug, Deserialize)]
pub struct PropDef {
    /// Sprite (rendu) ; `variants` > 1 : animations "0", "1"... choisies au hasard.
    pub sprite: String,
    #[serde(default = "one_u32")]
    pub variants: u32,
    /// Probabilite d'un element par case de 48 px.
    pub density: f32,
    /// Rayon de collision (pixels).
    #[serde(default)]
    pub radius: f32,
    /// Bloque le joueur et la horde.
    #[serde(default)]
    pub solid: bool,
    /// Teinte (rouge, vert, bleu, force) : arbre givre, rocher sombre...
    #[serde(default)]
    pub tint: Option<(f32, f32, f32, f32)>,
    /// Se casse au moindre coup (buissons, pots) et peut laisser un objet.
    #[serde(default)]
    pub breakable: bool,
    /// Animation jouee a la casse (sprite de l'atlas).
    #[serde(default)]
    pub break_fx: String,
}

fn one_u32() -> u32 {
    1
}

/// Meteo d'une carte (rendu et ambiance).
#[derive(Clone, Copy, Debug, Deserialize, Default, PartialEq, Eq)]
pub enum Weather {
    #[default]
    Clear,
    /// Pluie et eclairs.
    Rain,
    /// Brouillard qui derive.
    Fog,
    /// Neige.
    Snow,
    /// Vent de sable.
    Sand,
    /// Obscurite : seul un halo autour du joueur reste clair.
    Dark,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MapDef {
    pub name: String,
    /// Rang dans le choix des stages (1 = premier) ; 0 = hors selection (arene d'essai).
    #[serde(default)]
    pub order: u32,
    /// Boss final : a sa mort, la porte de sortie s'ouvre. Sans boss final, elle s'ouvre a `duration`.
    #[serde(default)]
    pub final_boss: Option<SpawnAt>,
    /// Musique du boss final (sinon `boss_music`).
    #[serde(default)]
    pub final_music: String,
    /// Decor : arbres, rochers...
    #[serde(default)]
    pub props: Vec<PropDef>,
    #[serde(default)]
    pub weather: Weather,
    /// Ambiance sonore (cle de `audio.ron`).
    #[serde(default)]
    pub ambience: Option<String>,
    /// Teinte multiplicative du monde (nuit, crepuscule...).
    #[serde(default)]
    pub tint: Option<(f32, f32, f32, f32)>,
    /// Musique (cle de `audio.ron`).
    pub music: String,
    /// Musique du boss.
    #[serde(default)]
    pub boss_music: String,
    /// Jeu de tuiles du sol.
    pub ground: String,
    #[serde(default)]
    pub waves: Vec<WaveDef>,
    #[serde(default)]
    pub bursts: Vec<BurstDef>,
    /// Ennemis d'elite (contour dore, lachent un coffre).
    #[serde(default)]
    pub elites: Vec<SpawnAt>,
    /// Boss de la carte.
    #[serde(default)]
    pub boss: Option<SpawnAt>,
    /// Plafond global d'ennemis en vie.
    pub max_enemies: u32,
    /// Duree de la partie (s).
    #[serde(default = "default_duration")]
    pub duration: f32,
    /// Difficulte du stage : vie et degats des ennemis ordinaires (x1 au premier stage).
    #[serde(default = "one")]
    pub power: f32,
}

fn default_duration() -> f32 {
    600.0
}

/// Regles globales de progression.
#[derive(Clone, Debug, Deserialize)]
pub struct RulesDef {
    /// Experience pour passer du niveau 1 au 2, puis increment par niveau.
    pub xp_first: f32,
    pub xp_step: f32,
    /// Increment supplementaire apres le niveau 20.
    pub xp_step_late: f32,
    /// Degats de toutes les armes en plus (0.15 = +15 %).
    #[serde(default)]
    pub damage_bonus: f32,
    /// Onde de choc a la montee de niveau (repousse sans blesser).
    pub levelup_wave_radius: f32,
    pub levelup_wave_force: f32,
    /// Invulnerabilite apres un coup recu (s).
    pub hurt_iframes: f32,
    /// Chance qu'un ennemi lache un petit coeur.
    pub heart_drop: f32,
    pub heart_heal: f32,
    /// Au-dela, les nouveaux rubis s'ajoutent aux anciens (perf).
    pub max_pickups: u32,
    /// Vie des ennemis : +x par minute de partie.
    #[serde(default)]
    pub hp_growth: f32,
    /// Elites : multiplicateurs de vie et de degats, rubis lâches.
    pub elite_hp: f32,
    pub elite_damage: f32,
    pub elite_xp: u32,
    /// Emplacements d'armes et d'objets passifs.
    pub max_weapons: usize,
    pub max_passives: usize,
    /// Nombre de choix a chaque niveau.
    pub choices: usize,
    /// Coffre sans evolution possible : nombre d'ameliorations (min, max).
    pub chest_upgrades: (u32, u32),
    /// Soin propose quand tout est au maximum (points).
    pub fallback_heal: f32,
    /// Rubis proposes quand tout est au maximum.
    pub fallback_rupees: u32,
    /// Bob-omb : ennemi rare qui, vaincu, souffle tous les ennemis a l'ecran (hors boss).
    #[serde(default)]
    pub bomb_enemy: String,
    /// Delai entre deux Bob-ombs (s, minimum et maximum) ; la premiere arrive apres un delai tire de meme.
    #[serde(default = "bomb_every_default")]
    pub bomb_every: (f32, f32),
    /// Butin du decor cassable : (objet, poids).
    #[serde(default)]
    pub prop_drops: Vec<(DropDef, f32)>,
    /// Chance : points de critique ajoutes par +1 de chance (0.3 : +10 % de chance = +3 points).
    #[serde(default = "luck_crit_default")]
    pub luck_crit: f32,
    /// Rencontres : champion errant (elite renforcee) a aller chercher ; vaincu, il donne une recompense.
    #[serde(default)]
    pub encounter: EncounterRules,
    /// Tirs ennemis : vitesse (multiplicateur des vitesses de enemies.ron) et zone de contact
    /// (part de la taille visible du projectile : 0.9 = 10 % plus petite que ce qu'on voit).
    #[serde(default = "one_f")]
    pub shot_speed: f32,
    #[serde(default = "one_f")]
    pub shot_hitbox: f32,
    /// Pikit voleur : ennemi rare qui fuit ; vaincu, il rapporte au joueur tous les rubis au sol.
    #[serde(default)]
    pub thief: ThiefRules,
}

fn one_f() -> f32 {
    1.0
}

/// Reglages du Pikit voleur de rubis.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ThiefRules {
    /// Ennemi utilise (vide : pas de Pikit).
    pub enemy: String,
    /// Premiere apparition (s de partie), puis delai entre deux (s, minimum et maximum).
    pub first: f32,
    pub every: (f32, f32),
    /// Temps avant qu'il ne reparte s'il n'est pas vaincu (s).
    pub life: f32,
    /// Vie : multiplie la vie de enemies.ron (qui grandit deja avec le temps).
    pub hp: f32,
    /// Il s'enfuit quand le joueur est plus pres que cette distance (pixels).
    pub flee: f32,
}

impl Default for ThiefRules {
    fn default() -> Self {
        ThiefRules { enemy: String::new(), first: 120.0, every: (150.0, 200.0), life: 45.0, hp: 1.0, flee: 110.0 }
    }
}

/// Reglages des rencontres (champions errants).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct EncounterRules {
    /// Premiere rencontre (s de partie).
    pub first: f32,
    /// Delai entre deux rencontres (s, minimum et maximum), compte apres la fin de la precedente.
    pub every: (f32, f32),
    /// Temps pour l'atteindre avant qu'il ne reparte (s).
    pub life: f32,
    /// Distance d'apparition au-dela du bord de l'ecran (pixels, minimum et maximum) : toujours hors champ.
    pub dist: (f32, f32),
    /// Le champion attend sur place tant que le joueur est plus loin que cette distance.
    pub engage: f32,
    /// Vie (multiplie la vie d'elite) et degats de contact (multiplicateur d'elite).
    pub hp: f32,
    pub damage: f32,
    /// Pas de rencontre dans les dernieres secondes du stage.
    pub quiet_end: f32,
}

impl Default for EncounterRules {
    fn default() -> Self {
        EncounterRules {
            first: 75.0,
            every: (100.0, 150.0),
            life: 70.0,
            dist: (60.0, 180.0),
            engage: 170.0,
            hp: 3.0,
            damage: 1.3,
            quiet_end: 40.0,
        }
    }
}

fn bomb_every_default() -> (f32, f32) {
    (70.0, 110.0)
}

fn luck_crit_default() -> f32 {
    0.3
}

/// Objet laisse par un element de decor casse.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum DropDef {
    Nothing,
    RupeeGreen,
    RupeeBlue,
    RupeeRed,
    Heart,
    /// Filet : attire tous les rubis.
    Magnet,
    /// Fee : soigne entierement.
    Fairy,
    /// Medaillon : gele les ennemis a l'ecran.
    Medallion,
}

/// Tout le contenu charge, indexe pour la simulation.
#[derive(Clone, Debug)]
pub struct Content {
    pub rules: RulesDef,
    pub characters: BTreeMap<String, CharacterDef>,
    pub weapons: Vec<WeaponDef>,
    pub weapon_ids: Vec<String>,
    pub passives: Vec<PassiveDef>,
    pub passive_ids: Vec<String>,
    pub enemies: Vec<EnemyDef>,
    pub enemy_ids: Vec<String>,
    pub maps: BTreeMap<String, MapDef>,
}

impl Content {
    pub fn enemy_index(&self, id: &str) -> Option<u16> {
        self.enemy_ids.iter().position(|e| e == id).map(|i| i as u16)
    }
    pub fn weapon_index(&self, id: &str) -> Option<u16> {
        self.weapon_ids.iter().position(|e| e == id).map(|i| i as u16)
    }
    pub fn passive_index(&self, id: &str) -> Option<u16> {
        self.passive_ids.iter().position(|e| e == id).map(|i| i as u16)
    }
}
