//! Le jeu : relie la simulation, le rendu, l'audio, les menus et le juice.
use crate::chars;
use crate::content::{load_bindings, load_content, AudioConfig};
use crate::director::{Director, Moment};
use crate::fairy_tip;
use crate::gallery;
use crate::hub;
use crate::hud::{Hud, GOLD};
use crate::juice::{Juice, NumKind};
use crate::lang::{Lang, LANGS};
use crate::menus::{self, Nav, NavIn, Opt, OptAction, OptValues};
use crate::meta::Meta;
use crate::online::{self, Online, Status as NetStatus, Submission};
use crate::players;
use crate::profile::{self as profiles, Profile, ScoreRecord};
use crate::scores;
use crate::settings::Settings;
use crate::reward::{self, Reward};
use crate::{clipboard, controls, records, shop, storage};
use crate::stages;
use crate::title::{self, Links, TitleItem, TitleVis};
use crate::update::{self, Status as UpdateStatus, UpdateMode, Updater};
use crate::visuals::{Orient, Vis};
use crate::weather::{self, WeatherVis};
use engine::app::{Ctx, Frame, Game as GameTrait};
use engine::audio::AudioFilter;
use engine::draw::{rgb, with_alpha, Color, Params, WHITE};
use engine::gfx::{Aspect, Blend, DrawList, Space, VideoFilter};
use engine::input::{Bindings, Device, KeyCode};
use engine::Vec2;
use sim::arsenal::ProjKind;
use sim::defs::{Content, Pattern, WeaponKind};
use sim::horde::{FACE_DOWN, ST_WINDUP};
use sim::pickups::PickupKind;
use sim::rng::Rng;
use sim::upgrades::{ChestReward, Offer};
use sim::{Event, PlayerInput, World, DT};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

// Outil de developpement (--uicheck) : a part, il a acces aux champs prives du jeu.
mod coop_play;
mod npcs;
mod uicheck_screens;

#[derive(Clone, Debug)]
pub struct Options {
    /// Test de charge : nombre d'ennemis maintenus en permanence.
    pub bench: Option<u32>,
    /// Deplacement et choix automatiques (captures, essais).
    pub demo: bool,
    pub seed: u64,
    pub map: String,
    pub character: String,
    /// Essais : couleur du personnage (indice dans `skins`), sans toucher au profil.
    pub skin: Option<usize>,
    /// Commencer la partie a cet instant (s) : essais de fin de partie, du boss.
    pub start_time: f32,
    /// Armes et objets donnes au depart (essais) : identifiants de `weapons.ron` / `passives.ron`.
    pub give: Vec<String>,
    /// Ouvrir un ecran des le depart (captures) : title, about, pause, options, gameover.
    pub menu: Option<String>,
    /// Langue imposee pour cette session (non enregistree).
    pub lang: Option<String>,
    /// Lancer directement une partie (sans ecran titre).
    pub play: bool,
    /// Verifier les placements de l'interface puis quitter.
    pub uicheck: bool,
    /// Mises a jour lues dans un dossier local (essais) au lieu de GitHub.
    pub update_source: Option<std::path::PathBuf>,
    /// Filtre video impose pour cette session (essais, non enregistre).
    pub video: Option<VideoFilter>,
    /// Session pilotee par un script (`--seconds`, captures) : la fermeture n'enregistre pas la partie.
    pub scripted: bool,
}

/// Niveaux de zoom en partie : la vue la plus large (celle d'avant la 0.8) puis deux plus proches.
pub const ZOOMS: [f32; 3] = [1.0, 1.25, 1.5];

/// Points de reprise de la partie en cours (profil enregistre comme si elle etait abandonnee) la ou
/// le jeu peut etre ferme sans prevenir : navigateur, Android.
const RUN_CHECKPOINTS: bool = cfg!(any(target_arch = "wasm32", target_os = "android"));

enum State {
    /// Ecran titre (hors partie).
    Title { sel: usize },
    /// Ecran « A propos » ; `toast` = confirmation d'ouverture d'un lien.
    About { sel: usize, toast: f32 },
    Playing,
    Paused { sel: usize },
    /// Options, ouvertes depuis l'ecran titre ou depuis la pause.
    Options { sel: usize, from_title: bool },
    LevelUp { offers: Vec<Offer>, sel: usize, t: f32 },
    Chest { reward: ChestReward, big: bool, t: f32 },
    /// Fin de partie : personnages et armes debloques, montres l'un apres l'autre, puis `next`.
    Unlocks { items: Vec<menus::Unlock>, t: f32, next: menus::AfterUnlock },
    /// Conseils de la fee sur l'ecran titre (page, temps de la page, question « passer ? »).
    Fairy { page: usize, t: f32, ask: Option<usize> },
    /// Plusieurs armes pretes pour le meme objet partenaire : le joueur choisit celle qui evolue.
    EvoChoice { options: Vec<(usize, u16)>, big: bool, sel: usize, t: f32 },
    /// Champion errant vaincu : recompense au choix.
    Reward { offers: Vec<Reward>, sel: usize, t: f32 },
    /// Diseuse de bonne aventure (0.6.3, apres le boss final) : jeux de hasard, partie en pause.
    Fortune { phase: crate::fortune::Phase, sel: usize, t: f32 },
    /// Sorciere (0.6.3) : potions de `Game::witch_offer` a acheter, ou rien.
    Witch { sel: usize, t: f32 },
    Dying(f32),
    Dead { t: f32, sel: usize },
    /// Choix du personnage, puis du stage (sur le decor du titre).
    /// `page` : univers (chars::GROUPS) ; `sel` : rang dans la page (n : Retour, n + 1 : univers).
    CharSelect { page: usize, sel: usize },
    /// Achat d'une couleur depuis le choix du personnage : question « Tu la prends ? » (0 = Non).
    SkinBuy { page: usize, sel: usize, skin: usize, ans: usize },
    StageSelect { sel: usize },
    /// Porte franchie : stage gagne.
    Victory { t: f32, sel: usize },
    /// Boutique (Kakariko, Grande Fee, reliques, defis).
    Shop { tab: usize, sel: usize },
    /// Preparatifs (0.8), entre le choix du stage et la partie : ligne 0 = « Partir ! », puis les
    /// onglets Reliques et Defis (`shop::PREP`).
    Prep { tab: usize, sel: usize },
    /// Profil (bestiaire, armurerie, statistiques, sauvegarde) ; `armed` : import a confirmer.
    Records { tab: usize, sel: usize, armed: bool },
    /// Commandes (depuis les options) ; `wait` : action en attente d'une touche et temps restant.
    Controls { tab: usize, sel: usize, wait: Option<(usize, f32)>, from_title: bool },
    /// Joueurs (5 emplacements) ; `armed` : suppression du joueur actif a confirmer.
    Players { sel: usize, armed: bool },
    /// Saisie d'un nom (nouveau joueur, renommage, pseudo du classement en ligne).
    NameEntry { purpose: NamePurpose, text: String, sel: usize },
    /// Choix du portrait ; `setup` : creation du profil au premier lancement (rien d'enregistre avant).
    Avatars { sel: usize, setup: bool },
    /// Galerie (salle dans `Game::gallery`) ; `wing` : ligne de l'onglet au retour ; `from_title` : ouverte
    /// depuis l'ecran titre (retour au titre).
    Gallery { wing: usize, from_title: bool },
    /// Coop en ligne : creer ou rejoindre une salle.
    Coop { sel: usize },
    /// Salon de la coop (salle dans `Game::coop`).
    Lobby { sel: usize },
    /// Classements : onglet, ligne, stage choisi, version choisie (vide : la plus recente).
    /// `players` : 1 (solo) ou 2 a 4 (classement de la coop).
    Scores { tab: usize, sel: usize, stage: usize, version: String, players: usize },
}

/// Ce que valide la saisie d'un nom.
enum NamePurpose {
    /// Nouveau joueur dans cet emplacement.
    Create(usize),
    Rename,
    /// Premier lancement : nom du joueur, puis son portrait (obligatoires).
    Setup,
    /// Publication d'une partie ; retour a la ligne `sel` du stage `stage`.
    Pseudo { rec: ScoreRecord, stage: usize, sel: usize },
    /// Code d'une salle de coop a rejoindre.
    Room,
}

impl State {
    /// Etats ou le jeu est fige (menus).
    fn frozen(&self) -> bool {
        !matches!(self, State::Playing | State::Dying(_))
    }

    /// Ecrans hors partie (le decor du titre remplace le monde).
    fn in_title(&self) -> bool {
        matches!(
            self,
            State::Title { .. }
                | State::About { .. }
                | State::CharSelect { .. }
                | State::SkinBuy { .. }
                | State::StageSelect { .. }
                | State::Shop { .. }
                | State::Prep { .. }
                | State::Records { .. }
                | State::Players { .. }
                | State::NameEntry { .. }
                | State::Avatars { .. }
                | State::Gallery { .. }
                | State::Coop { .. }
                | State::Lobby { .. }
                | State::Scores { .. }
                | State::Controls { from_title: true, .. }
                | State::Options { from_title: true, .. }
                | State::Fairy { .. }
        )
    }
}

/// Coop : regles de l'hote pour toute la partie (difficulte, multiplicateurs de la banque et du score,
/// poids de vie des ennemis).
#[derive(Clone, Copy)]
pub(super) struct CoopRules {
    pub difficulty: usize,
    pub bank: f32,
    pub score: f32,
    pub kw: f32,
}

pub struct Game {
    opts: Options,
    content: Content,
    audio_cfg: AudioConfig,
    world: World,
    vis: Vis,
    juice: Juice,
    hud: Hud,
    director: Director,
    state: State,
    settings: Settings,
    /// Touches d'origine (`data/input.ron`), base de l'ecran Commandes.
    base_bindings: Bindings,
    bindings: Bindings,
    lang: Lang,
    lang_names: Vec<String>,
    nav: Nav,
    cam: Option<Vec2>,
    frame_dt: f32,
    debug: bool,
    rupee_chain: u32,
    rupee_chain_t: f32,
    low_hp_t: f32,
    ghost_t: f32,
    /// Petits nuages de poussiere sous les pas.
    dust_t: f32,
    rng: Rng,
    sort: Vec<(u32, u32)>,
    bench_angle: f32,
    report_t: f32,
    events: Vec<Event>,
    /// Coups des armes de zone pendant le pas (pour les « gros impacts »).
    tick_area_hits: u32,
    mass_fx_cd: f32,
    /// Ignore les appuis (dash, bascule) juste apres la fermeture d'un menu.
    suppress: bool,
    /// Commandes tactiles en partie.
    touch: crate::touch::Touch,
    /// Tout est au maximum : bonus de niveau choisi une fois (Fee ou Rubis), repris ensuite sans ecran ;
    /// se change dans le menu Pause. Remis a zero a chaque partie.
    fallback: Option<Offer>,
    /// Couleur regardee dans le choix du personnage (personnage, indice) : peut ne pas etre achetee.
    skin_browse: Option<(String, usize)>,
    /// Zoom du monde (glisse vers `ZOOMS[settings.zoom]`), puis celui affiche : arrondi pour que les
    /// tuiles de 16 pixels tombent sur des pixels entiers de l'ecran (pas de jointures pendant le glissement).
    zoom: f32,
    view_zoom: f32,
    /// Reste de molette pour le zoom (pave tactile : fractions de cran).
    zoom_wheel: f32,
    boss_music: bool,
    title_vis: TitleVis,
    links: Links,
    /// Temps des animations de l'ecran titre (s).
    menu_t: f32,
    updater: Updater,
    /// Telechargement automatique deja lance.
    update_started: bool,
    /// Dernier eclair de l'ecran titre dont le tonnerre a ete joue.
    thunder: Option<u32>,
    weather_vis: WeatherVis,
    /// Decor visible (rempli a chaque image).
    prop_buf: Vec<sim::props::Prop>,
    /// Progression entre les parties.
    profile: Profile,
    /// Stages proposes, dans l'ordre.
    stage_ids: Vec<String>,
    /// Partie en cours pas encore comptee dans le profil.
    run_open: bool,
    /// Ennemis vaincus par type pendant la partie (bestiaire).
    run_kills: Vec<u32>,
    run_bosses: u32,
    run_elites: u32,
    run_chests: u32,
    /// Bilan de la derniere victoire : rubis mis en banque, stage debloque.
    last_banked: u64,
    last_unlocked: Option<String>,
    /// Reliques gagnees par la derniere victoire (stage et rencontres).
    last_relics: Vec<String>,
    /// Reliques de rencontre choisies pendant la partie (gardees si elle est gagnee).
    run_relics: Vec<String>,
    /// Personnages debloques par la derniere partie.
    last_chars: Vec<String>,
    /// Armes debloquees pendant la partie en cours (mini-boss vaincus pour la premiere fois).
    run_new_weapons: Vec<u16>,
    /// 0.6.3 : potions de la sorciere en visite, jeux deja faits chez la diseuse (coffres, quitte ou
    /// double), multiplicateur de banque du quitte ou double, PNJ touche (menu a ouvrir) et potion
    /// bue (couleur, instant en temps reel).
    witch_offer: Vec<crate::witch::Potion>,
    fortune_played: [bool; 2],
    run_bank_mult: f32,
    npc_met: Option<sim::npc::NpcKind>,
    drink: Option<(usize, f64)>,
    /// Navigateur : prochain enregistrement de la partie en cours (temps reel) et focus precedent.
    checkpoint_at: f64,
    was_focused: bool,
    /// Deblocages a montrer apres la partie (rempli a la fin d'une partie).
    pending_unlocks: Vec<menus::Unlock>,
    meta: Meta,
    /// Message de la boutique / du profil et son temps restant (s).
    hub_toast: (String, f32),
    /// Bestiaire : fiche ouverte (ennemi, instant d'ouverture en temps reel), et instant du dernier
    /// achat de figurine (eclat dore).
    beast: Option<(usize, f64)>,
    beast_bought: f64,
    /// Demi-taille visible des sprites de tirs ennemis (zone de contact, voir `World::shot_hit`).
    shot_hit: Vec<f32>,
    /// Classement en ligne (requetes en arriere-plan).
    online: Online,
    /// Score de la derniere partie et record du stage battu.
    last_score: Option<(u64, bool)>,
    /// Les 5 emplacements de joueurs (relus a l'ouverture des ecrans Joueurs et Classements).
    slots: Vec<Option<Profile>>,
    /// Partie en cours de publication (stage, partie).
    publishing: Option<(String, ScoreRecord)>,
    /// Juke-box : derniere musique ecoutee en extrait (ligne, debut en temps reel) ; un second appui
    /// sur la meme ligne l'achete. `jukebox_playing` : l'extrait passe encore.
    jukebox_try: Option<(usize, f64)>,
    jukebox_playing: bool,
    /// Collection : fiche ouverte (onglet armes / fusions / objets, element, instant d'ouverture).
    card: Option<(usize, usize, f64)>,
    /// Salle de la galerie (ouverte depuis la Collection).
    gallery: Option<gallery::Room>,
    /// Coop en ligne : salle rejointe (salon ou partie).
    coop: Option<crate::coop::Session>,
    /// Personnage et preparatifs choisis dans le salon.
    coop_ready: bool,
    /// Partie de coop : difficulte de l'hote et multiplicateur de banque (au lieu de ceux du joueur).
    coop_rules: Option<CoopRules>,
    /// Relances utilisees sur l'ecran de niveau en cours (envoyees avec le choix).
    coop_rerolls: u32,
    /// Allies : visuels (par heros) et noms.
    others_vis: Vec<crate::visuals::CharVis>,
    coop_names: Vec<String>,
    /// Coop : personnages des heros, separes par des virgules (score de l'equipe).
    coop_chars: String,
    /// Pendant le dessin, `world.projectiles` contient ceux du joueur local puis ceux de chaque allie : fin
    /// de chaque groupe (joueur local, puis allies dans l'ordre de `world.others`).
    proj_ends: Vec<usize>,
    /// Choix du personnage : premiere ligne visible de la grille ; personnage valide, choix de sa couleur.
    char_top: usize,
    char_lock: bool,
}

/// Musique de la galerie (fournie pour elle).
const GALLERY_MUSIC: &str = "great_fairys_fountain";

/// Position des entrees fixes du menu titre (retour depuis un sous-ecran).
const TITLE_COOP: usize = 1;
const TITLE_PLAYERS: usize = 2;
const TITLE_SHOP: usize = 3;
const TITLE_COLLECTION: usize = 4;
const TITLE_GALLERY: usize = 5;
const TITLE_EXPLOITS: usize = 6;
const TITLE_SCORES: usize = 7;
const TITLE_OPTIONS: usize = 8;
const TITLE_ABOUT: usize = 9;
/// Entree de mise a jour (apres les 10 entrees fixes).
const TITLE_UPDATE: usize = 10;
/// Extrait du juke-box : environ 5 s entendues (depart differe et fondu d'entree compris), puis fondu
/// de sortie vers la musique du menu.
const JUKEBOX_PREVIEW: f64 = 5.5;
/// Fichier de repli de l'export du profil (presse-papiers indisponible).
const CODE_FILE: &str = "profile_code.txt";

/// Marque des elements de decor dans le tri par profondeur.
const PROP_BIT: u32 = 0x8000_0000;
const DUST: [f32; 4] = [0.86, 0.8, 0.58, 1.0];
const FIRE: [f32; 4] = [1.0, 0.6, 0.2, 1.0];
const ICE: [f32; 4] = [0.7, 0.88, 1.0, 1.0];
/// Couleur des champions errants (rencontres).
const CHAMPION: [f32; 4] = [0.85, 0.5, 1.0, 1.0];
/// Couleur du voleur de rubis (Boo).
const THIEF: [f32; 4] = [0.47, 1.0, 0.63, 1.0];

impl Game {
    pub fn new(ctx: &mut Ctx, opts: Options) -> Game {
        let content = load_content(&ctx.root);
        let audio_cfg = AudioConfig::load(&ctx.root);
        for (name, r) in &audio_cfg.sfx {
            ctx.audio.set_rule(
                name,
                engine::audio::SfxRule {
                    max_voices: r.max_voices,
                    min_interval: r.min_interval,
                    volume: r.volume,
                    pitch_jitter: r.pitch_jitter,
                },
            );
        }
        let settings = Settings::load(&ctx.root);
        let updater = Updater::new(&ctx.root, opts.update_source.clone());
        settings.apply_audio(&mut ctx.audio);
        let base_bindings = load_bindings(&ctx.root);
        let bindings = settings.bindings(&base_bindings);
        let lang = Lang::load(&ctx.root, opts.lang.as_deref().unwrap_or(&settings.lang));
        let lang_names = LANGS.iter().map(|c| Lang::name_of(&ctx.root, c)).collect();
        let ch = content.characters.get(&opts.character).unwrap_or_else(|| panic!("personnage {}", opts.character));
        let map = content.maps.get(&opts.map).unwrap_or_else(|| panic!("carte {}", opts.map));
        let vis = Vis::new(&ctx.atlas, &content, ch, map);
        let world = Self::make_world(&content, &opts, ctx.view);
        let mut g = Game {
            debug: opts.bench.is_some(),
            opts,
            content,
            audio_cfg,
            world,
            vis,
            juice: Juice::new(),
            hud: Hud::new(),
            director: Director::new(),
            state: State::Playing,
            settings,
            base_bindings,
            bindings,
            lang,
            lang_names,
            nav: Nav::new(),
            cam: None,
            frame_dt: 0.0,
            rupee_chain: 0,
            rupee_chain_t: 0.0,
            low_hp_t: 0.0,
            ghost_t: 0.0,
            dust_t: 0.0,
            rng: Rng::new(99),
            sort: Vec::with_capacity(8192),
            bench_angle: 0.0,
            report_t: 0.0,
            events: Vec::with_capacity(1024),
            tick_area_hits: 0,
            mass_fx_cd: 0.0,
            suppress: false,
            touch: crate::touch::Touch::default(),
            zoom: 1.0,
            skin_browse: None,
            fallback: None,
            view_zoom: 1.0,
            zoom_wheel: 0.0,
            boss_music: false,
            title_vis: TitleVis::new(ctx),
            weather_vis: WeatherVis::new(&ctx.atlas),
            prop_buf: Vec::new(),
            links: Links::load(&ctx.root),
            menu_t: 0.0,
            updater,
            update_started: false,
            thunder: None,
            profile: Profile::load(&ctx.root),
            stage_ids: Vec::new(),
            run_open: false,
            run_kills: Vec::new(),
            run_bosses: 0,
            run_elites: 0,
            run_chests: 0,
            last_banked: 0,
            last_unlocked: None,
            last_relics: Vec::new(),
            run_relics: Vec::new(),
            last_chars: Vec::new(),
            run_new_weapons: Vec::new(),
            witch_offer: Vec::new(),
            fortune_played: [false; 2],
            run_bank_mult: 1.0,
            npc_met: None,
            drink: None,
            checkpoint_at: 0.0,
            was_focused: true,
            pending_unlocks: Vec::new(),
            meta: Meta::load(&ctx.root),
            hub_toast: (String::new(), 0.0),
            beast: None,
            beast_bought: f64::MIN,
            shot_hit: Vec::new(),
            online: Online::default(),
            last_score: None,
            slots: Vec::new(),
            publishing: None,
            jukebox_try: None,
            jukebox_playing: false,
            card: None,
            gallery: None,
            coop: None,
            coop_ready: false,
            coop_rules: None,
            coop_rerolls: 0,
            others_vis: Vec::new(),
            coop_names: Vec::new(),
            coop_chars: String::new(),
            proj_ends: Vec::new(),
            char_top: 0,
            char_lock: false,
        };
        g.shot_hit = g
            .vis
            .shots
            .iter()
            .map(|&(a, _)| {
                let f = ctx.atlas.frame(a, 0.0);
                0.5 * f.w.min(f.h) as f32
            })
            .collect();
        g.world.shot_hit = g.shot_hit.clone();
        g.stage_ids = stages::stage_ids(&g.content);
        g.meta.complete_wardrobe(&g.content);
        g.slots = profiles::all_slots(&ctx.root);
        g.run_kills = vec![0; g.content.enemies.len()];
        if let Some(v) = g.opts.video {
            g.settings.video = v;
        }
        // essais (demo, charge, depart avance) : directement en partie, sinon ecran titre
        // couleurs choisies (profil, ou --skin pour les essais) : sprite du personnage
        let mut skins = g.profile.clone();
        if let Some(k) = g.opts.skin {
            skins.skins.insert(g.opts.character.clone(), k);
            // essai d'une couleur : portee meme sans l'avoir achetee (profil copie, pas enregistre)
            if let Some(s) = g.content.characters.get(&g.opts.character).and_then(|d| d.skins.get(k)) {
                skins.wardrobe.insert(s.0.clone());
            }
        }
        chars::apply_skins(&mut g.content, &skins, &g.meta);
        let map = g.opts.map.clone();
        g.set_map(ctx, &map);
        g.hud.relics = g.relic_icons(&ctx.atlas);
        if g.opts.give.iter().any(|i| i == "relics") {
            // captures : trois reliques dans le HUD, sans toucher au profil
            g.hud.relics = g.meta.relics.iter().take(3).filter_map(|r| ctx.atlas.find(&r.icon, "0")).collect();
        }
        let o = &g.opts;
        let direct = o.play || o.demo || o.bench.is_some() || o.start_time > 0.0 || !o.give.is_empty();
        // premier lancement (aucun joueur enregistre) : nom et portrait avant tout le reste
        let first_run = !direct && g.opts.menu.is_none() && !g.opts.uicheck && !g.opts.scripted && profiles::all_slots(&ctx.root).iter().all(Option::is_none);
        g.state = match g.opts.menu.as_deref() {
            None if first_run => State::NameEntry { purpose: NamePurpose::Setup, text: String::new(), sel: 0 },
            Some("setup") => State::NameEntry { purpose: NamePurpose::Setup, text: String::new(), sel: 0 },
            Some("avatars") => State::Avatars { sel: g.profile.avatar, setup: false },
            // essais avec --demo : --menu coop_auto cree une salle, coop_auto_<CODE> la rejoint (pret d'office)
            Some(m) if m.starts_with("coop_auto") => {
                let room = m.strip_prefix("coop_auto_").unwrap_or("new").to_owned();
                g.coop_open(&room);
                g.coop_ready = true;
                State::Lobby { sel: 0 }
            }
            // --menu coop_menu, coop_salon, coop_partie... : ecrans de la coop avec une salle factice
            Some(m) if m.starts_with("coop_") => {
                let m = m.to_owned();
                let st = match m.as_str() {
                    "coop_menu" => State::Coop { sel: 0 },
                    m if m.starts_with("coop_partie") => State::Playing,
                    _ => State::Lobby { sel: 1 },
                };
                let _ = g.coop_screen(ctx, &m);
                st
            }
            // --menu gallery (aile des armes), gallery_<aile>, ou gallery_trophy_<n> (fiche du trophee n)
            Some(m) if m.starts_with("gallery") => {
                let trophy = m.strip_prefix("gallery_trophy_").and_then(|k| k.parse().ok());
                let wing = if trophy.is_some() { 0 } else { m.strip_prefix("gallery_").and_then(|k| k.parse().ok()).unwrap_or(1) };
                g.card = trophy.map(|k| (records::TAB_TROPHIES, k, ctx.real_time - 10.0));
                g.gallery = Some(gallery::Room::new(&g.content, &g.meta, &g.profile, wing));
                State::Gallery { wing, from_title: false }
            }
            Some("title") => State::Title { sel: 0 },
            // logo du champion (captures)
            Some("champion") => State::Title { sel: 0 },
            Some("about") => State::About { sel: 0, toast: 0.0 },
            Some("pause") => State::Paused { sel: 1 },
            Some("options") => State::Options { sel: 1, from_title: false },
            Some("title_options") => State::Options { sel: 1, from_title: true },
            Some("gameover") => State::Dead { t: 1.0, sel: 0 },
            Some("stages") => State::StageSelect { sel: 0 },
            Some("chars") => g.char_select(),
            // couleurs toutes achetees (profil en memoire seulement, pas enregistre) : bouton des couleurs
            Some("chars_skins") => {
                let all: Vec<String> = g.meta.wardrobe.iter().map(|w| w.sprite.clone()).collect();
                g.profile.wardrobe.extend(all);
                g.char_select()
            }
            Some("victory") => State::Victory { t: 3.0, sel: 0 },
            // fiche de la Collection (captures) : card_<arme ou objet>, beast_<ennemi>
            Some(m) if m.starts_with("card_") || m.starts_with("beast_") => {
                let (kind, id) = m.split_once('_').unwrap_or_default();
                let c = &g.content;
                let at = ctx.real_time - 10.0;
                let tab = if kind == "beast" {
                    g.beast = c.enemy_index(id).map(|k| (k as usize, at));
                    records::TAB_BESTIARY
                } else if let Some(k) = c.weapon_index(id) {
                    let tab = if c.weapons[k as usize].evolved { records::TAB_FUSIONS } else { records::TAB_WEAPONS };
                    g.card = Some((tab, k as usize, at));
                    tab
                } else {
                    g.card = c.passive_index(id).map(|k| (records::TAB_ITEMS, k as usize, at));
                    records::TAB_ITEMS
                };
                State::Records { tab, sel: 1, armed: false }
            }
            Some("levelup") => State::LevelUp { offers: g.world.roll_offers(), sel: 0, t: 1.0 },
            // fusion proposee (et fusion ultime) au milieu de choix ordinaires (captures)
            Some(m @ ("levelup_fusion" | "levelup_ultimate")) => {
                let c = g.world.content.clone();
                let into = if m == "levelup_ultimate" { "master_sword" } else { "combat_sword" };
                let mut offers: Vec<Offer> = c.weapon_index(into).map(Offer::Evolve).into_iter().collect();
                offers.extend(["quiver", "lamp"].iter().filter_map(|id| c.passive_index(id)).map(Offer::NewPassive));
                State::LevelUp { offers, sel: 1, t: 1.0 }
            }
            // objets dont un partenaire de fusion n'est pas encore debloque : « ??? » (captures)
            Some("levelup_locked") => {
                let c = g.world.content.clone();
                let offers = ["quiver", "lamp", "pegasus_boots"].iter().filter_map(|id| c.passive_index(id)).map(Offer::NewPassive).collect();
                State::LevelUp { offers, sel: 0, t: 1.0 }
            }
            // deblocages de fin de partie : 3 personnages et 2 armes (captures)
            Some("fairy") => State::Fairy { page: 0, t: 3.0, ask: None },
            Some("fairy_skip") => State::Fairy { page: 1, t: 3.0, ask: Some(0) },
            // tout le roster d'un coup : plusieurs pages (captures)
            Some("unlocks_all") => State::Unlocks { items: chars::roster(&g.content).into_iter().map(menus::Unlock::Char).collect(), t: 0.0, next: menus::AfterUnlock::Title },
            Some("unlocks") => {
                let mut items: Vec<menus::Unlock> = ["sheik", "mario", "vivi"].iter().map(|s| menus::Unlock::Char(s.to_string())).collect();
                items.extend(["slingshot", "ocarina"].iter().filter_map(|id| g.content.weapon_index(id)).map(menus::Unlock::Weapon));
                State::Unlocks { items, t: 0.0, next: menus::AfterUnlock::Title }
            }
            // fusion : arc et carquois au maximum, le coffre fait evoluer l'arc (captures)
            Some("evolution") => {
                let c = g.world.content.clone();
                if let (Some(bow), Some(q)) = (c.weapon_index("bow"), c.passive_index("quiver")) {
                    g.world.add_weapon(bow);
                    while g.world.weapon_level(bow).unwrap_or(99) < c.weapons[bow as usize].max_level() {
                        g.world.apply_offer(Offer::UpgradeWeapon(bow));
                    }
                    g.world.apply_offer(Offer::NewPassive(q));
                    while g.world.passive_level(q).unwrap_or(99) < c.passives[q as usize].max_level() {
                        g.world.apply_offer(Offer::UpgradePassive(q));
                    }
                }
                let reward = g.world.open_chest(false);
                State::Chest { reward, big: false, t: 0.0 }
            }
            // choix d'evolution : arc et lance-pierre partagent le carquois (captures)
            Some("evochoice") => {
                let pairs: Vec<(u16, u16)> = [("bow", "silver_bow"), ("slingshot", "hyper_slingshot")]
                    .iter()
                    .filter_map(|(a, b)| Some((g.world.content.weapon_index(a)?, g.world.content.weapon_index(b)?)))
                    .collect();
                let mut options = Vec::new();
                for (from, into) in pairs {
                    g.world.add_weapon(from);
                    options.push((g.world.weapons.len() - 1, into));
                }
                State::EvoChoice { options, big: false, sel: 0, t: 1.0 }
            }
            // PNJ de passage (captures) : diseuse, ses coffres, son quitte ou double, sorciere
            Some("fortune") => State::Fortune { phase: crate::fortune::Phase::Menu, sel: 0, t: 0.0 },
            Some("chests") => State::Fortune { phase: crate::fortune::Phase::Opened { stake: 500, mults: [1, 3, 0], pick: 1 }, sel: 1, t: 0.0 },
            Some("double") => State::Fortune { phase: crate::fortune::Phase::Double { win: true }, sel: 0, t: 0.0 },
            Some("witch") => {
                g.world.player.rupees = 260;
                g.witch_offer = crate::witch::roll(&mut g.rng, 1.0, 1.0, 200.0);
                g.world.add_npc(sim::npc::NpcKind::Witch, g.world.player.pos + Vec2::new(40.0, 0.0));
                State::Witch { sel: 0, t: 0.0 }
            }
            Some("npcs") => {
                g.world.add_npc(sim::npc::NpcKind::Witch, g.world.player.pos + Vec2::new(-50.0, -10.0));
                g.world.add_npc(sim::npc::NpcKind::Fortune, g.world.player.pos + Vec2::new(50.0, -10.0));
                State::Playing
            }
            // butin de champion avec au moins une relique (captures)
            Some("reward") => {
                let mut offers = reward::roll(&g.world, &g.meta, &g.profile, &[], &mut g.rng);
                if !offers.iter().any(|o| matches!(o, Reward::Relic(_))) && !g.meta.relics.is_empty() {
                    offers[0] = Reward::Relic(0);
                }
                State::Reward { offers, sel: 1, t: 1.0 }
            }
            Some("shop") => State::Shop { tab: 0, sel: 0 },
            // onglets de la boutique et fiche du bestiaire (captures)
            Some("prep") => State::Prep { tab: shop::TAB_RELICS, sel: 0 },
            Some(m) if m.starts_with("prep") => State::Prep { tab: shop::PREP.start + m[4..].parse::<usize>().unwrap_or(0).min(shop::PREP.len() - 1), sel: 1 },
            Some(m) if m.starts_with("shop") => State::Shop { tab: m[4..].parse::<usize>().unwrap_or(0).min(shop::SHOP.len() - 1), sel: 1 },
            Some(m) if m.starts_with("beast") => {
                let k = m[5..].parse().unwrap_or(0);
                g.beast = Some((k, 0.0));
                State::Records { tab: 0, sel: 0, armed: false }
            }
            Some("profile") => State::Records { tab: 0, sel: 0, armed: false },
            Some("trophies") => State::Records { tab: records::TAB_TROPHIES, sel: 0, armed: false },
            // fiche du bestiaire ouverte (captures) : le plus grand boss, pour tester le cadre
            Some("beast") => {
                g.beast = g.content.enemy_ids.iter().position(|e| e == "trinexx").map(|k| (k, 0.0));
                State::Records { tab: 0, sel: 0, armed: false }
            }
            Some("players") => State::Players { sel: 0, armed: false },
            Some("scores") => State::Scores { tab: 0, sel: 0, stage: 0, version: String::new(), players: 1 },
            Some("scores_world") => State::Scores { tab: scores::WORLD, sel: 0, stage: 0, version: String::new(), players: 1 },
            Some("controls") => State::Controls { tab: 0, sel: 0, wait: None, from_title: true },
            // annonce d'arme debloquee et case d'arme evoluee qui brille, en jeu (captures)
            Some("toast") => {
                if let Some(k) = g.content.weapon_index("slingshot") {
                    g.hud.toasts.push(crate::hud::Toast {
                        icon: g.vis.weapons[k as usize].icon,
                        title: g.lang.get("ui.weapon_unlocked").to_string(),
                        name: g.lang.get("weapon.slingshot.name").to_string(),
                        sub: g.lang.get("ui.weapon_unlocked_sub").to_string(),
                        t: 0.0,
                        life: 60.0,
                    });
                }
                g.hud.evo_flash = g.world.weapons.first().map(|w| (w.id, 60.0));
                State::Playing
            }
            // annonce en jeu (captures : centrage selon la taille de l'ecran)
            Some("banner") => {
                let s = g.lang.get("ui.thief_spawned").to_string();
                g.hud.banner(&s, THIEF, 60.0);
                State::Playing
            }
            // recueil, onglet des armes, sur le lance-pierre (indice du mini-boss s'il est verrouille)
            Some("armory") => State::Records { tab: records::TAB_WEAPONS, sel: 0, armed: false },
            _ if direct => State::Playing,
            _ => State::Title { sel: 0 },
        };
        if g.state.in_title() && !g.opts.uicheck && g.settings.updates != UpdateMode::Off {
            g.updater.check();
        }
        if g.state.in_title() {
            g.menu_t = if g.opts.menu.is_some() { 1.0 } else { 0.0 };
            let id = g.menu_music();
            g.play_music(ctx, &id, 0.5);
        } else {
            g.start_music(ctx);
            g.run_open = true;
            g.apply_mods();
        }
        g
    }

    /// Bonus de la boutique, reliques, difficulte et Sceaux pour la partie qui commence.
    fn apply_mods(&mut self) {
        if self.opts.demo || self.opts.bench.is_some() || self.opts.uicheck {
            return;
        }
        let m = self.meta.run_mods(&self.profile);
        self.world.set_mods(m);
        // armes a debloquer (mini-boss) : pas proposees tant que le joueur ne les a pas gagnees
        let p = &self.profile;
        self.world.locked_weapons = (0..self.content.weapons.len())
            .filter(|&k| self.content.weapons[k].unlock.is_some() && !p.weapons.contains(&self.content.weapon_ids[k]))
            .map(|k| k as u16)
            .collect();
    }

    /// Navigateur et Android : aucun evenement de fermeture n'arrive au jeu (onglet ferme, appli tuee
    /// en arriere-plan). Le profil est donc
    /// enregistre comme si la partie etait abandonnee maintenant (toutes les 10 s et quand la page
    /// perd le focus), sans toucher au profil en memoire : la vraie fin de partie l'ecrase ensuite.
    fn save_run_checkpoint(&mut self, ctx: &Ctx) {
        if !self.run_open || self.opts.scripted {
            return;
        }
        let keep = (self.profile.clone(), self.last_score, std::mem::take(&mut self.last_chars));
        self.commit_run(&ctx.root, false);
        (self.profile, self.last_score, self.last_chars) = keep;
        self.run_open = true;
    }

    /// Mini-boss vaincu : debloque les armes qui l'attendaient, pour cette partie et les suivantes.
    fn unlock_weapons_from(&mut self, ctx: &mut Ctx, kind: u16) {
        let eid = self.content.enemy_ids[kind as usize].clone();
        let ids: Vec<u16> = self
            .world
            .locked_weapons
            .iter()
            .copied()
            .filter(|&w| self.content.weapons[w as usize].unlock.as_deref() == Some(eid.as_str()))
            .collect();
        if ids.is_empty() {
            return;
        }
        for w in ids {
            self.world.locked_weapons.retain(|&x| x != w);
            let wid = self.content.weapon_ids[w as usize].clone();
            self.profile.weapons.insert(wid.clone());
            self.run_new_weapons.push(w);
            // Master Sword : fusion ultime (epee + bouclier), annoncee avec la fanfare de l'epee
            let ultimate = self.content.weapons[w as usize].evolved;
            let (title, sub) = if ultimate { ("ui.ultimate_unlocked", "ui.ultimate_unlocked_sub") } else { ("ui.weapon_unlocked", "ui.weapon_unlocked_sub") };
            if ultimate {
                self.sfx(ctx, "evo_merge");
                self.juice.flash([1.0, 0.8, 1.0, 0.5], 0.4);
            }
            self.hud.toasts.push(crate::hud::Toast {
                icon: self.vis.weapons[w as usize].icon,
                title: self.lang.get(title).to_string(),
                name: self.lang.get(&format!("weapon.{wid}.name")).to_string(),
                sub: self.lang.get(sub).to_string(),
                t: 0.0,
                life: 6.5,
            });
        }
        self.sfx(ctx, "weapon_unlock");
        let root = ctx.root.clone();
        self.profile.save(&root);
        // cet enregistrement n'a pas les gains de la partie en cours : point de reprise a l'image suivante
        self.checkpoint_at = 0.0;
    }

    /// Deblocages de la partie qui vient de finir : personnages puis armes.
    fn collect_unlocks(&mut self) {
        // 0.6.5 : des centaines de personnages, plusieurs seuils parfois franchis d'un coup : tous
        // montres, page apres page (menus::unlocks_per_page)
        let mut items: Vec<menus::Unlock> = self.last_chars.iter().cloned().map(menus::Unlock::Char).collect();
        items.extend(self.run_new_weapons.drain(..).map(menus::Unlock::Weapon));
        self.pending_unlocks = items;
    }

    /// Icones des reliques actives (equipees et possedees) pour le HUD de la partie.
    /// Reliques actives de la partie : celles equipees, puis celles gagnees en cours de partie (actives
    /// tout de suite). Essais et captures : seulement celles de la partie, pas celles du profil.
    fn relic_icons(&self, atlas: &engine::assets::Atlas) -> Vec<engine::assets::AnimId> {
        let profile = !(self.opts.demo || self.opts.bench.is_some());
        let p = &self.profile;
        let equipped = |id: &String| profile && p.equipped.contains(id) && p.relics.contains(id);
        let won = self.meta.relics.iter().filter(|r| !equipped(&r.id) && self.run_relics.contains(&r.id));
        self.meta
            .relics
            .iter()
            .filter(|r| equipped(&r.id))
            .chain(won)
            .filter_map(|r| atlas.find(&r.icon, "0"))
            .collect()
    }

    fn toast(&mut self, key: &str) {
        self.hub_toast = (self.lang.get(key).to_string(), 2.5);
    }

    /// Change de carte (sol et decor suivent).
    fn set_map(&mut self, ctx: &Ctx, id: &str) {
        if !self.content.maps.contains_key(id) {
            log::warn!("carte inconnue : {id}");
            return;
        }
        self.opts.map = id.to_string();
        let ch = self.content.characters.get(&self.opts.character).cloned();
        if let (Some(ch), Some(map)) = (ch, self.content.maps.get(id)) {
            self.vis = Vis::new(&ctx.atlas, &self.content, &ch, map);
        }
    }

    /// Ajoute la partie terminee (ou abandonnee) au profil. Renvoie les rubis mis en banque.
    fn commit_run(&mut self, root: &std::path::Path, won: bool) -> u64 {
        if !self.run_open {
            return 0;
        }
        self.run_open = false;
        self.last_score = None;
        if self.opts.demo || self.opts.bench.is_some() || self.opts.uicheck {
            return 0;
        }
        let before = self.profile.clone();
        let w = &self.world;
        let p = &mut self.profile;
        let rec = p.stages.entry(w.map_id.clone()).or_default();
        rec.runs += 1;
        let first_win = won && rec.wins == 0;
        if won {
            rec.wins += 1;
            if rec.best_time <= 0.0 || w.time < rec.best_time {
                rec.best_time = w.time;
            }
            rec.best_difficulty = rec.best_difficulty.max(self.coop_rules.map_or(p.difficulty, |r| r.difficulty) as i32);
        }
        // quitte ou double de la diseuse ; coop : multiplicateurs de l'hote
        let mult = self.coop_rules.map_or_else(|| self.meta.bank_mult(p), |r| r.bank);
        let banked = (bank_amount(w.player.rupees, won, mult) as f32 * self.run_bank_mult).round() as u64;
        p.rupees += banked;
        let t = &mut p.totals;
        t.runs += 1;
        t.wins += won as u32;
        t.deaths += (!w.player.alive) as u32;
        t.kills += w.player.kills as u64;
        t.bosses += self.run_bosses;
        t.elites += self.run_elites;
        t.chests += self.run_chests;
        t.time += w.time as f64;
        t.rupees += w.player.rupees as u64;
        t.banked += banked;
        t.best_level = t.best_level.max(w.player.level);
        for (k, &n) in self.run_kills.iter().enumerate() {
            if n > 0 {
                *p.bestiary.entry(self.content.enemy_ids[k].clone()).or_default() += n as u64;
            }
        }
        for s in &w.weapons {
            let id = &self.content.weapon_ids[s.id as usize];
            *p.armory.entry(id.clone()).or_default() += 1;
            if self.content.weapons[s.id as usize].evolved {
                p.evolutions.insert(id.clone());
            }
        }
        for s in &w.passives {
            *p.armory.entry(self.content.passive_ids[s.id as usize].clone()).or_default() += 1;
        }
        self.last_banked = banked;
        self.last_unlocked = None;
        self.last_relics.clear();
        if won {
            let mut ids: Vec<String> = self.meta.relic_of(&w.map_id).map(|r| r.id.clone()).into_iter().collect();
            ids.extend(self.run_relics.iter().cloned());
            for id in ids {
                if p.relics.insert(id.clone()) {
                    // emplacement libre : equipee d'office
                    if p.equipped.len() < self.meta.max_relics {
                        p.equipped.push(id.clone());
                    }
                    self.last_relics.push(id);
                }
            }
        }
        if first_win {
            if let Some(k) = self.stage_ids.iter().position(|s| *s == w.map_id) {
                self.last_unlocked = self.stage_ids.get(k + 1).cloned();
            }
        }
        // meilleurs scores du stage : ni les parties quittees (le joueur est encore en vie), ni
        // celles de moins de 30 s (retour 0.6.1) ; coop : score de l'equipe, a part
        let w = &self.world;
        if (!won && w.player.alive) || w.time < crate::profile::MIN_RUN_TIME {
            self.last_chars = chars::newly_unlocked(&self.content, &before, &self.profile);
        self.profile.new_chars.extend(self.last_chars.iter().cloned());
            self.profile.save(root);
            return banked;
        }
        let rec = match self.coop_rules {
            None => {
                let mult = self.meta.score_mult(&self.profile);
                let kw = Some(self.meta.kill_weight(&self.profile));
                ScoreRecord {
                    score: profiles::score(w.player.kills, w.time, self.run_bosses, won, mult, kw),
                    time: w.time,
                    kills: w.player.kills,
                    level: w.player.level,
                    bosses: self.run_bosses,
                    won,
                    character: self.opts.character.clone(),
                    difficulty: self.profile.difficulty,
                    mult,
                    kw,
                    version: title::VERSION.to_string(),
                    published: false,
                    team: Vec::new(),
                }
            }
            // coop : ennemis vaincus par toute l'equipe, regles de l'hote ; la meme partie dans chaque jeu
            Some(r) => {
                let heroes: Vec<&sim::world::Player> = {
                    let mut v: Vec<_> = std::iter::once(&w.player).chain(w.others.iter().map(|h| &h.player)).collect();
                    v.sort_by_key(|p| p.slot);
                    v
                };
                let kills = heroes.iter().map(|p| p.kills).sum();
                ScoreRecord {
                    score: profiles::score(kills, w.time, self.run_bosses, won, r.score, Some(r.kw)),
                    time: w.time,
                    kills,
                    level: heroes.iter().map(|p| p.level).max().unwrap_or(1),
                    bosses: self.run_bosses,
                    won,
                    character: self.coop_chars.clone(),
                    difficulty: r.difficulty,
                    mult: r.score,
                    kw: Some(r.kw),
                    version: title::VERSION.to_string(),
                    published: false,
                    team: self.coop_names.clone(),
                }
            }
        };
        let score = rec.score;
        let had = !self.profile.score_list(&w.map_id, rec.team.len().max(1)).is_empty();
        let place = self.profile.add_score(&w.map_id, rec);
        self.last_score = Some((score, had && place == Some(0)));
        self.last_chars = chars::newly_unlocked(&self.content, &before, &self.profile);
        self.profile.new_chars.extend(self.last_chars.iter().cloned());
        self.profile.save(root);
        banked
    }

    /// Retour a l'ecran titre (partie abandonnee).
    fn quit_to_title(&mut self, ctx: &mut Ctx) {
        let root = ctx.root.clone();
        let was_open = self.run_open;
        self.commit_run(&root, false);
        if was_open {
            self.collect_unlocks();
        }
        self.state = if self.pending_unlocks.is_empty() {
            State::Title { sel: 0 }
        } else {
            State::Unlocks { items: std::mem::take(&mut self.pending_unlocks), t: 0.0, next: menus::AfterUnlock::Title }
        };
        self.menu_t = 0.0;
        self.thunder = None;
        self.juice.clear();
        self.director.clear();
        ctx.audio.stop_music(0.3);
        let id = self.menu_music();
        self.play_music(ctx, &id, 0.8);
        ctx.audio.duck_music(1.0, 0.2);
    }

    /// Musique de l'ecran titre : celle choisie au juke-box, sinon celle par defaut.
    fn menu_music(&self) -> String {
        shop::title_music(&self.meta, &self.profile).map(str::to_string).unwrap_or_else(|| self.audio_cfg.menu_music.clone())
    }

    fn make_world(content: &Content, opts: &Options, view: Vec2) -> World {
        let mut w = World::new(content.clone(), &opts.character, &opts.map, opts.seed);
        w.view_half = view * 0.5;
        if opts.start_time > 0.0 {
            w.skip_to(opts.start_time);
        }
        for id in &opts.give {
            if let Some(k) = content.weapon_index(id) {
                match w.weapon_level(k) {
                    Some(_) => w.apply_offer(Offer::UpgradeWeapon(k)),
                    None => w.apply_offer(Offer::NewWeapon(k)),
                }
            } else if let Some(k) = content.passive_index(id) {
                match w.passive_level(k) {
                    Some(_) => w.apply_offer(Offer::UpgradePassive(k)),
                    None => w.apply_offer(Offer::NewPassive(k)),
                }
            } else if id == "chest" || id == "big_chest" {
                // coffre ouvert des le debut (essais des evolutions)
                w.pending_chests.push(id == "big_chest");
            } else if id == "thief" {
                // essais : le voleur (Boo) arrive tout de suite
                w.thief_t = 0.0;
            } else if id == "bobomb" {
                // essais : une Bob-omb tout pres (souffle de l'ecran)
                if let Some(k) = w.bomb_kind {
                    let at = w.player.pos + Vec2::new(70.0, 20.0);
                    w.spawn_enemy(k, at);
                }
            } else if id == "encounter" {
                // essais : un champion errant tout de suite
                w.encounter_t = 0.0;
            } else if id == "reward" {
                w.pending_rewards += 1;
            } else if id == "relics" {
                // captures : reliques du HUD (voir Game::new)
            } else if id == "drops" {
                // essais : les objets rares du decor au sol
                for (k, kind) in [PickupKind::Magnet, PickupKind::Fairy, PickupKind::Medallion].into_iter().enumerate() {
                    let at = w.player.pos + Vec2::new(-30.0 + k as f32 * 30.0, -40.0);
                    w.pickups.spawn(kind, 0, at, Vec2::ZERO, 0.0);
                }
            } else {
                log::warn!("--give : inconnu {id}");
            }
        }
        if let Some(n) = opts.bench {
            w.spawning = false;
            w.player.god = true;
            let kinds: Vec<u16> = ["soldier_green", "soldier_blue", "rat", "rope", "keese"]
                .iter()
                .filter_map(|k| content.enemy_index(k))
                .collect();
            for i in 0..n {
                let k = kinds[i as usize % kinds.len()];
                let p = w.player.pos + w.rng.dir() * w.rng.range(40.0, view.length() * 0.5);
                w.spawn_enemy(k, p);
            }
        }
        w
    }

    fn play_music(&mut self, ctx: &mut Ctx, id: &str, fade: f32) {
        if let Some(def) = self.audio_cfg.music_def(id) {
            let root = ctx.root.clone();
            ctx.audio.play_music(id, &def, &root, fade);
        } else {
            log::warn!("musique inconnue : {id}");
        }
    }

    fn start_music(&mut self, ctx: &mut Ctx) {
        let id = self.world.map.music.clone();
        self.play_music(ctx, &id, 1.0);
        ctx.audio.duck_music(1.0, 0.2);
        self.boss_music = false;
        // navigateur : les musiques des boss sont telechargees et decodees des maintenant
        for id in [self.world.map.boss_music.clone(), self.world.map.final_music.clone()] {
            if let Some(def) = self.audio_cfg.music_def(&id) {
                ctx.audio.prefetch(&def);
            }
        }
    }

    fn restart(&mut self, ctx: &mut Ctx) {
        let root = ctx.root.clone();
        self.commit_run(&root, false);
        // personnage joue : il n'est plus nouveau
        if self.profile.new_chars.remove(&self.opts.character) {
            self.profile.save(&root);
        }
        self.run_open = true;
        self.run_relics.clear();
        self.run_new_weapons.clear();
        self.witch_offer.clear();
        self.fortune_played = [false; 2];
        self.run_bank_mult = 1.0;
        self.npc_met = None;
        self.drink = None;
        self.pending_unlocks.clear();
        self.run_kills.iter_mut().for_each(|k| *k = 0);
        self.run_bosses = 0;
        self.run_elites = 0;
        self.run_chests = 0;
        self.thunder = None;
        ctx.audio.stop_ambience(0.5);
        self.world = Self::make_world(&self.content, &self.opts, ctx.view);
        self.fallback = None;
        // nouvelle partie : directement au zoom choisi
        self.zoom = ZOOMS[self.settings.zoom as usize % ZOOMS.len()];
        self.world.shot_hit = self.shot_hit.clone();
        self.apply_mods();
        self.juice.clear();
        self.hud = Hud::new();
        self.hud.relics = self.relic_icons(&ctx.atlas);
        self.director.clear();
        self.state = State::Playing;
        self.cam = None;
        self.rupee_chain = 0;
        self.suppress = true;
        ctx.audio.stop_music(0.2);
        self.start_music(ctx);
    }

    fn sfx(&mut self, ctx: &mut Ctx, event: &str) {
        self.sfx_p(ctx, event, 1.0, 1.0);
    }

    /// Son propre a un ennemi (« shot.agahnim »...), sinon le son commun de l'evenement.
    fn sfx_enemy(&mut self, ctx: &mut Ctx, event: &str, kind: u16) {
        let prefix = if event == "enemy_shot" { "shot" } else { event };
        let key = format!("{prefix}.{}", self.content.enemy_ids[kind as usize]);
        if self.audio_cfg.events.contains_key(&key) {
            self.sfx(ctx, &key);
        } else {
            self.sfx(ctx, event);
        }
    }

    fn sfx_p(&mut self, ctx: &mut Ctx, event: &str, gain: f32, pitch: f32) {
        let Some(list) = self.audio_cfg.events.get(event) else { return };
        if list.is_empty() {
            return;
        }
        let name = &list[self.rng.below(list.len() as u32) as usize];
        ctx.audio.play(name, gain, pitch);
    }

    /// Lit les commandes de jeu (pas de simulation).
    fn read_input(&mut self, ctx: &Ctx) -> PlayerInput {
        if self.opts.bench.is_some() {
            self.bench_angle += DT * 0.5;
            return PlayerInput {
                move_dir: Vec2::from_angle(self.bench_angle) * 0.8,
                aim: None,
                attack: false,
                dash: self.world.ticks.is_multiple_of(150),
                toggle_auto: false,
            };
        }
        if self.opts.demo {
            return self.demo_input();
        }
        let i = &ctx.input;
        let b = &self.bindings;
        let mut mv = Vec2::ZERO;
        if i.action(b, "up") {
            mv.y -= 1.0;
        }
        if i.action(b, "down") {
            mv.y += 1.0;
        }
        if i.action(b, "left") {
            mv.x -= 1.0;
        }
        if i.action(b, "right") {
            mv.x += 1.0;
        }
        let t = self.touch.read(i, ctx.scale as f32);
        let mv = (mv.normalize_or_zero() + i.stick(false) + t.mv).clamp_length_max(1.0);
        // visee tactile (doigt a droite) : l'epee frappe dans sa direction
        let attack = i.action(b, "attack") || t.aim.is_some();
        let dash = !self.suppress && (i.action_pressed(b, "dash") || t.dash);
        let toggle_auto = !self.suppress && (i.action_pressed(b, "toggle_auto") || t.toggle);
        let aim = match i.last_device {
            Device::Mouse => {
                let cam = self.cam.unwrap_or(self.world.player.pos);
                let off = (Vec2::new(ctx.view.x, ctx.view.y) * ctx.scale as f32 * 0.5).round();
                let m = cam + (i.mouse_px - off) / (ctx.scale as f32 * self.view_zoom);
                Some(m - (self.world.player.pos - Vec2::new(0.0, 8.0)))
            }
            Device::Gamepad => {
                let r = i.stick(true);
                (r.length_squared() > 0.05).then_some(r)
            }
            Device::Touch => t.aim,
            Device::Keyboard => None,
        };
        PlayerInput { move_dir: mv, aim, attack, dash, toggle_auto }
    }

    /// Pilote du mode demo : garde ses distances, ramasse les rubis et les coffres, esquive au dash.
    /// Sert aux captures et a juger l'equilibrage sans joueur.
    fn demo_input(&mut self) -> PlayerInput {
        let w = &self.world;
        let me = w.player.pos;
        let mut flee = Vec2::ZERO;
        let mut close = 0;
        let mut crowd = 0;
        let mut nearest: Option<(f32, Vec2)> = None;
        for &p in &w.horde.pos {
            let d = me - p;
            let l2 = d.length_squared();
            if l2 < 60.0 * 60.0 {
                let l = l2.sqrt().max(1.0);
                flee += d / l * (60.0 - l) / 60.0;
                crowd += 1;
                if l < 22.0 {
                    close += 1;
                }
            }
            if nearest.is_none_or(|n| l2 < n.0) {
                nearest = Some((l2, -d));
            }
        }
        // peu d'ennemis autour : va au contact (portee de l'epee), sinon recule
        if crowd <= 5 {
            if let Some((l2, d)) = nearest {
                let l = l2.sqrt();
                let want = if l > 24.0 { d / l } else { -d / l.max(1.0) * 0.5 + d.perp() / l.max(1.0) };
                flee = flee * 0.5 + want;
            }
        }
        for s in &w.shots {
            let d = me - s.pos;
            if d.length_squared() < 40.0 * 40.0 {
                flee += d.normalize_or_zero() * 1.5;
            }
        }
        // objet le plus proche (les coffres comptent double)
        let pk = &w.pickups;
        let mut target: Option<(f32, Vec2)> = None;
        for i in 0..pk.len() {
            let d = pk.pos[i] - me;
            let mut l = d.length();
            if pk.kind[i].is_chest() {
                l *= 0.5;
            }
            if l < 180.0 && target.is_none_or(|t| l < t.0) {
                target = Some((l, d));
            }
        }
        self.bench_angle += DT * 0.35;
        let wander = Vec2::from_angle(self.bench_angle) * 0.35;
        let seek = target.map(|t| t.1.normalize_or_zero() * 0.8).unwrap_or(wander);
        let mv = (flee * 1.6 + seek).clamp_length_max(1.0);
        PlayerInput { move_dir: mv, aim: None, attack: false, dash: close >= 3 && w.player.dash_cd <= 0.0, toggle_auto: false }
    }

    fn weapon_sfx(&mut self, ctx: &mut Ctx, weapon: u16) {
        if let Some(d) = self.content.weapons.get(weapon as usize) {
            let key = d.sfx.clone();
            if !key.is_empty() {
                self.sfx(ctx, &key);
            }
        }
    }

    fn on_events(&mut self, ctx: &mut Ctx) {
        let mut evs = std::mem::take(&mut self.events);
        evs.clear();
        evs.append(&mut self.world.events);
        self.tick_area_hits = 0;
        // souffle de Bob-omb : les morts en masse restent sobres, le souffle fait le spectacle
        let clearing = evs.iter().any(|e| matches!(e, Event::ScreenClear { .. }));
        for ev in evs.iter().copied() {
            match ev {
                Event::Swing { weapon, .. } => {
                    self.weapon_sfx(ctx, weapon);
                    if self.content.weapons[weapon as usize].beam.is_some() {
                        self.sfx(ctx, "beam");
                    }
                }
                Event::Fire { weapon, .. } => self.weapon_sfx(ctx, weapon),
                Event::Dash { pos, dir } => {
                    self.sfx(ctx, "dash");
                    self.juice.anim(self.vis.dust, pos - dir * 4.0, 0.3);
                    self.juice.debris(pos, -dir, 5, [0.85, 0.8, 0.6, 1.0], 50.0, false);
                    self.juice.add_trauma(0.08, 0.3);
                }
                Event::EnemyHit { pos, kind, damage, crit, dir, weapon } => {
                    let boss = self.content.enemies[kind as usize].boss;
                    // brulure et flammes au sol : discret, sans etincelle ni son
                    if weapon == u16::MAX || dir == Vec2::ZERO {
                        self.juice.number(pos - Vec2::new(0.0, 14.0), damage, NumKind::Dot);
                        continue;
                    }
                    // nuage de poudre : touches continues, discretes (petits chiffres, sans son)
                    if self.content.weapons[weapon as usize].kind == WeaponKind::Aura && !crit {
                        self.juice.number(pos - Vec2::new(0.0, 14.0), damage, NumKind::Dot);
                        continue;
                    }
                    if matches!(
                        self.content.weapons[weapon as usize].kind,
                        WeaponKind::Lob | WeaponKind::Slam | WeaponKind::Storm | WeaponKind::Pillars
                    ) {
                        self.tick_area_hits += 1;
                    }
                    let nk = if boss {
                        NumKind::Boss
                    } else if crit {
                        NumKind::Crit
                    } else {
                        NumKind::Normal
                    };
                    let top = if boss { 58.0 } else { 14.0 };
                    self.juice.number(pos - Vec2::new(0.0, top), damage, nk);
                    let rot = dir.y.atan2(dir.x) + FRAC_PI_4;
                    self.juice.anim_rot(self.vis.spark, pos - Vec2::new(0.0, 9.0) - dir * 3.0, 0.1, rot);
                    self.juice.debris(pos - Vec2::new(0.0, 6.0), dir, 1, [1.0, 1.0, 0.8, 1.0], 90.0, true);
                    if boss {
                        self.sfx(ctx, "boss_hit");
                    } else if crit {
                        self.sfx(ctx, "enemy_crit");
                        self.juice.add_trauma(0.1, 0.3);
                    } else {
                        self.sfx(ctx, "enemy_hit");
                    }
                }
                Event::EnemyKilled { pos, dir, elite, kind } => {
                    if let Some(n) = self.run_kills.get_mut(kind as usize) {
                        *n += 1;
                    }
                    self.run_elites += elite as u32;
                    // le corps est projete en tournoyant (dans le sens du coup)
                    let k = kind as usize;
                    if !self.content.enemies[k].boss {
                        let (anim, flip) = self.vis.enemies[k].pick(FACE_DOWN, 0.0, false);
                        let f = *ctx.atlas.frame(anim, 0.0);
                        let d = if dir == Vec2::ZERO { self.rng.dir() } else { dir };
                        self.juice.corpse(f, pos, d, flip, self.vis.enemy_scale[k], if clearing { 1.6 } else { 1.0 });
                    }
                    self.juice.debris(pos - Vec2::new(0.0, 5.0), dir, 3, DUST, 110.0, false);
                    if !clearing {
                        self.juice.anim(self.vis.poof, pos - Vec2::new(0.0, 7.0), 0.31);
                        self.sfx(ctx, "enemy_die");
                        self.juice.add_trauma(0.02, 0.2);
                    }
                    if elite {
                        self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, 40.0, 0.4, GOLD);
                        self.juice.debris(pos - Vec2::new(0.0, 8.0), Vec2::ZERO, 14, GOLD, 120.0, true);
                        self.juice.add_trauma(0.25, 0.5);
                    }
                    if self.director.kill(self.world.time) {
                        self.mass_kill(ctx);
                    }
                }
                // coop : un allie touche, seulement les eclats
                Event::PlayerHurt { pos, slot, .. } if slot != self.world.local => {
                    self.juice.debris(pos - Vec2::new(0.0, 8.0), Vec2::ZERO, 6, rgb(230, 40, 40), 80.0, false);
                }
                Event::PlayerHurt { pos, .. } => {
                    self.sfx(ctx, "hurt");
                    self.juice.add_trauma(0.5, 0.75);
                    self.juice.hitstop(0.07);
                    self.juice.flash([0.9, 0.1, 0.1, 0.3], 0.25);
                    self.juice.debris(pos - Vec2::new(0.0, 8.0), Vec2::ZERO, 8, rgb(230, 40, 40), 100.0, false);
                    self.hud.heart_shake = 1.0;
                }
                // coop : a terre tant qu'un allie tient debout (un allie proche le releve)
                Event::PlayerDied { pos, slot } if self.world.coop() && (self.world.any_alive() || !self.run_open) => {
                    if self.run_open {
                        let me = slot == self.world.local;
                        let s = if me { self.lang.get("ui.coop.down").to_string() } else { self.lang.get("ui.coop.ally_down").replace("{s}", &self.hero_name(slot)) };
                        self.hud.banner(&s, rgb(255, 120, 110), 2.5);
                        self.sfx(ctx, if me { "die" } else { "hurt" });
                    }
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 2.0, 40.0, 0.5, WHITE);
                }
                Event::PlayerDied { pos, .. } => {
                    let root = ctx.root.clone();
                    self.commit_run(&root, false);
                    self.collect_unlocks();
                    self.sfx(ctx, "die");
                    self.state = State::Dying(0.0);
                    ctx.audio.stop_music(2.0);
                    self.juice.flash([1.0, 1.0, 1.0, 0.6], 0.5);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 2.0, 60.0, 0.6, WHITE);
                }
                Event::Pickup { pos, kind, value } => match kind {
                    PickupKind::Heart => {
                        self.sfx(ctx, "heart");
                        self.juice.ring(pos - Vec2::new(0.0, 6.0), 2.0, 16.0, 0.3, rgb(255, 120, 140));
                    }
                    PickupKind::Chest | PickupKind::BigChest => {}
                    PickupKind::Magnet => {
                        self.sfx(ctx, "net_pickup");
                        let s = self.lang.get("ui.pickup_net").to_string();
                        self.hud.banner(&s, rgb(150, 230, 150), 2.0);
                        self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, 120.0, 0.5, rgb(150, 230, 150));
                    }
                    PickupKind::Fairy => {
                        self.sfx(ctx, "fairy_pickup");
                        self.sfx(ctx, "fairy");
                        let s = self.lang.get("ui.pickup_fairy").to_string();
                        self.hud.banner(&s, rgb(255, 170, 230), 2.0);
                        self.juice.flash([1.0, 0.8, 0.95, 0.35], 0.4);
                        self.juice.ring(pos - Vec2::new(0.0, 8.0), 2.0, 50.0, 0.5, rgb(255, 170, 230));
                        self.juice.debris(pos - Vec2::new(0.0, 8.0), Vec2::ZERO, 16, rgb(255, 200, 240), 90.0, true);
                    }
                    PickupKind::Medallion => {}
                    _ => {
                        // les rubis ramasses en chaine montent la gamme
                        self.rupee_chain = (self.rupee_chain + 1).min(16);
                        self.rupee_chain_t = 0.7;
                        let pitch = 1.0 + 0.045 * (self.rupee_chain - 1) as f32;
                        self.sfx_p(ctx, "rupee", 1.0, pitch);
                        self.hud.rupee_pop = 1.0;
                        let c = match kind {
                            PickupKind::RupeeBlue => rgb(120, 160, 255),
                            PickupKind::RupeeRed => rgb(255, 110, 110),
                            _ => rgb(140, 255, 140),
                        };
                        self.juice.debris(pos - Vec2::new(0.0, 6.0), Vec2::ZERO, if value >= 5 { 6 } else { 2 }, c, 60.0, true);
                    }
                },
                // coop : le niveau d'un allie, seulement l'onde
                Event::LevelUp { pos, slot, .. } if slot != self.world.local => {
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, self.content.rules.levelup_wave_radius, 0.45, WHITE);
                }
                Event::LevelUp { pos, .. } => {
                    self.sfx(ctx, "level_up");
                    self.sfx(ctx, "level_wave");
                    let r = self.content.rules.levelup_wave_radius;
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, r, 0.45, WHITE);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 2.0, r * 0.6, 0.35, GOLD);
                    self.juice.flash([1.0, 1.0, 0.85, 0.2], 0.18);
                    self.juice.add_trauma(0.3, 0.5);
                }
                Event::AutoToggled { .. } => {
                    self.sfx(ctx, "toggle_auto");
                    self.hud.mode_banner = 1.5;
                }
                Event::Explosion { pos, radius, .. } => {
                    let sc = (radius / 20.0).clamp(1.0, 4.0).round();
                    self.juice.anim_ex(self.vis.explosion, pos - Vec2::new(0.0, 4.0), 0.36, 0.0, sc, WHITE, false);
                    self.juice.ring(pos - Vec2::new(0.0, 4.0), 4.0, radius, 0.3, FIRE);
                    self.juice.debris(pos - Vec2::new(0.0, 4.0), Vec2::ZERO, 8, FIRE, 130.0, true);
                    self.juice.add_trauma(0.12 + radius * 0.002, 0.5);
                    self.sfx(ctx, "explosion");
                }
                Event::Slam { pos, radius, weapon } => {
                    let def = &self.content.weapons[weapon as usize];
                    let fire = matches!(def.status, Some(sim::defs::StatusDef::Burn { .. }));
                    if def.kind == WeaponKind::Jump {
                        self.sfx(ctx, "land");
                    }
                    let n = (radius / 8.0) as u32;
                    if fire {
                        self.juice.ring(pos - Vec2::new(0.0, 2.0), 6.0, radius, 0.4, FIRE);
                        for k in 0..n {
                            let a = k as f32 / n as f32 * TAU;
                            let p = pos + Vec2::from_angle(a) * radius * 0.75;
                            self.juice.anim_ex(self.vis.flame, p, 0.4, 0.0, 1.0, WHITE, true);
                        }
                        self.juice.debris(pos, Vec2::ZERO, 12, FIRE, 160.0, true);
                        self.sfx(ctx, "explosion");
                    } else {
                        self.juice.ring(pos - Vec2::new(0.0, 2.0), 6.0, radius, 0.35, rgb(240, 220, 160));
                        for k in 0..n {
                            let a = k as f32 / n as f32 * TAU;
                            let p = pos + Vec2::from_angle(a) * radius * 0.7;
                            self.juice.anim(self.vis.dust, p, 0.25);
                        }
                        self.juice.debris(pos, Vec2::ZERO, 10, DUST, 140.0, false);
                    }
                    self.juice.add_trauma(if radius > 90.0 { 0.55 } else { 0.25 }, 0.7);
                    if radius > 90.0 {
                        self.juice.flash([1.0, 0.95, 0.8, 0.22], 0.2);
                    }
                }
                Event::Storm { pos, radius, .. } => {
                    self.juice.flash([0.7, 0.85, 1.0, 0.35], 0.3);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 8.0, radius, 0.45, ICE);
                    self.juice.add_trauma(0.35, 0.6);
                }
                Event::Pillar { pos, radius, .. } => {
                    let sc = (radius / 10.0).clamp(1.0, 3.0).round();
                    self.juice.anim_ex(self.vis.flame, pos, 0.4, 0.0, sc, WHITE, true);
                    self.juice.ring(pos - Vec2::new(0.0, 4.0), 3.0, radius, 0.25, FIRE);
                    self.juice.add_trauma(0.06, 0.4);
                    self.sfx(ctx, "explosion");
                }
                Event::Windup { pos, kind } => {
                    if self.content.enemies[kind as usize].boss {
                        self.juice.anim_ex(self.vis.spark, pos - Vec2::new(0.0, 66.0), 0.2, 0.0, 2.0, WHITE, true);
                    }
                    self.sfx_enemy(ctx, "windup", kind);
                }
                Event::EnemyShot { kind, .. } => self.sfx_enemy(ctx, "enemy_shot", kind),
                Event::ShotBlast { pos, radius } => {
                    let sc = (radius / 18.0).clamp(1.0, 3.0).round();
                    self.juice.anim_ex(self.vis.explosion, pos - Vec2::new(0.0, 4.0), 0.36, 0.0, sc, WHITE, false);
                    self.juice.ring(pos - Vec2::new(0.0, 4.0), 4.0, radius, 0.3, rgb(255, 90, 60));
                    self.juice.add_trauma(0.2, 0.5);
                    self.sfx(ctx, "shot_blast");
                }
                Event::Revived { pos, slot } => {
                    self.sfx(ctx, "fairy");
                    let s = match self.world.coop() {
                        true if slot == self.world.local => self.lang.get("ui.coop.revived_me").to_string(),
                        true => self.lang.get("ui.coop.revived").replace("{s}", &self.hero_name(slot)),
                        false => self.lang.get("ui.fairy_revive").to_string(),
                    };
                    self.hud.banner(&s, rgb(255, 170, 230), 2.5);
                    self.juice.flash([1.0, 0.8, 0.95, 0.6], 0.6);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, 90.0, 0.6, rgb(255, 170, 230));
                }
                Event::BossSlam { pos, radius } => {
                    self.juice.ring(pos - Vec2::new(0.0, 2.0), 6.0, radius, 0.4, rgb(230, 200, 150));
                    self.juice.ring(pos - Vec2::new(0.0, 2.0), 2.0, radius * 0.6, 0.3, rgb(255, 255, 220));
                    self.juice.add_trauma(0.45, 0.6);
                    self.sfx(ctx, "boss_slam");
                }
                Event::Teleport { from, to, .. } => {
                    self.juice.ring(from - Vec2::new(0.0, 12.0), 20.0, 2.0, 0.35, rgb(200, 140, 255));
                    self.juice.ring(to - Vec2::new(0.0, 12.0), 2.0, 26.0, 0.4, rgb(200, 140, 255));
                    self.sfx(ctx, "teleport");
                }
                Event::EliteSpawned { pos } => {
                    self.sfx(ctx, "elite");
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 2.0, 30.0, 0.5, GOLD);
                }
                Event::BossSpawned { pos, last, .. } => {
                    self.sfx(ctx, "boss_spawn");
                    let s = self.lang.get(if last { "ui.final_boss_banner" } else { "ui.boss" }).to_string();
                    self.hud.banner(&s, rgb(255, 110, 110), 2.8);
                    self.juice.add_trauma(0.6, 0.8);
                    self.juice.flash([0.4, 0.0, 0.0, 0.3], 0.5);
                    self.juice.ring(pos - Vec2::new(0.0, 16.0), 6.0, 80.0, 0.6, rgb(255, 80, 80));
                    let m = &self.world.map;
                    let id = if last && !m.final_music.is_empty() { m.final_music.clone() } else { m.boss_music.clone() };
                    if !id.is_empty() {
                        self.play_music(ctx, &id, 1.0);
                        self.boss_music = true;
                    }
                }
                Event::BossDied { pos, last, kind } => {
                    self.run_bosses += 1;
                    self.unlock_weapons_from(ctx, kind);
                    if last {
                        ctx.audio.stop_music(1.5);
                        self.boss_music = false;
                    }
                    self.director.request(Moment::Boss);
                    self.sfx(ctx, "boss_die");
                    self.juice.flash([1.0, 1.0, 1.0, 0.7], 0.6);
                    for (k, r) in [60.0, 110.0, 170.0].iter().enumerate() {
                        self.juice.ring(pos - Vec2::new(0.0, 16.0), 4.0 + k as f32 * 10.0, *r, 0.5 + k as f32 * 0.15, if k == 1 { GOLD } else { WHITE });
                    }
                    self.juice.debris(pos - Vec2::new(0.0, 16.0), Vec2::ZERO, 40, GOLD, 180.0, true);
                    self.juice.add_trauma(0.8, 1.0);
                    let s = self.lang.get("ui.boss_down").to_string();
                    self.hud.banner(&s, GOLD, 3.0);
                    if self.boss_music {
                        let id = self.world.map.music.clone();
                        self.play_music(ctx, &id, 2.0);
                        self.boss_music = false;
                    }
                }
                Event::PortalOpened { pos } => {
                    self.sfx(ctx, "portal_open");
                    let s = self.lang.get("ui.portal").to_string();
                    self.hud.banner(&s, rgb(150, 240, 150), 3.5);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, 70.0, 0.8, rgb(180, 255, 180));
                    self.world.vacuum();
                }
                Event::Victory { pos } => {
                    self.sfx(ctx, "victory");
                    self.juice.flash([1.0, 1.0, 1.0, 0.8], 0.8);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 6.0, 120.0, 0.8, GOLD);
                    let root = ctx.root.clone();
                    self.commit_run(&root, true);
                    self.collect_unlocks();
                    ctx.audio.stop_music(0.3);
                    self.play_music(ctx, "great_victory", 0.0);
                    self.state = if self.pending_unlocks.is_empty() {
                        State::Victory { t: 0.0, sel: 0 }
                    } else {
                        State::Unlocks { items: std::mem::take(&mut self.pending_unlocks), t: 0.0, next: menus::AfterUnlock::Victory }
                    };
                }
                Event::ChestDropped { pos, big } => {
                    self.sfx(ctx, "chest_drop");
                    if self.world.evolution_ready().is_some() {
                        for k in 0..3 {
                            self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0 + k as f32 * 6.0, 40.0 + k as f32 * 18.0, 0.6 + k as f32 * 0.15, rgb(255, 130, 240));
                        }
                    }
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 2.0, if big { 50.0 } else { 30.0 }, 0.5, GOLD);
                }
                Event::Frozen { pos } => {
                    self.juice.anim_ex(self.vis.ice_shatter, pos - Vec2::new(0.0, 8.0), 0.18, 0.0, 1.0, ICE, true);
                    self.sfx(ctx, "frozen");
                }
                Event::EncounterSpawned { .. } => {
                    self.sfx(ctx, "encounter");
                    let s = self.lang.get("ui.encounter_spawned").to_string();
                    self.hud.banner(&s, CHAMPION, 3.5);
                }
                Event::EncounterFled { pos } => {
                    self.sfx(ctx, "encounter_fled");
                    let s = self.lang.get("ui.encounter_fled").to_string();
                    self.hud.banner(&s, rgb(190, 180, 210), 2.4);
                    self.juice.ring(pos - Vec2::new(0.0, 10.0), 26.0, 2.0, 0.45, CHAMPION);
                }
                Event::EncounterWon { pos } => {
                    self.sfx(ctx, "encounter_won");
                    let s = self.lang.get("ui.encounter_won").to_string();
                    self.hud.banner(&s, CHAMPION, 2.6);
                    self.juice.ring(pos - Vec2::new(0.0, 10.0), 4.0, 70.0, 0.55, CHAMPION);
                    self.juice.ring(pos - Vec2::new(0.0, 10.0), 2.0, 40.0, 0.4, WHITE);
                    self.juice.flash([0.9, 0.7, 1.0, 0.25], 0.3);
                    self.juice.add_trauma(0.35, 0.5);
                }
                Event::BombSpawned { .. } => {
                    self.sfx(ctx, "bomb_spawn");
                    let s = self.lang.get("ui.bomb_spawned").to_string();
                    self.hud.banner(&s, rgb(255, 150, 80), 2.2);
                }
                Event::ScreenClear { pos, count } => {
                    self.sfx(ctx, "screen_clear");
                    self.sfx(ctx, "screen_rumble");
                    self.sfx(ctx, "explosion");
                    self.juice.flash([1.0, 0.97, 0.85, 0.85], 0.5);
                    self.juice.hitstop(0.12);
                    self.juice.add_trauma(0.95, 1.0);
                    let c = pos - Vec2::new(0.0, 8.0);
                    for (k, r) in [40.0, 130.0, 260.0, 420.0].iter().enumerate() {
                        self.juice.ring(c, 4.0 + k as f32 * 10.0, *r, 0.45 + k as f32 * 0.12, if k % 2 == 0 { FIRE } else { WHITE });
                    }
                    self.juice.anim_ex(self.vis.explosion, c, 0.36, 0.0, 4.0, WHITE, false);
                    // explosions en cascade sur tout l'ecran
                    let half = self.world.view_half * 0.85;
                    let center = self.world.player.pos;
                    for _ in 0..16 {
                        let q = center + Vec2::new(self.rng.range(-half.x, half.x), self.rng.range(-half.y, half.y));
                        let sc = if self.rng.chance(0.4) { 2.0 } else { 1.0 };
                        self.juice.anim_ex(self.vis.explosion, q, 0.36, 0.0, sc, WHITE, false);
                    }
                    self.juice.debris(c, Vec2::ZERO, 40, FIRE, 220.0, true);
                    self.director.request(Moment::Boss);
                    if count > 0 {
                        let s = self.lang.get("ui.screen_clear").replace("{n}", &count.to_string());
                        self.hud.banner(&s, rgb(255, 150, 80), 3.0);
                    }
                }
                Event::PropBroken { pos, kind } => {
                    let pd = &self.world.map.props[kind as usize];
                    let tint = pd.tint.map(|(r, g, b, a)| [1.0 + (r - 1.0) * a, 1.0 + (g - 1.0) * a, 1.0 + (b - 1.0) * a, 1.0]).unwrap_or(WHITE);
                    let pot = pd.break_fx.contains("pot");
                    if let Some(a) = self.vis.prop_break.get(kind as usize).copied().flatten() {
                        self.juice.anim_ex(a, pos - Vec2::new(0.0, 10.0), 0.44, 0.0, 1.0, tint, false);
                    }
                    let leaf = if pot { [0.9, 0.85, 0.75, 1.0] } else { [0.3 * tint[0], 0.65 * tint[1], 0.25 * tint[2], 1.0] };
                    self.juice.debris(pos - Vec2::new(0.0, 6.0), Vec2::ZERO, 5, leaf, 70.0, false);
                    self.sfx(ctx, if pot { "pot_break" } else { "bush_break" });
                    self.juice.add_trauma(0.03, 0.2);
                }
                Event::Burst { pos, .. } => {
                    self.sfx(ctx, "somaria_burst");
                    self.juice.ring(pos, 3.0, 34.0, 0.3, FIRE);
                    self.juice.anim_ex(self.vis.explosion, pos, 0.3, 0.0, 1.0, WHITE, false);
                    self.juice.add_trauma(0.06, 0.4);
                }
                Event::ThiefSpawned { .. } => {
                    self.sfx(ctx, "thief_spawn");
                    let s = self.lang.get("ui.thief_spawned").to_string();
                    self.hud.banner(&s, THIEF, 3.0);
                }
                Event::ThiefFled { pos } => {
                    self.sfx(ctx, "encounter_fled");
                    let s = self.lang.get("ui.thief_fled").to_string();
                    self.hud.banner(&s, rgb(190, 210, 190), 2.2);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 22.0, 2.0, 0.4, THIEF);
                }
                Event::ThiefLooted { pos, count } => {
                    self.sfx(ctx, "thief_loot");
                    let s = self.lang.get("ui.thief_looted").replace("{n}", &count.to_string());
                    self.hud.banner(&s, THIEF, 2.6);
                    self.juice.flash([0.7, 1.0, 0.8, 0.25], 0.3);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 4.0, 90.0, 0.5, THIEF);
                    self.juice.debris(pos - Vec2::new(0.0, 8.0), Vec2::ZERO, 18, rgb(140, 255, 140), 120.0, true);
                    self.juice.add_trauma(0.3, 0.5);
                }
                Event::NpcArrived { kind, pos } => {
                    self.sfx(ctx, "npc_arrive");
                    let (key, col) = match kind {
                        sim::npc::NpcKind::Fortune => ("ui.fortune.arrived", crate::fortune::PURPLE),
                        sim::npc::NpcKind::Witch => {
                            // potions tirees a l'arrivee : les memes jusqu'a son depart
                            let d = self.meta.difficulty(&self.profile).map(|d| d.rupees).unwrap_or(1.0);
                            self.witch_offer = crate::witch::roll(&mut self.rng, self.world.map.power, d, self.world.time);
                            ("ui.witch.arrived", npcs::WITCH)
                        }
                    };
                    let s = self.lang.get(key).to_string();
                    self.hud.banner(&s, col, 3.0);
                    // apparition : eclair violet, anneaux et etincelles
                    self.juice.flash([0.8, 0.5, 1.0, 0.35], 0.4);
                    for (k, r) in [40.0, 80.0].iter().enumerate() {
                        self.juice.ring(pos - Vec2::new(0.0, 12.0), 4.0, *r, 0.5 + k as f32 * 0.2, col);
                    }
                    self.juice.debris(pos - Vec2::new(0.0, 12.0), Vec2::ZERO, 24, col, 110.0, true);
                }
                Event::NpcMet { kind } => self.npc_met = Some(kind),
                Event::NpcLeft { kind, pos } => {
                    self.sfx(ctx, "npc_leave");
                    self.juice.debris(pos - Vec2::new(0.0, 12.0), Vec2::ZERO, 16, rgb(200, 180, 255), 90.0, true);
                    // partie sans achat : la sorciere s'en va
                    if kind == sim::npc::NpcKind::Witch && !self.witch_offer.is_empty() {
                        self.witch_offer.clear();
                        let s = self.lang.get("ui.witch.left").to_string();
                        self.hud.banner(&s, npcs::WITCH, 2.0);
                    }
                }
                Event::Medallion { pos } => {
                    self.sfx(ctx, "medallion");
                    let s = self.lang.get("ui.pickup_medallion").to_string();
                    self.hud.banner(&s, ICE, 2.2);
                    self.juice.flash([0.7, 0.85, 1.0, 0.5], 0.45);
                    self.juice.ring(pos - Vec2::new(0.0, 8.0), 8.0, 380.0, 0.6, ICE);
                    self.juice.add_trauma(0.35, 0.6);
                }
            }
        }
        // un gros impact (explosion, seisme...) qui touche beaucoup d'ennemis : ralenti tres court
        if self.tick_area_hits >= 18 && self.director.request(Moment::Impact) {
            self.juice.flash([1.0, 1.0, 0.9, 0.12], 0.12);
        }
        self.events = evs;
    }

    /// Grosse vague vaincue : effets (toujours) et ralenti (si le directeur l'accorde).
    fn mass_kill(&mut self, ctx: &mut Ctx) {
        if self.mass_fx_cd > 0.0 {
            return;
        }
        self.mass_fx_cd = 2.5;
        let pos = self.world.player.pos - Vec2::new(0.0, 8.0);
        let slowed = self.director.request(Moment::MassKill);
        self.sfx(ctx, "mass_kill");
        self.juice.ring(pos, 10.0, 200.0, 0.5, with_alpha(GOLD, 0.8));
        self.juice.add_trauma(0.2, 0.5);
        if slowed {
            self.juice.flash([1.0, 1.0, 0.85, 0.16], 0.25);
            self.juice.ring(pos, 4.0, 120.0, 0.4, WHITE);
        }
    }

    /// « Le Link bleu, ca te fera 500 rubis rouges. Tu le prends ? » (question d'achat d'une couleur).
    fn skin_question(&self, id: &str, skin: usize) -> String {
        let def = &self.content.characters[id];
        let price = chars::skin_price(&self.meta, &self.profile, &def.skins[skin].0).unwrap_or(0);
        self.lang
            .get("ui.skin_ask")
            .replace("{c}", &crate::hud::hl(self.lang.get(&format!("skin.{}", def.skins[skin].1))))
            .replace("{n}", &crate::hud::hl(&chars::name(&self.lang, id)))
            .replace("{p}", &crate::hud::hl(&price.to_string()))
    }

    /// Choix d'une montee de niveau : une fusion proposee passe en tete (preselectionnee) et le jingle
    /// des secrets d'A Link to the Past l'annonce.
    fn levelup_offers(&mut self, ctx: &mut Ctx) -> Vec<Offer> {
        let mut offers = self.world.roll_offers();
        offers.sort_by_key(|o| !matches!(o, Offer::Evolve(_)));
        if matches!(offers.first(), Some(Offer::Evolve(_))) {
            self.sfx(ctx, "fusion_offer");
        }
        offers
    }

    /// Niveaux et coffres en attente : ouvre l'ecran correspondant.
    fn check_pending(&mut self, ctx: &mut Ctx) -> bool {
        // test de charge : choix automatiques, sans ecran
        if self.opts.bench.is_some() {
            while self.world.pending_levels > 0 {
                self.world.pending_levels -= 1;
                let o = self.world.roll_offers();
                self.world.apply_offer(o[0]);
            }
            while let Some(big) = self.world.pending_chests.pop() {
                self.world.open_chest(big);
            }
            if let Some(k) = self.npc_met.take() {
                self.world.remove_npc(k);
            }
            while self.world.pending_rewards > 0 {
                self.world.pending_rewards -= 1;
                let o = reward::roll(&self.world, &self.meta, &self.profile, &self.run_relics, &mut self.rng);
                if let Some(id) = reward::apply(&mut self.world, &self.meta, o[0]) {
                    self.run_relics.push(id);
                    self.hud.relics = self.relic_icons(&ctx.atlas);
                }
            }
            return false;
        }
        // coop : un choix local attend encore son pas (quelques images) avant le suivant
        if self.coop_queued() {
            return false;
        }
        // tout est au maximum et le bonus de niveau est deja choisi : il s'applique sans ecran
        if let Some(f) = self.fallback {
            let mut given = false;
            let mut left = self.world.pending_levels;
            while left > 0 && self.world.roll_offers().iter().all(|o| matches!(o, Offer::Heal | Offer::Rupees)) {
                left -= 1;
                self.take_level(Some(f));
                given = true;
            }
            if given {
                self.sfx(ctx, if f == Offer::Heal { "fairy_pickup" } else { "pick" });
            }
        }
        if self.world.pending_levels > 0 {
            let offers = self.levelup_offers(ctx);
            self.state = State::LevelUp { offers, sel: 0, t: 0.0 };
            ctx.audio.duck_music(0.45, 0.2);
            return true;
        }
        if self.world.pending_rewards > 0 {
            let offers = reward::roll(&self.world, &self.meta, &self.profile, &self.run_relics, &mut self.rng);
            self.state = State::Reward { offers, sel: 0, t: 0.0 };
            ctx.audio.duck_music(0.45, 0.2);
            return true;
        }
        if !self.world.pending_chests.is_empty() {
            // coop : le coffre reste en attente jusqu'a son ouverture, au meme pas dans tous les jeux
            let big = if self.coop_run() { self.world.pending_chests[0] } else { self.world.pending_chests.remove(0) };
            let options = self.world.evolution_choices();
            if options.len() > 1 {
                self.state = State::EvoChoice { options, big, sel: 0, t: 0.0 };
                ctx.audio.duck_music(0.35, 0.2);
                return true;
            }
            let reward = self.take_chest(big, None);
            self.state = State::Chest { reward, big, t: 0.0 };
            ctx.audio.duck_music(0.35, 0.2);
            return true;
        }
        if let Some(kind) = self.npc_met.take() {
            self.sfx(ctx, "pause");
            match kind {
                sim::npc::NpcKind::Fortune => {
                    let sel = (0..2).find(|&k| !self.fortune_played[k]).unwrap_or(2);
                    self.state = State::Fortune { phase: crate::fortune::Phase::Menu, sel, t: 0.0 };
                    self.play_music(ctx, "fortune_teller", 0.5);
                }
                sim::npc::NpcKind::Witch => {
                    self.state = State::Witch { sel: 0, t: 0.0 };
                    ctx.audio.duck_music(0.45, 0.2);
                }
            }
            return true;
        }
        false
    }

    fn resume(&mut self, ctx: &mut Ctx) {
        if !self.check_pending(ctx) {
            self.state = State::Playing;
            ctx.audio.duck_music(1.0, 0.25);
        }
        self.suppress = true;
    }

    fn bench_refill(&mut self) {
        let Some(n) = self.opts.bench else { return };
        let have = self.world.horde.len() as u32;
        if have < n {
            let kinds = ["soldier_green", "soldier_blue", "rat", "rope"];
            let k = self.content.enemy_index(kinds[self.rng.below(4) as usize]).unwrap_or(0);
            self.world.spawn_burst(k, (n - have).min(64), Pattern::Pack);
        }
    }

    /// Menu de l'ecran titre (avec l'entree de mise a jour quand il y en a une).
    fn title_items(&self) -> Vec<(TitleItem, String)> {
        let l = &self.lang;
        let mut v = vec![
            (TitleItem::Play, l.get("ui.play").to_string()),
            (TitleItem::Coop, l.get("ui.coop").to_string()),
            (TitleItem::Players, l.get("ui.player_item").replace("{s}", &self.profile.name)),
            (TitleItem::Shop, l.get("ui.shop").to_string()),
            (TitleItem::Collection, l.get("ui.collection").to_string()),
            (TitleItem::Gallery, l.get("ui.gallery").to_string()),
            (TitleItem::Exploits, l.get("ui.exploits").to_string()),
            (TitleItem::Scores, l.get("ui.scores").to_string()),
            (TitleItem::Options, l.get("ui.options").to_string()),
            (TitleItem::About, l.get("ui.about").to_string()),
        ];
        let upd = match self.updater.status() {
            UpdateStatus::Available(r) => Some(l.get("ui.update.install").replace("{v}", &r.version)),
            UpdateStatus::Downloading { done, total, .. } => {
                let p = (done * 100).checked_div(total).map_or(0, |p| p.min(99));
                Some(l.get("ui.update.progress").replace("{p}", &p.to_string()))
            }
            UpdateStatus::Ready(_) => Some(l.get("ui.update.restart").to_string()),
            UpdateStatus::Failed(_) => Some(l.get("ui.update.failed").to_string()),
            _ => None,
        };
        if let Some(s) = upd {
            v.push((TitleItem::Update, s));
        }
        // dans le navigateur, on quitte en fermant l'onglet
        if cfg!(not(target_arch = "wasm32")) {
            v.push((TitleItem::Quit, l.get("ui.quit").to_string()));
        }
        v
    }

    /// Mises a jour automatiques : le telechargement part seul des qu'une version est trouvee.
    /// Ecran titre sous l'orage : pluie, tonnerre apres chaque eclair, image sans filtre.
    /// En partie (et dans les options, pour juger du rendu), le filtre choisi par le joueur.
    fn title_ambience(&mut self, ctx: &mut Ctx) {
        // galerie : la musique seule, sans l'orage du titre
        if matches!(self.state, State::Gallery { .. }) {
            ctx.audio.stop_ambience(1.2);
            return;
        }
        let scene = self.state.in_title();
        if scene {
            if let (Some(id), false) = (self.audio_cfg.menu_ambience.clone(), self.opts.uicheck) {
                if let Some(def) = self.audio_cfg.music_def(&id) {
                    let root = ctx.root.clone();
                    ctx.audio.play_ambience(&id, &def, &root, 1.5);
                }
            }
            if let Some((k, at)) = title::strike_before(self.menu_t) {
                if self.menu_t >= at + title::THUNDER_DELAY && self.thunder.is_none_or(|t| t < k) {
                    self.thunder = Some(k);
                    self.sfx_p(ctx, "storm_thunder", 0.8, 1.0);
                }
            }
        } else {
            let amb = if self.opts.uicheck || self.opts.bench.is_some() { None } else { self.world.map.ambience.clone() };
            match amb.and_then(|id| self.audio_cfg.music_def(&id).map(|d| (id, d))) {
                Some((id, def)) => {
                    let root = ctx.root.clone();
                    ctx.audio.play_ambience(&id, &def, &root, 2.0);
                }
                None => ctx.audio.stop_ambience(1.2),
            }
            // tonnerre des stages sous l'orage
            if self.world.map.weather == sim::defs::Weather::Rain && matches!(self.state, State::Playing) {
                if let Some((k, at)) = weather::strike_before(self.world.time) {
                    if self.world.time >= at + 0.35 && self.thunder.is_none_or(|t| t < k) {
                        self.thunder = Some(k);
                        self.sfx_p(ctx, "storm_thunder", 0.7, 1.0);
                    }
                }
            }
        }
        let bare = matches!(self.state, State::Title { .. } | State::About { .. });
        let want = if bare { VideoFilter::None } else { self.settings.video };
        if ctx.video != want {
            ctx.set_video(want, ctx.aspect);
        }
    }

    fn poll_updates(&mut self) {
        if self.update_started || self.settings.updates != UpdateMode::Auto {
            return;
        }
        if let UpdateStatus::Available(r) = self.updater.status() {
            self.update_started = true;
            self.updater.download(r);
        }
    }

    /// Applique un changement d'option.
    fn change_option(&mut self, ctx: &mut Ctx, o: Opt, step: i32, set: Option<f32>) {
        let s = &mut self.settings;
        match o {
            Opt::Language => {
                let cur = LANGS.iter().position(|&c| c == s.lang).unwrap_or(0) as i32;
                let next = (cur + step).rem_euclid(LANGS.len() as i32) as usize;
                s.lang = LANGS[next].to_string();
                self.lang = Lang::load(&ctx.root, &s.lang);
            }
            Opt::Video => {
                let all = VideoFilter::ALL;
                let cur = all.iter().position(|&f| f == s.video).unwrap_or(0) as i32;
                s.video = all[(cur + step).rem_euclid(all.len() as i32) as usize];
                ctx.set_video(s.video, s.aspect);
            }
            Opt::Aspect => {
                s.aspect = if s.aspect == Aspect::Wide { Aspect::Classic } else { Aspect::Wide };
                ctx.set_video(s.video, s.aspect);
            }
            Opt::Filter => {
                let all = AudioFilter::ALL;
                let cur = all.iter().position(|&f| f == s.filter).unwrap_or(0) as i32;
                s.filter = all[(cur + step).rem_euclid(all.len() as i32) as usize];
                s.apply_audio(&mut ctx.audio);
            }
            Opt::Master | Opt::Music | Opt::Sfx => {
                let v = match o {
                    Opt::Master => &mut s.master,
                    Opt::Music => &mut s.music,
                    _ => &mut s.sfx,
                };
                *v = match set {
                    Some(x) => x,
                    None => ((*v * 10.0).round() + step as f32).clamp(0.0, 10.0) / 10.0,
                };
                s.apply_audio(&mut ctx.audio);
                ctx.audio.duck_music(0.35, 0.05);
            }
            Opt::Fullscreen => {
                s.fullscreen = !ctx.fullscreen;
                ctx.set_fullscreen(s.fullscreen);
            }
            Opt::Updates => {
                let all = UpdateMode::ALL;
                let cur = all.iter().position(|&m| m == s.updates).unwrap_or(0) as i32;
                s.updates = all[(cur + step).rem_euclid(all.len() as i32) as usize];
                if s.updates != UpdateMode::Off {
                    self.updater.check();
                }
            }
            Opt::Controls | Opt::Save | Opt::Back => {}
        }
        self.sfx(ctx, "cursor");
    }

    /// Logique des menus (temps reel).
    /// Boutique, ou Preparatifs (`prep`) : memes onglets, ligne 0 des Preparatifs = « Partir ! ».
    /// None : la partie vient d'etre lancee.
    fn update_shop(&mut self, ctx: &mut Ctx, nav: &NavIn, mut tab: usize, mut sel: usize, prep: bool) -> Option<State> {
        let book = if prep { shop::PREP } else { shop::SHOP };
        let off = prep as usize;
        let ntabs = book.len();
        let make = |tab, sel| if prep { State::Prep { tab, sel } } else { State::Shop { tab, sel } };
        let (rows, _) = shop::rows(&self.meta, &self.content, &self.profile, &self.lang, &ctx.atlas, tab);
        let n = rows.len() + off;
        let mut rel = tab - book.start;
        let input = hub::input(nav, ctx.view, ntabs, rel, n, &mut sel);
        let side = match input {
            hub::Input::Left(k) => Some((k, -1)),
            hub::Input::Right(k) => Some((k, 1)),
            _ => None,
        };
        if let Some((k, dir)) = side {
            if k >= off && k < n && shop::row_uses_sides(tab, k - off) {
                self.shop_act(ctx, tab, k - off, dir);
            } else {
                rel = (rel as i32 + dir).rem_euclid(ntabs as i32) as usize;
                tab = book.start + rel;
                sel = 0;
                self.sfx(ctx, "cursor");
            }
            return Some(make(tab, sel));
        }
        match input {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                return Some(if prep && self.coop.as_ref().is_some_and(|s| !s.is_host()) {
                    self.char_select()
                } else if prep {
                    State::StageSelect { sel: self.stage_ids.iter().position(|s| *s == self.opts.map).unwrap_or(0) }
                } else {
                    State::Title { sel: TITLE_SHOP }
                });
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                if k != rel {
                    tab = book.start + k;
                    if sel != hub::TABS_ROW {
                        sel = 0;
                    }
                    self.sfx(ctx, "cursor");
                }
            }
            hub::Input::Pick(0) if prep && self.coop.is_some() => {
                self.sfx(ctx, "confirm");
                self.coop_set_ready(ctx);
                return Some(State::Lobby { sel: 0 });
            }
            hub::Input::Pick(0) if prep => {
                self.sfx(ctx, "confirm");
                ctx.audio.stop_music(0.6);
                self.restart(ctx);
                return None;
            }
            // juke-box : une musique pas encore achetee s'ecoute d'abord en extrait ; un second appui l'achete
            hub::Input::Pick(k)
                if tab == shop::TAB_JUKEBOX
                    && self.meta.jukebox.get(k - off).is_some_and(|t| !shop::track_owned(t, &self.profile))
                    && self.jukebox_try.map(|t| t.0) != Some(k - off) =>
            {
                self.preview_track(ctx, k - off);
            }
            hub::Input::Pick(k) => self.shop_act(ctx, tab, k - off, 0),
            _ => {}
        }
        Some(make(tab, sel))
    }

    /// Extrait d'une musique du juke-box (la musique du menu reprend ensuite, voir `tick_preview`).
    fn preview_track(&mut self, ctx: &mut Ctx, k: usize) {
        let id = self.meta.jukebox[k].id.clone();
        if let Some(def) = self.audio_cfg.music_def(&id) {
            let root = ctx.root.clone();
            ctx.audio.preview_music(&id, &def, &root);
        }
        self.jukebox_try = Some((k, ctx.real_time));
        self.jukebox_playing = true;
        self.sfx(ctx, "cursor");
    }

    /// Fin de l'extrait (duree ecoulee ou juke-box quitte) : la musique du menu revient en fondu.
    fn tick_preview(&mut self, ctx: &mut Ctx) {
        let Some((_, t0)) = self.jukebox_try else { return };
        let here = matches!(self.state, State::Shop { tab: shop::TAB_JUKEBOX, .. });
        if self.jukebox_playing && (!here || ctx.real_time - t0 >= JUKEBOX_PREVIEW) {
            self.jukebox_playing = false;
            let id = self.menu_music();
            self.play_music(ctx, &id, if here { 1.2 } else { 0.5 });
        }
        if !here {
            self.jukebox_try = None;
        }
    }

    fn shop_act(&mut self, ctx: &mut Ctx, tab: usize, k: usize, dir: i32) {
        let outcome = shop::act(&self.meta, &self.content, &self.lang, &mut self.profile, tab, k, dir);
        if matches!(outcome, shop::Outcome::Done | shop::Outcome::Selected) {
            match tab {
                // garde-robe : la tenue achetee (ou choisie) est portee tout de suite
                shop::TAB_WARDROBE => {
                    let w = shop::wardrobe_items(&self.meta, &self.content, &self.profile, &self.lang).get(k).map(|w| (w.character.clone(), w.sprite.clone()));
                    if let Some((ch, sprite)) = w {
                        if let Some(i) = self.content.characters.get(&ch).and_then(|d| d.skins.iter().position(|s| s.0 == sprite)) {
                            self.profile.skins.insert(ch, i);
                            chars::apply_skins(&mut self.content, &self.profile, &self.meta);
                        }
                    }
                }
                // juke-box : la musique choisie passe aussitot (et devient celle de l'ecran titre) ;
                // achetee pendant son extrait, elle continue sans repartir du debut
                shop::TAB_JUKEBOX => {
                    self.jukebox_try = None;
                    self.jukebox_playing = false;
                    let id = self.menu_music();
                    self.play_music(ctx, &id, 0.4);
                }
                _ => {}
            }
        }
        match outcome {
            shop::Outcome::Done => {
                self.sfx(ctx, "buy");
                self.hub_toast.1 = 0.0;
                self.profile.save(&ctx.root);
            }
            shop::Outcome::Selected => {
                self.sfx(ctx, "equip");
                self.hub_toast.1 = 0.0;
                self.profile.save(&ctx.root);
            }
            shop::Outcome::Refused(key) => {
                self.sfx(ctx, "error");
                self.toast(key);
            }
            shop::Outcome::Nothing => {}
        }
    }

    fn update_records(&mut self, ctx: &mut Ctx, nav: &NavIn, mut tab: usize, mut sel: usize, mut armed: bool) -> State {
        // fiche d'un ennemi ouverte : gauche / droite (ou haut / bas) pour passer au suivant,
        // A, Echap ou un clic pour fermer
        if let Some((k, _)) = self.beast {
            let id = self.content.enemy_ids[k].clone();
            let owned = self.profile.figurines.contains(&id);
            if (nav.confirm || nav.click) && !owned {
                // figurine : A (ou un clic) l'achete, Echap ferme la fiche
                let cost = self.meta.figurine_cost(&self.content.enemies[k], &id);
                if self.profile.rupees >= cost {
                    self.profile.rupees -= cost;
                    self.profile.figurines.insert(id);
                    self.profile.save(&ctx.root);
                    self.sfx(ctx, "buy");
                    self.beast = Some((k, ctx.real_time - 0.5));
                    self.beast_bought = ctx.real_time;
                } else {
                    self.sfx(ctx, "error");
                    self.toast("ui.not_enough");
                }
            } else if nav.back || nav.confirm || nav.click {
                self.beast = None;
                self.sfx(ctx, "confirm");
            } else if nav.left || nav.right || nav.up || nav.down {
                let known = records::bestiary_known(&self.content, &self.profile);
                if let Some(i) = known.iter().position(|&e| e == k) {
                    let d = if nav.left || nav.up { -1 } else { 1 };
                    let e = known[(i as i32 + d).rem_euclid(known.len() as i32) as usize];
                    self.beast = Some((e, ctx.real_time));
                    sel = records::bestiary_order(&self.content, &self.profile).iter().position(|&o| o == e).unwrap_or(sel);
                    self.sfx(ctx, "cursor");
                }
            }
            return State::Records { tab, sel, armed };
        }
        // fiche d'une arme, d'une fusion ou d'un objet : gauche / droite pour le suivant, A, Echap ou un clic ferme
        if let Some((ct, k, _)) = self.card {
            if nav.back || nav.confirm || nav.click {
                self.card = None;
                self.sfx(ctx, "confirm");
            } else if nav.left || nav.right || nav.up || nav.down {
                let (all, known) = records::card_items(&self.content, &self.meta, &self.profile, ct);
                if let Some(i) = known.iter().position(|&e| e == k) {
                    let d = if nav.left || nav.up { -1 } else { 1 };
                    let e = known[(i as i32 + d).rem_euclid(known.len() as i32) as usize];
                    self.card = Some((ct, e, ctx.real_time));
                    sel = all.iter().position(|&o| o == e).unwrap_or(sel);
                    self.sfx(ctx, "cursor");
                }
            }
            return State::Records { tab, sel, armed };
        }
        // Recueil, Exploits ou Sauvegarde : seulement les onglets de cet ecran
        let (range, _) = records::book(tab);
        let ntabs = range.len();
        let (rows, _) = records::rows(&self.content, &self.meta, &ctx.atlas, &self.vis, &self.profile, &self.lang, tab, armed);
        let n = rows.len();
        let input = hub::input(nav, ctx.view, ntabs, tab - range.start, n, &mut sel);
        if !matches!(input, hub::Input::None | hub::Input::Pick(_)) {
            armed = false;
        }
        match input {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                return match tab {
                    records::TAB_SAVE => State::Options { sel: menus::OPTIONS.iter().position(|o| *o == Opt::Save).unwrap_or(0), from_title: true },
                    t if t >= records::TAB_TROPHIES => State::Title { sel: TITLE_EXPLOITS },
                    _ => State::Title { sel: TITLE_COLLECTION },
                };
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                let k = range.start + k;
                if k != tab {
                    tab = k;
                    if sel != hub::TABS_ROW {
                        sel = 0;
                    }
                    self.sfx(ctx, "cursor");
                }
            }
            hub::Input::Left(_) | hub::Input::Right(_) => {
                let dir = if matches!(input, hub::Input::Left(_)) { -1 } else { 1 };
                tab = range.start + ((tab - range.start) as i32 + dir).rem_euclid(ntabs as i32) as usize;
                sel = 0;
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if tab == 0 => {
                let e = records::bestiary_order(&self.content, &self.profile).get(k).copied();
                if let Some(e) = e.filter(|&e| self.profile.bestiary.get(&self.content.enemy_ids[e]).copied().unwrap_or(0) > 0) {
                    self.beast = Some((e, ctx.real_time));
                    self.sfx(ctx, "confirm");
                } else {
                    self.sfx(ctx, "error");
                }
            }
            hub::Input::Pick(k) if tab == records::TAB_GALLERY => {
                self.gallery = Some(gallery::Room::new(&self.content, &self.meta, &self.profile, k));
                self.sfx(ctx, "confirm");
                self.play_music(ctx, GALLERY_MUSIC, 0.6);
                return State::Gallery { wing: k, from_title: false };
            }
            hub::Input::Pick(k) if matches!(tab, records::TAB_WEAPONS | records::TAB_FUSIONS | records::TAB_ITEMS) => {
                let (all, known) = records::card_items(&self.content, &self.meta, &self.profile, tab);
                match all.get(k).filter(|e| known.contains(e)) {
                    Some(&e) => {
                        self.card = Some((tab, e, ctx.real_time));
                        self.sfx(ctx, "confirm");
                    }
                    None => self.sfx(ctx, "error"),
                }
            }
            hub::Input::Pick(k) if tab == records::TAB_SAVE && k == records::SAVE_EXPORT => {
                armed = false;
                let code = self.profile.export_code();
                if clipboard::set(&code) {
                    self.sfx(ctx, "save_code");
                    self.toast("ui.exported");
                } else if storage::write(&ctx.root, CODE_FILE, &code).is_ok() {
                    self.sfx(ctx, "save_code");
                    self.toast("ui.exported_file");
                } else {
                    self.sfx(ctx, "error");
                    self.toast("ui.export_failed");
                }
            }
            hub::Input::Pick(k) if tab == records::TAB_SAVE && k == records::SAVE_IMPORT => {
                if !armed {
                    armed = true;
                    self.sfx(ctx, "cursor");
                    self.toast("ui.import_warn");
                } else {
                    armed = false;
                    let text = clipboard::get()
                        .filter(|t| t.trim().starts_with("ASTP"))
                        .or_else(|| storage::read(&ctx.root, CODE_FILE));
                    match text.and_then(|t| Profile::import_code(&t)) {
                        Some(p) => {
                            // le code importe remplace le joueur actif (meme emplacement)
                            let slot = self.profile.slot;
                            self.profile = p;
                            self.profile.slot = slot;
                            chars::apply_skins(&mut self.content, &self.profile, &self.meta);
                            self.profile.save(&ctx.root);
                            self.sfx(ctx, "relic");
                            self.toast("ui.imported");
                        }
                        None => {
                            self.sfx(ctx, "error");
                            self.toast("ui.import_failed");
                        }
                    }
                }
            }
            hub::Input::Pick(_) | hub::Input::None => {}
        }
        State::Records { tab, sel, armed }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_controls(
        &mut self,
        ctx: &mut Ctx,
        nav: &NavIn,
        mut tab: usize,
        mut sel: usize,
        wait: Option<(usize, f32)>,
        from_title: bool,
        dt: f32,
    ) -> State {
        if let Some((k, t)) = wait {
            let t = t - dt;
            let wait = match controls::read_new(&ctx.input, tab) {
                Some(Some(b)) => {
                    for (a, list) in controls::rebind(&self.bindings, controls::ACTIONS[k], tab, b) {
                        self.settings.bindings.insert(a, list);
                    }
                    self.bindings = self.settings.bindings(&self.base_bindings);
                    self.settings.save(&ctx.root);
                    self.sfx(ctx, "equip");
                    None
                }
                Some(None) => {
                    self.sfx(ctx, "cursor");
                    None
                }
                None if t <= 0.0 => None,
                None => Some((k, t)),
            };
            return State::Controls { tab, sel, wait, from_title };
        }
        let n = controls::RESET + 1;
        let mut wait = None;
        match hub::input(nav, ctx.view, controls::TABS.len(), tab, n, &mut sel) {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                let sel = menus::OPTIONS.iter().position(|o| *o == Opt::Controls).unwrap_or(0);
                return State::Options { sel, from_title };
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                if k != tab {
                    tab = k;
                    if sel != hub::TABS_ROW {
                        sel = 0;
                    }
                    self.sfx(ctx, "cursor");
                }
            }
            hub::Input::Left(_) | hub::Input::Right(_) => {
                tab = 1 - tab.min(1);
                sel = 0;
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if k == controls::RESET => {
                for a in controls::ACTIONS {
                    self.settings.bindings.remove(a);
                }
                self.bindings = self.settings.bindings(&self.base_bindings);
                self.settings.save(&ctx.root);
                self.sfx(ctx, "equip");
                self.toast("ui.keys_reset");
            }
            hub::Input::Pick(k) => {
                self.sfx(ctx, "confirm");
                self.hub_toast.1 = 0.0;
                wait = Some((k, controls::WAIT));
            }
            hub::Input::None => {}
        }
        State::Controls { tab, sel, wait, from_title }
    }

    /// Avatar propose a un nouveau joueur : le premier que personne n'a pris.
    fn free_avatar(&self, slot: usize) -> usize {
        let n = players::AVATARS.len();
        let used: Vec<usize> = self.slots.iter().flatten().filter(|p| p.slot != slot).map(|p| p.avatar).collect();
        (0..n).map(|a| (a + slot * 4) % n).find(|a| !used.contains(a)).unwrap_or(0)
    }

    /// Change de joueur actif (le profil courant est enregistre d'abord).
    fn switch_profile(&mut self, root: &std::path::Path, slot: usize) {
        self.profile.save(root);
        self.profile = Profile::load_slot(root, slot).unwrap_or_else(|| Profile::fresh(slot, profiles::DEFAULT_NAME, 0));
        profiles::set_active(root, slot);
        chars::apply_skins(&mut self.content, &self.profile, &self.meta);
        self.slots = profiles::all_slots(root);
    }

    fn update_players(&mut self, ctx: &mut Ctx, nav: &NavIn, mut sel: usize, mut armed: bool) -> State {
        let root = ctx.root.clone();
        let active = self.profile.slot;
        let act = players::input(nav, ctx.view, active, &mut sel);
        if !matches!(act, players::Action::None | players::Action::Delete) {
            armed = false;
        }
        match act {
            players::Action::Back => {
                self.sfx(ctx, "confirm");
                return State::Title { sel: TITLE_PLAYERS };
            }
            players::Action::Moved => self.sfx(ctx, "cursor"),
            players::Action::Avatar(d) => {
                let n = players::AVATARS.len() as i32;
                self.profile.avatar = (self.profile.avatar as i32 + d).rem_euclid(n) as usize;
                self.profile.save(&root);
                self.sfx(ctx, "cursor");
            }
            players::Action::Pick(k) if k == active => {
                self.sfx(ctx, "confirm");
                return State::NameEntry { purpose: NamePurpose::Rename, text: self.profile.name.clone(), sel: players::DONE };
            }
            players::Action::Pick(k) if self.slots.get(k).is_some_and(|s| s.is_some()) => {
                self.switch_profile(&root, k);
                self.sfx(ctx, "equip");
                let s = self.lang.get("ui.player_switched").replace("{s}", &self.profile.name);
                self.hub_toast = (s, 2.5);
            }
            players::Action::Pick(k) => {
                self.sfx(ctx, "confirm");
                return State::NameEntry { purpose: NamePurpose::Create(k), text: String::new(), sel: 0 };
            }
            players::Action::Delete if !armed => {
                armed = true;
                sel = players::DELETE;
                self.sfx(ctx, "cursor");
            }
            players::Action::Delete => {
                // le joueur actif est efface ; on passe au premier autre joueur (ou a un nouveau LINK)
                armed = false;
                profiles::delete_slot(&root, active);
                let next = (0..profiles::SLOTS).find(|&k| k != active && self.slots.get(k).is_some_and(|s| s.is_some()));
                self.profile = match next.and_then(|k| Profile::load_slot(&root, k)) {
                    Some(p) => p,
                    None => {
                        let p = Profile::fresh(active, profiles::DEFAULT_NAME, 0);
                        p.save(&root);
                        p
                    }
                };
                profiles::set_active(&root, self.profile.slot);
                chars::apply_skins(&mut self.content, &self.profile, &self.meta);
                self.slots = profiles::all_slots(&root);
                sel = self.profile.slot;
                self.sfx(ctx, "confirm");
                self.toast("ui.player_deleted");
            }
            players::Action::None => {}
        }
        State::Players { sel, armed }
    }

    fn update_name(&mut self, ctx: &mut Ctx, nav: &NavIn, purpose: NamePurpose, mut text: String, mut sel: usize) -> State {
        let act = players::name_input(ctx, nav, &mut text, &mut sel);
        if matches!(purpose, NamePurpose::Room) {
            // code de salle : 4 lettres sans ambiguite, en majuscules
            text = text.to_uppercase().chars().filter(|c| crate::coop::CODE_CHARS.contains(*c)).take(crate::coop::CODE_LEN).collect();
        }
        match act {
            players::NameAction::Moved | players::NameAction::Typed => self.sfx(ctx, "cursor"),
            players::NameAction::Cancel => {
                self.sfx(ctx, "confirm");
                return match purpose {
                    NamePurpose::Create(k) => State::Players { sel: k, armed: false },
                    NamePurpose::Rename => State::Players { sel: self.profile.slot, armed: false },
                    NamePurpose::Pseudo { stage, sel, .. } => State::Scores { tab: scores::MINE, sel, stage, version: String::new(), players: 1 },
                    NamePurpose::Room => State::Coop { sel: 1 },
                    // creation du profil : obligatoire
                    NamePurpose::Setup => {
                        self.sfx(ctx, "error");
                        State::NameEntry { purpose, text, sel }
                    }
                };
            }
            players::NameAction::Done => {
                let name = text.trim().to_string();
                if name.is_empty() || (matches!(purpose, NamePurpose::Room) && name.chars().count() != crate::coop::CODE_LEN) {
                    self.sfx(ctx, "error");
                } else {
                    return self.name_done(ctx, purpose, name);
                }
            }
            players::NameAction::None => {}
        }
        State::NameEntry { purpose, text, sel }
    }

    /// Galerie : Link se promene ; A devant un socle ouvre sa fiche (celles de la Collection), Echap sort.
    fn update_gallery(&mut self, ctx: &mut Ctx, nav: &NavIn, wing: usize, from_title: bool, dt: f32) -> State {
        if self.beast.is_some() || self.card.is_some() {
            // fiche ouverte : memes commandes que dans la Collection (achat de figurine compris)
            let _ = self.update_records(ctx, nav, records::TAB_GALLERY, wing, false);
            return State::Gallery { wing, from_title };
        }
        if nav.back || self.gallery.is_none() {
            self.gallery = None;
            self.sfx(ctx, "confirm");
            let id = self.menu_music();
            self.play_music(ctx, &id, 0.6);
            if from_title {
                return State::Title { sel: TITLE_GALLERY };
            }
            return State::Records { tab: records::TAB_GALLERY, sel: wing, armed: false };
        }
        let mv = self.read_input(ctx).move_dir;
        let Some(room) = self.gallery.as_mut() else { return State::Gallery { wing, from_title } };
        room.step(mv, dt);
        let cry = gallery::cry(&self.content, &self.profile, room);
        let near = room.near();
        if let Some(s) = cry {
            self.sfx_p(ctx, s, 0.5, 1.0);
        }
        if nav.confirm {
            match near.and_then(|e| gallery::card_of(&self.content, &self.profile, e)) {
                Some((records::TAB_BESTIARY, k)) => {
                    self.beast = Some((k, ctx.real_time));
                    self.sfx(ctx, "confirm");
                }
                Some((tab, k)) => {
                    self.card = Some((tab, k, ctx.real_time));
                    self.sfx(ctx, "confirm");
                }
                None if near.is_some_and(|e| !matches!(e, gallery::Exhibit::Fairy | gallery::Exhibit::Statue(_))) => self.sfx(ctx, "error"),
                None => {}
            }
        }
        State::Gallery { wing, from_title }
    }

    /// Choix du portrait : A l'enregistre (creation du profil : le profil n'existe qu'a partir de la).
    fn update_avatars(&mut self, ctx: &mut Ctx, nav: &NavIn, mut sel: usize, setup: bool) -> State {
        match players::avatar_input(nav, ctx.view, &mut sel) {
            players::AvatarAction::Moved => self.sfx(ctx, "cursor"),
            players::AvatarAction::Back if setup => {
                self.sfx(ctx, "confirm");
                return State::NameEntry { purpose: NamePurpose::Setup, text: self.profile.name.clone(), sel: players::DONE };
            }
            players::AvatarAction::Back => {
                self.sfx(ctx, "confirm");
                return State::Players { sel: self.profile.slot, armed: false };
            }
            players::AvatarAction::Pick => {
                self.profile.avatar = sel;
                if self.profile.pseudo.is_empty() {
                    self.profile.pseudo = self.profile.name.clone();
                }
                self.profile.save(&ctx.root);
                profiles::set_active(&ctx.root, self.profile.slot);
                self.slots = profiles::all_slots(&ctx.root);
                if setup {
                    self.sfx(ctx, "relic");
                    let s = self.lang.get("ui.player_created").replace("{s}", &self.profile.name);
                    self.hub_toast = (s, 2.5);
                    return State::Title { sel: 0 };
                }
                self.sfx(ctx, "equip");
                return State::Players { sel: self.profile.slot, armed: false };
            }
            players::AvatarAction::None => {}
        }
        State::Avatars { sel, setup }
    }

    fn name_done(&mut self, ctx: &mut Ctx, purpose: NamePurpose, name: String) -> State {
        let root = ctx.root.clone();
        match purpose {
            NamePurpose::Create(k) => {
                self.profile.save(&root);
                let p = Profile::fresh(k, &name, self.free_avatar(k));
                p.save(&root);
                self.profile = p;
                profiles::set_active(&root, k);
                chars::apply_skins(&mut self.content, &self.profile, &self.meta);
                self.slots = profiles::all_slots(&root);
                self.sfx(ctx, "relic");
                let s = self.lang.get("ui.player_created").replace("{s}", &name);
                self.hub_toast = (s, 2.5);
                State::Players { sel: k, armed: false }
            }
            NamePurpose::Rename => {
                self.profile.name = name;
                self.profile.save(&root);
                self.slots = profiles::all_slots(&root);
                self.sfx(ctx, "equip");
                State::Avatars { sel: self.profile.avatar, setup: false }
            }
            NamePurpose::Setup => {
                self.profile.name = name;
                self.sfx(ctx, "confirm");
                State::Avatars { sel: self.profile.avatar, setup: true }
            }
            NamePurpose::Room => {
                self.sfx(ctx, "confirm");
                self.coop_open(&name);
                State::Lobby { sel: 0 }
            }
            NamePurpose::Pseudo { rec, stage, sel } => {
                self.profile.pseudo = name.clone();
                self.profile.save(&root);
                let map = self.stage_ids.get(stage).cloned().unwrap_or_default();
                self.publish(ctx, map, rec, name);
                State::Scores { tab: scores::MINE, sel, stage, version: String::new(), players: 1 }
            }
        }
    }

    /// Publie une partie (`name` : pseudo ; coop : les pseudos de l'equipe sont dans la partie).
    fn publish(&mut self, ctx: &mut Ctx, map: String, rec: ScoreRecord, name: String) {
        self.online.submit(Submission {
            map: map.clone(),
            version: rec.version.clone(),
            name,
            score: rec.score,
            time: rec.time,
            kills: rec.kills,
            level: rec.level,
            bosses: rec.bosses,
            won: rec.won,
            character: rec.character.clone(),
            difficulty: rec.difficulty,
            mult: rec.mult,
            kw: rec.kw,
            title: if rec.team.is_empty() { self.profile.title } else { 0 },
            players: rec.team.len().max(1),
            team: rec.team.clone(),
        });
        self.publishing = Some((map, rec));
        self.sfx(ctx, "confirm");
        self.hub_toast = (self.lang.get("ui.publishing").to_string(), 30.0);
    }

    /// Reponse du serveur a une publication.
    fn poll_publish(&mut self, ctx: &mut Ctx) {
        if self.publishing.is_none() {
            return;
        }
        let (ok, msg) = match self.online.submit_status() {
            NetStatus::Done(rep) => {
                let msg = match rep.rank {
                    Some(n) => self.lang.get("ui.published_rank").replace("{n}", &n.to_string()),
                    None => self.lang.get("ui.published_ok").to_string(),
                };
                (true, msg)
            }
            NetStatus::Failed(e) => {
                let key = match e.as_str() {
                    "bad_name" => "ui.net.bad_name",
                    "implausible" => "ui.net.implausible",
                    "rate_limited" => "ui.net.rate_limited",
                    "duplicate" => "ui.already_published",
                    "old_version" => "ui.net.old_version",
                    _ => "ui.net.offline",
                };
                log::info!("publication refusee : {e}");
                (e == "duplicate", self.lang.get(key).to_string())
            }
            _ => return,
        };
        if let Some((map, rec)) = self.publishing.take() {
            if ok {
                let found = self.profile.score_list_mut(&map, rec.team.len().max(1)).and_then(|l| l.iter_mut().find(|r| r.score == rec.score && r.time == rec.time && r.kills == rec.kills));
                if let Some(r) = found {
                    r.published = true;
                }
                self.profile.save(&ctx.root);
                self.online.refresh();
            }
        }
        self.online.clear_submit();
        self.sfx(ctx, if ok { "reward" } else { "error" });
        self.hub_toast = (msg, 4.0);
    }

    /// Lignes de l'ecran Classements.
    fn score_rows(&self, ctx: &Ctx, tab: usize, stage: usize, version: &str, players: usize) -> (Vec<hub::Row>, Vec<String>) {
        let lang = &self.lang;
        let stage_id = self.stage_ids.get(stage).map(|s| s.as_str()).unwrap_or("");
        let board = self.online.board();
        let diffs: Vec<String> = self.meta.difficulties.iter().map(|d| lang.get(&format!("difficulty.{}.name", d.id)).to_string()).collect();
        let titles: Vec<String> = self.meta.titles.iter().map(|t| lang.get(&format!("title.{}", t.id)).to_string()).collect();
        let v = scores::View {
            lang,
            content: &self.content,
            atlas: &ctx.atlas,
            stage: stage_id,
            profile: &self.profile,
            slots: &self.slots,
            difficulties: &diffs,
            titles: &titles,
            board: &board,
            version,
            players: if scores::has_mode(tab) { players } else { 1 },
        };
        scores::rows(&v, tab)
    }

    /// Version voisine dans le classement mondial (vide : la plus recente).
    fn step_version(&self, stage_id: &str, version: &str, d: i32) -> String {
        let ((map, _, _), st) = self.online.board();
        let NetStatus::Done(b) = st else { return version.to_string() };
        if map != stage_id || b.versions.is_empty() {
            return version.to_string();
        }
        let cur = if version.is_empty() { b.version.clone() } else { version.to_string() };
        let n = b.versions.len() as i32;
        let k = b.versions.iter().position(|v| *v == cur).unwrap_or(0) as i32;
        let next = (k + d).rem_euclid(n) as usize;
        if next == 0 {
            String::new()
        } else {
            b.versions[next].clone()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_scores(&mut self, ctx: &mut Ctx, nav: &NavIn, (mut tab, mut sel, mut stage): (usize, usize, usize), mut version: String, mut players: usize) -> State {
        let ns = self.stage_ids.len().max(1);
        stage = stage.min(ns - 1);
        let stage_id = self.stage_ids.get(stage).cloned().unwrap_or_default();
        if tab == scores::WORLD {
            self.online.fetch(&stage_id, &version, players);
        }
        let n = self.score_rows(ctx, tab, stage, &version, players).0.len();
        // solo, puis coop a 2, 3 ou 4
        let step_players = |p: usize, d: i32| (p as i32 - 1 + d).rem_euclid(scores::MAX_PLAYERS as i32) as usize + 1;
        let input = hub::input(nav, ctx.view, scores::TABS.len(), tab, n, &mut sel);
        let head = scores::head(tab);
        match input {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                return State::Title { sel: TITLE_SCORES };
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                if k != tab {
                    tab = k;
                    if sel != hub::TABS_ROW {
                        sel = 0;
                    }
                    self.sfx(ctx, "cursor");
                }
            }
            hub::Input::Left(k) | hub::Input::Right(k) => {
                let d = if matches!(input, hub::Input::Left(_)) { -1 } else { 1 };
                if k == scores::STAGE_ROW {
                    stage = (stage as i32 + d).rem_euclid(ns as i32) as usize;
                } else if scores::has_mode(tab) && k == scores::MODE_ROW {
                    players = step_players(players, d);
                } else if tab == scores::WORLD && k == scores::VERSION_ROW {
                    version = self.step_version(&stage_id, &version, d);
                } else {
                    tab = (tab as i32 + d).rem_euclid(scores::TABS.len() as i32) as usize;
                    sel = 0;
                }
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if k == scores::STAGE_ROW => {
                stage = (stage + 1) % ns;
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if scores::has_mode(tab) && k == scores::MODE_ROW => {
                players = step_players(players, 1);
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if tab == scores::WORLD && k == scores::VERSION_ROW => {
                version = self.step_version(&stage_id, &version, 1);
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if tab == scores::MINE => {
                let rec = self.profile.score_list(&stage_id, players).get(k.wrapping_sub(head)).cloned();
                match rec {
                    Some(r) if r.published => {
                        self.sfx(ctx, "error");
                        self.toast("ui.already_published");
                    }
                    Some(_) if self.publishing.is_some() => {
                        self.sfx(ctx, "error");
                        self.toast("ui.publishing");
                    }
                    // coop : publiee sous les pseudos de l'equipe (ceux de la partie)
                    Some(r) if !r.team.is_empty() => {
                        let name = r.team[0].clone();
                        self.publish(ctx, stage_id.clone(), r, name);
                    }
                    Some(r) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        let text = if self.profile.pseudo.is_empty() { self.profile.name.clone() } else { self.profile.pseudo.clone() };
                        return State::NameEntry { purpose: NamePurpose::Pseudo { rec: r, stage, sel }, text, sel: players::DONE };
                    }
                    None => {}
                }
            }
            hub::Input::Pick(_) | hub::Input::None => {}
        }
        State::Scores { tab, sel, stage, version, players }
    }

    fn draw_hub(&self, ctx: &Ctx, list: &mut DrawList) {
        let lang = &self.lang;
        let (title, tabs, tab, sel, rows, details, corner) = match self.state {
            State::Shop { tab, sel } => {
                let (r, mut d) = shop::rows(&self.meta, &self.content, &self.profile, lang, &ctx.atlas, tab);
                // juke-box : musique ecoutee en extrait, le second appui l'achete
                if let (shop::TAB_JUKEBOX, Some((k, _))) = (tab, self.jukebox_try) {
                    if let (Some(line), Some(t)) = (d.get_mut(k), self.meta.jukebox.get(k)) {
                        *line = lang.get("ui.jukebox_confirm").replace("{n}", &t.cost.to_string());
                    }
                }
                let corner = Some((self.vis.icon_rupees, self.profile.rupees.to_string()));
                (lang.get("ui.shop"), &shop::TABS[shop::SHOP], tab - shop::SHOP.start, sel, r, d, corner)
            }
            State::Prep { tab, sel } => {
                let (mut r, mut d) = shop::rows(&self.meta, &self.content, &self.profile, lang, &ctx.atlas, tab);
                let stage = stages::stage_name(lang, &self.opts.map);
                let who = chars::name(lang, &self.opts.character);
                r.insert(0, hub::Row { label: lang.get("ui.prep_go").to_string(), label_color: Some(hub::GREEN), value: stage.clone(), value_color: Some(GOLD), ..Default::default() });
                d.insert(0, lang.get("ui.prep_go_desc").replace("{c}", &crate::hud::hl(&who)).replace("{s}", &crate::hud::hl(&stage)));
                let corner = Some((self.vis.icon_rupees, self.profile.rupees.to_string()));
                (lang.get("ui.prep"), &shop::TABS[shop::PREP], tab - shop::PREP.start, sel, r, d, corner)
            }
            State::Records { tab, sel, armed } => {
                let (r, d) = records::rows(&self.content, &self.meta, &ctx.atlas, &self.vis, &self.profile, lang, tab, armed);
                let (range, title) = records::book(tab);
                (lang.get(title), &records::TABS[range.clone()], tab - range.start, sel, r, d, None)
            }
            State::Scores { tab, sel, stage, ref version, players } => {
                let (r, d) = self.score_rows(ctx, tab, stage, version, players);
                (lang.get("ui.scores"), &scores::TABS[..], tab, sel, r, d, None)
            }
            State::Controls { tab, sel, wait, .. } => {
                let (r, d) = controls::rows(&ctx.input, lang, &self.bindings, tab, wait.map(|w| w.0));
                (lang.get("ui.controls"), &controls::TABS[..], tab, sel, r, d, None)
            }
            _ => return,
        };
        let tabs: Vec<String> = tabs.iter().map(|k| lang.get(k).to_string()).collect();
        let detail = if self.hub_toast.1 > 0.0 {
            self.hub_toast.0.as_str()
        } else {
            details.get(sel).map(|s| s.as_str()).unwrap_or("")
        };
        hub::draw(list, ctx, title, corner, &tabs, tab, &rows, sel, detail, lang.get("ui.back"));
        if let (State::Records { tab: 0, .. }, Some((k, t0))) = (&self.state, self.beast) {
            self.draw_beast(ctx, list, k, (ctx.real_time - t0) as f32);
        }
        if let (State::Records { .. }, Some((ct, k, t0))) = (&self.state, self.card) {
            self.draw_card(ctx, list, ct, k, (ctx.real_time - t0) as f32);
        }
    }

    /// Fiche d'une arme, d'une fusion, d'un objet ou d'un trophee : icone, description, caracteristiques
    /// au niveau 1 et au maximum (ou bonus par niveau, ou paliers), effets et fusions.
    fn draw_card(&self, ctx: &Ctx, list: &mut DrawList, tab: usize, k: usize, t: f32) {
        use crate::card::Line;
        use engine::draw::Align;
        let atlas = &ctx.atlas;
        let (c, icon) = match tab {
            records::TAB_ITEMS => (crate::card::passive(&self.content, &self.lang, &self.profile, k), Some(self.vis.passives[k])),
            records::TAB_TROPHIES => {
                let icon = crate::trophies::families(&self.content, &self.meta, &self.profile).get(k).and_then(|f| atlas.find(f.icon, "0"));
                (crate::card::trophy(&self.content, &self.meta, &self.lang, &self.profile, k), icon.or_else(|| atlas.find("icon_crystal", "0")))
            }
            _ => (crate::card::weapon(&self.content, &self.lang, &self.profile, k), Some(self.vis.weapons[k].icon)),
        };
        let view = ctx.view;
        let main = atlas.font("main");
        let white = &self.vis.white;
        let lh = main.line_height as f32 + 1.0;
        let width = (view.x - 16.0).min(420.0).round();
        let inner = width - 24.0;
        let desc = crate::hud::wrap(main, &c.desc, inner, 1.0);
        let notes: Vec<String> = c.notes.iter().flat_map(|n| crate::hud::wrap(main, n, inner, 1.0)).collect();
        let stats = c.lines.iter().any(|l| matches!(l, Line::Stat(_, _, b) if !b.is_empty()));
        let rows = c.lines.len() + stats as usize;
        let hint = self.lang.get("card.hint");
        let height = (12.0 + 34.0 + lh + 4.0 + desc.len() as f32 * lh + 6.0 + rows as f32 * lh + 6.0 + notes.len() as f32 * lh + 4.0 + lh + 8.0).min(view.y - 8.0).round();
        let size = Vec2::new(width, height);
        let pos = ((view - size) * 0.5).round();
        let u = (t / 0.15).min(1.0);
        list.set(Space::Ui, Blend::Alpha);
        list.probe_layer();
        list.rect(white, Vec2::ZERO, view, [0.0, 0.0, 0.0, 0.85 * u]);
        list.probe_panel(pos, size, "fiche");
        list.rect(white, pos, size, [0.05, 0.035, 0.09, 0.97]);
        let edge = if c.fusion { [0.75, 0.55, 1.0, 0.9] } else { with_alpha(GOLD, 0.9) };
        crate::hud::frame_rect(list, white, pos, size, edge);
        crate::hud::frame_rect(list, white, pos + Vec2::splat(2.0), size - Vec2::splat(4.0), [1.0, 1.0, 1.0, 0.15]);
        let cx = (view.x * 0.5).round();
        // icone agrandie, qui flotte doucement
        if let Some(icon) = icon {
            let f = atlas.frame(icon, 0.0);
            let sc = (32.0 / f.w.max(f.h).max(1) as f32).floor().clamp(1.0, 3.0);
            let bob = (t * 2.0).sin() * 1.5;
            let at = Vec2::new(cx, pos.y + 8.0 + 16.0 + bob) + Vec2::new(f.px as f32 - f.w as f32 * 0.5, f.py as f32 - f.h as f32 * 0.5) * sc;
            list.frame(f, at.round(), &Params::default().scale(Vec2::splat(sc)));
        }
        let mut y = pos.y + 12.0 + 34.0;
        let title = if c.fusion { crate::hud::hl_fusion(&c.title) } else { c.title.clone() };
        crate::hud::text(list, main, &title, Vec2::new(cx, y), 1.0, GOLD, Align::Center);
        y += lh + 4.0;
        let left = pos.x + 12.0;
        let right = pos.x + width - 12.0;
        let col = (pos.x + width * 0.7).round();
        for l in &desc {
            crate::hud::text(list, main, l, Vec2::new(left, y), 1.0, [0.86, 0.86, 0.94, 1.0], Align::Left);
            y += lh;
        }
        y += 6.0;
        if stats {
            let dim = [0.6, 0.6, 0.72, 1.0];
            crate::hud::text(list, main, self.lang.get("card.lv_first"), Vec2::new(col, y), 1.0, dim, Align::Right);
            crate::hud::text(list, main, self.lang.get("card.lv_max"), Vec2::new(right, y), 1.0, dim, Align::Right);
            y += lh;
        }
        for (i, l) in c.lines.iter().enumerate() {
            // une ligne sur deux legerement eclairee (lecture du tableau)
            if i % 2 == 0 {
                list.rect(white, Vec2::new(left - 4.0, y - 1.0), Vec2::new(inner + 8.0, lh), [1.0, 1.0, 1.0, 0.04]);
            }
            match l {
                Line::Stat(name, a, b) if b.is_empty() => {
                    crate::hud::text(list, main, name, Vec2::new(left, y), 1.0, [0.82, 0.82, 0.92, 1.0], Align::Left);
                    crate::hud::text(list, main, a, Vec2::new(right, y), 1.0, [0.95, 0.95, 1.0, 1.0], Align::Right);
                }
                Line::Stat(name, a, b) => {
                    crate::hud::text(list, main, name, Vec2::new(left, y), 1.0, [0.82, 0.82, 0.92, 1.0], Align::Left);
                    crate::hud::text(list, main, a, Vec2::new(col, y), 1.0, [0.95, 0.95, 1.0, 1.0], Align::Right);
                    crate::hud::text(list, main, b, Vec2::new(right, y), 1.0, if a == b { [0.95, 0.95, 1.0, 1.0] } else { crate::hub::GREEN }, Align::Right);
                }
                Line::Text(name, v) => {
                    crate::hud::text(list, main, name, Vec2::new(left, y), 1.0, [0.82, 0.82, 0.92, 1.0], Align::Left);
                    crate::hud::text(list, main, v, Vec2::new(right, y), 1.0, [0.95, 0.85, 0.55, 1.0], Align::Right);
                }
            }
            y += lh;
        }
        y += 6.0;
        for l in &notes {
            crate::hud::text(list, main, l, Vec2::new(left, y), 1.0, [0.8, 0.9, 1.0, 1.0], Align::Left);
            y += lh;
        }
        crate::hud::text(list, main, hint, Vec2::new(cx, pos.y + size.y - lh - 6.0), 1.0, [0.6, 0.6, 0.72, 1.0], Align::Center);
    }

    /// Fiche d'un ennemi du bestiaire : le sprite a sa taille reelle en partie (echelle comprise),
    /// anime, qui se tourne dans les quatre directions, dans un cadre.
    fn draw_beast(&self, ctx: &Ctx, list: &mut DrawList, k: usize, t: f32) {
        use sim::horde::{FACE_LEFT, FACE_RIGHT, FACE_UP};
        let atlas = &ctx.atlas;
        let view = ctx.view;
        let main = atlas.font("main");
        let white = &self.vis.white;
        let e = &self.content.enemies[k];
        let sc = e.scale.max(1.0);
        let face = [FACE_DOWN, FACE_RIGHT, FACE_UP, FACE_LEFT][((t / 1.6) as usize) % 4];
        let (anim, flip) = self.vis.enemies[k].pick(face, 0.0, false);
        let f = atlas.frame(anim, t);
        // cadre fixe : la plus grande image de l'ennemi (toutes directions, toute l'animation)
        let (mut fw, mut fh) = (0u32, 0u32);
        for d in [FACE_DOWN, FACE_RIGHT, FACE_UP, FACE_LEFT] {
            let a = self.vis.enemies[k].pick(d, 0.0, false).0;
            for i in 0..12 {
                let g = atlas.frame(a, i as f32 * 0.09);
                fw = fw.max(g.w as u32);
                fh = fh.max(g.h as u32);
            }
        }
        let (w, h) = (fw as f32 * sc, fh as f32 * sc);
        let name = self.lang.get(&format!("enemy.{}", self.content.enemy_ids[k])).to_string();
        let stats = format!(
            "{} {}  {} {}  {} {}",
            self.lang.get("ui.beast_hp"),
            e.hp.round(),
            self.lang.get("ui.beast_damage"),
            (e.damage / 10.0 * 2.0).round() / 2.0,
            self.lang.get("ui.beast_speed"),
            e.speed.round()
        );
        let id = &self.content.enemy_ids[k];
        let owned = self.profile.figurines.contains(id);
        // figurine a acheter : A achete, la ligne d'aide ne dit plus « A : fermer »
        let hint = self.lang.get(if owned { "ui.beast_hint" } else { "ui.beast_hint_nav" });
        let fig = if owned {
            self.lang.get("ui.figurine_owned").to_string()
        } else {
            let cost = self.meta.figurine_cost(e, id);
            self.lang.get("ui.figurine_buy").replace("{n}", &cost.to_string())
        };
        // comportement et particularites (enemies.ron), une ligne chacun
        let traits = crate::card::enemy_traits(&self.content, &self.lang, k);
        let extra = traits.len() as f32 * 12.0;
        let tw = [name.as_str(), stats.as_str(), hint, fig.as_str()].into_iter().chain(traits.iter().map(String::as_str)).map(|s| main.measure(s) as f32).fold(0.0, f32::max);
        let size = Vec2::new((w + 48.0).max(200.0).max(tw + 24.0), (h + 98.0 + extra).max(144.0 + extra)).min(view - Vec2::splat(12.0)).round();
        let pos = ((view - size) * 0.5).round();
        let u = (t / 0.15).min(1.0);
        list.set(Space::Ui, Blend::Alpha);
        list.probe_layer();
        list.rect(white, Vec2::ZERO, view, [0.0, 0.0, 0.0, 0.85 * u]);
        list.probe_panel(pos, size, "fiche");
        list.rect(white, pos, size, [0.05, 0.035, 0.09, 0.97]);
        crate::hud::frame_rect(list, white, pos, size, with_alpha(GOLD, 0.9));
        crate::hud::frame_rect(list, white, pos + Vec2::splat(2.0), size - Vec2::splat(4.0), [1.0, 1.0, 1.0, 0.15]);
        let cx = (view.x * 0.5).round();
        crate::hud::text(list, main, &name, Vec2::new(cx, pos.y + 8.0), 1.0, if e.boss { rgb(255, 170, 170) } else { GOLD }, engine::draw::Align::Center);
        // sol : ombre, puis le sprite pose dessus ; figurine achetee : socle dore qui scintille
        let floor = (pos.y + 26.0 + h + ((size.y - 98.0 - extra - h) * 0.5).max(0.0)).round();
        if owned {
            let pw = (w * 0.8 + 16.0).round();
            list.rect(white, Vec2::new(cx - pw * 0.5, floor - 2.0).round(), Vec2::new(pw, 5.0), rgb(240, 200, 80));
            list.rect(white, Vec2::new(cx - pw * 0.5 + 2.0, floor + 3.0).round(), Vec2::new(pw - 4.0, 4.0), rgb(170, 110, 30));
            list.rect(white, Vec2::new(cx - pw * 0.5, floor - 2.0).round(), Vec2::new(pw, 1.0), rgb(255, 240, 170));
            list.set(Space::Ui, Blend::Add);
            let rt = ctx.real_time as f32;
            for s in 0..6 {
                let ph = (rt * 0.5 + s as f32 / 6.0).fract();
                let x = cx + ((s as f32 * 2.1).sin() * pw * 0.45).round();
                list.rect(white, Vec2::new(x, floor - 4.0 - ph * (h + 10.0)).round(), Vec2::splat(1.0 + (s % 2) as f32), with_alpha(GOLD, 1.0 - ph));
            }
            // achat : eclat dore qui retombe
            let since = (ctx.real_time - self.beast_bought) as f32;
            if since < 0.6 {
                list.rect(white, pos, size, with_alpha(GOLD, 0.35 * (1.0 - since / 0.6)));
            }
            list.set(Space::Ui, Blend::Alpha);
        } else {
            let sh = atlas.frame(self.vis.shadow, 0.0);
            list.frame(sh, Vec2::new(cx, floor), &Params::default().scale(Vec2::splat(sc)).color([1.0, 1.0, 1.0, 0.55]));
        }
        list.frame(f, Vec2::new(cx, floor), &Params::default().scale(Vec2::splat(sc)).flip(flip));
        for (i, s) in traits.iter().enumerate() {
            let y = pos.y + size.y - 46.0 - extra + i as f32 * 12.0;
            crate::hud::text(list, main, s, Vec2::new(cx, y), 1.0, [0.8, 0.9, 1.0, 1.0], engine::draw::Align::Center);
        }
        crate::hud::text(list, main, &stats, Vec2::new(cx, pos.y + size.y - 46.0), 1.0, [0.82, 0.82, 0.92, 1.0], engine::draw::Align::Center);
        crate::hud::text(list, main, &fig, Vec2::new(cx, pos.y + size.y - 32.0), 1.0, if owned { GOLD } else { [0.95, 0.85, 0.55, 1.0] }, engine::draw::Align::Center);
        crate::hud::text(list, main, hint, Vec2::new(cx, pos.y + size.y - 18.0), 1.0, [0.6, 0.6, 0.72, 1.0], engine::draw::Align::Center);
    }

    fn update_menus(&mut self, ctx: &mut Ctx, nav: &NavIn, dt: f32) {
        let view = ctx.view;
        self.hub_toast.1 = (self.hub_toast.1 - dt).max(0.0);
        let touch_pause = self.touch.update(&ctx.input, view, ctx.scale as f32, matches!(self.state, State::Playing));
        let pause = ctx.input.action_pressed(&self.bindings, "pause") || touch_pause;
        self.poll_publish(ctx);
        self.tick_preview(ctx);
        let state = std::mem::replace(&mut self.state, State::Playing);
        self.state = match state {
            State::Title { mut sel } => {
                let items = self.title_items();
                let (mut pick, moved) = title::input_title(nav, view, items.len(), &mut sel);
                if self.menu_t < 0.5 {
                    // pendant le fondu d'ouverture : pas de choix par un appui reste enfonce
                    pick = None;
                }
                if pick.is_some() {
                    log::info!("ecran titre : choix {:?} (confirmer {}, clic {})", pick, nav.confirm, nav.click);
                }
                if moved {
                    self.sfx(ctx, "cursor");
                }
                match pick.map(|k| items[k].0) {
                    Some(TitleItem::Play) => {
                        self.sfx(ctx, "confirm");
                        self.char_select()
                    }
                    // salle deja rejointe : retour au salon
                    Some(TitleItem::Coop) => {
                        self.sfx(ctx, "confirm");
                        if self.coop.is_some() {
                            State::Lobby { sel: 0 }
                        } else {
                            State::Coop { sel: 0 }
                        }
                    }
                    Some(TitleItem::Players) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        self.slots = profiles::all_slots(&ctx.root);
                        State::Players { sel: self.profile.slot, armed: false }
                    }
                    Some(TitleItem::Shop) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        State::Shop { tab: 0, sel: hub::TABS_ROW }
                    }
                    Some(TitleItem::Scores) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        self.slots = profiles::all_slots(&ctx.root);
                        let stage = self.stage_ids.iter().position(|s| *s == self.world.map_id).unwrap_or(0);
                        State::Scores { tab: scores::MINE, sel: hub::TABS_ROW, stage, version: String::new(), players: 1 }
                    }
                    Some(TitleItem::Collection) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        State::Records { tab: 0, sel: hub::TABS_ROW, armed: false }
                    }
                    Some(TitleItem::Gallery) => {
                        self.sfx(ctx, "confirm");
                        self.gallery = Some(gallery::Room::new(&self.content, &self.meta, &self.profile, 0));
                        self.play_music(ctx, GALLERY_MUSIC, 0.6);
                        State::Gallery { wing: 0, from_title: true }
                    }
                    Some(TitleItem::Exploits) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        State::Records { tab: records::TAB_TROPHIES, sel: hub::TABS_ROW, armed: false }
                    }
                    Some(TitleItem::Options) => {
                        self.sfx(ctx, "confirm");
                        self.settings.fullscreen = ctx.fullscreen;
                        State::Options { sel: 0, from_title: true }
                    }
                    Some(TitleItem::About) => {
                        self.sfx(ctx, "confirm");
                        State::About { sel: 0, toast: 0.0 }
                    }
                    Some(TitleItem::Update) => {
                        match self.updater.status() {
                            UpdateStatus::Available(r) => {
                                self.sfx(ctx, "confirm");
                                self.updater.download(r);
                            }
                            UpdateStatus::Ready(_) => {
                                // l'installateur remplace le jeu puis le relance
                                self.settings.save(&ctx.root);
                                if update::launch_pending(&ctx.root) {
                                    ctx.quit();
                                } else {
                                    self.sfx(ctx, "error");
                                }
                            }
                            UpdateStatus::Failed(_) => {
                                let ok = Links::open(&self.links.releases);
                                self.sfx(ctx, if ok { "confirm" } else { "error" });
                            }
                            _ => self.sfx(ctx, "error"),
                        }
                        State::Title { sel }
                    }
                    Some(TitleItem::Quit) => {
                        ctx.quit();
                        State::Title { sel }
                    }
                    None => State::Title { sel },
                }
            }
            State::CharSelect { mut page, mut sel } => {
                let n0 = chars::page_roster(&self.content, page).len();
                let was_tabs = sel == n0 + 1;
                let (pick, moved, turn) = if self.char_lock {
                    // choix de la couleur : la grille ne suit plus la souris ; clic sur le personnage ou le
                    // portrait = valider, sur un autre personnage = le choisir
                    let mut pick = nav.confirm.then_some(sel);
                    if nav.click {
                        if let Some(k) = chars::lock_click(view, n0, self.char_top, sel, nav.mouse) {
                            if k != sel {
                                self.char_lock = false;
                                self.skin_browse = None;
                                sel = k;
                            }
                            pick = Some(k);
                        }
                    }
                    (pick, false, 0)
                } else {
                    chars::input_select(nav, view, n0, &mut sel, &mut self.char_top)
                };
                if moved {
                    self.sfx(ctx, "cursor");
                }
                // gachettes, Q / E : univers precedent / suivant, comme les onglets des autres menus
                let trigger = if nav.tab_next { 1 } else if nav.tab_prev { -1 } else { 0 };
                let turn = if trigger != 0 { trigger } else { turn };
                if turn != 0 {
                    // autre univers : le curseur reste sur le choix de l'univers, ou revient au premier personnage
                    page = (page as i32 + turn).rem_euclid(chars::GROUPS.len() as i32) as usize;
                    let n = chars::page_roster(&self.content, page).len();
                    sel = if trigger != 0 && !was_tabs && n > 0 { 0 } else { n + 1 };
                    self.char_top = 0;
                    self.char_lock = false;
                    self.sfx(ctx, "cursor");
                }
                let ids = chars::page_roster(&self.content, page);
                let n = ids.len();
                // autre personnage : la couleur regardee (pas achetee) est oubliee
                if moved || turn != 0 {
                    self.skin_browse = None;
                }
                // R, Y (ou clic sur une fleche du bouton des couleurs) : couleur du personnage ; pendant le
                // choix de la couleur, aussi gauche / droite
                let mut side = if nav.skin { 1 } else { 0 };
                if self.char_lock {
                    side = if nav.left { -1 } else if nav.right { 1 } else { side };
                }
                if nav.click {
                    if let Some((bp, bs)) = chars::skin_button(ctx, &self.content, &self.profile, &ids, sel) {
                        let m = nav.mouse;
                        if m.x >= bp.x && m.y >= bp.y && m.x < bp.x + bs.x && m.y < bp.y + bs.y {
                            side = if m.x < bp.x + bs.x * 0.5 { -1 } else { 1 };
                        }
                    }
                }
                if side != 0 && sel < n {
                    let id = ids[sel].clone();
                    let def = &self.content.characters[&id];
                    // toutes les couleurs defilent ; une couleur achetee est portee aussitot, une autre
                    // est seulement montree (grisee, avec son prix : A l'achete)
                    let m = def.skins.len();
                    if m > 1 && chars::unlocked(&self.content, def, &self.profile) {
                        let browse = self.skin_browse.as_ref().filter(|b| b.0 == id).map(|b| b.1);
                        let cur = chars::shown_skin(def, &self.profile, &id, &self.meta, browse);
                        let k = if side > 0 { (cur + 1) % m } else { (cur + m - 1) % m };
                        if chars::skin_price(&self.meta, &self.profile, &def.skins[k].0).is_some() {
                            self.skin_browse = Some((id, k));
                        } else {
                            self.skin_browse = None;
                            self.profile.skins.insert(id, k);
                            self.profile.save(&ctx.root);
                            chars::apply_skins(&mut self.content, &self.profile, &self.meta);
                        }
                        self.sfx(ctx, "cursor");
                    }
                }
                if nav.back && self.char_lock {
                    // retour a la grille
                    self.sfx(ctx, "confirm");
                    self.char_lock = false;
                    self.skin_browse = None;
                    State::CharSelect { page, sel }
                } else if nav.back || matches!(pick, Some(k) if k >= n) {
                    self.sfx(ctx, "confirm");
                    self.char_lock = false;
                    if self.coop.is_some() {
                        State::Lobby { sel: 0 }
                    } else {
                        State::Title { sel: 0 }
                    }
                } else {
                    match pick {
                        // couleur regardee pas encore achetee : A propose de l'acheter (ou le dit trop chere)
                        Some(k) if self.skin_browse.as_ref().is_some_and(|b| b.0 == ids[k]) && (!nav.click || self.char_lock) => {
                            let skin = self.skin_browse.as_ref().map_or(0, |b| b.1);
                            let def = &self.content.characters[&ids[k]];
                            match chars::skin_price(&self.meta, &self.profile, &def.skins[skin].0) {
                                Some(p) if p > self.profile.rupees => {
                                    self.sfx(ctx, "error");
                                    State::CharSelect { page, sel }
                                }
                                Some(_) => {
                                    self.sfx(ctx, "dialog_open");
                                    State::SkinBuy { page, sel, skin, ans: 0 }
                                }
                                None => {
                                    self.skin_browse = None;
                                    State::CharSelect { page, sel }
                                }
                            }
                        }
                        // personnage a plusieurs couleurs, valide une premiere fois : choix de la couleur
                        Some(k) if !self.char_lock && self.content.characters[&ids[k]].skins.len() > 1 && chars::unlocked(&self.content, &self.content.characters[&ids[k]], &self.profile) => {
                            self.sfx(ctx, "confirm");
                            self.char_lock = true;
                            State::CharSelect { page, sel: k }
                        }
                        Some(k) if chars::unlocked(&self.content, &self.content.characters[&ids[k]], &self.profile) => {
                            self.sfx(ctx, "confirm");
                            self.char_lock = false;
                            self.skin_browse = None;
                            self.opts.character = ids[k].clone();
                            if self.profile.character != ids[k] {
                                self.profile.character = ids[k].clone();
                                self.profile.save(&ctx.root);
                            }
                            let map = self.opts.map.clone();
                            self.set_map(ctx, &map);
                            if self.coop.as_ref().is_some_and(|s| !s.is_host()) {
                                // coop : le stage est choisi par l'hote
                                State::Prep { tab: shop::TAB_RELICS, sel: 0 }
                            } else {
                                State::StageSelect { sel: stages::suggested(&self.stage_ids, &self.profile) }
                            }
                        }
                        Some(_) => {
                            self.sfx(ctx, "error");
                            State::CharSelect { page, sel }
                        }
                        None => State::CharSelect { page, sel },
                    }
                }
            }
            State::SkinBuy { page, sel, skin, mut ans } => {
                let ids = chars::page_roster(&self.content, page);
                let q = fairy_tip::question(ctx, &self.skin_question(&ids[sel], skin));
                let (pick, moved) = q.input(nav, &mut ans);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                match pick {
                    Some(1) => {
                        let id = ids[sel].clone();
                        let sprite = self.content.characters[&id].skins[skin].0.clone();
                        match chars::skin_price(&self.meta, &self.profile, &sprite) {
                            Some(p) if p <= self.profile.rupees => {
                                self.profile.rupees -= p;
                                self.profile.wardrobe.insert(sprite);
                                self.profile.skins.insert(id, skin);
                                self.profile.save(&ctx.root);
                                chars::apply_skins(&mut self.content, &self.profile, &self.meta);
                                self.skin_browse = None;
                                self.sfx(ctx, "buy");
                            }
                            _ => self.sfx(ctx, "error"),
                        }
                        State::CharSelect { page, sel }
                    }
                    Some(_) => {
                        self.sfx(ctx, "dialog_close");
                        State::CharSelect { page, sel }
                    }
                    None => State::SkinBuy { page, sel, skin, ans },
                }
            }
            State::StageSelect { mut sel } => {
                let n = self.stage_ids.len();
                let (pick, moved) = stages::input_select(nav, view, n, &mut sel);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if nav.back {
                    self.sfx(ctx, "confirm");
                    self.char_select()
                } else {
                    match pick {
                        Some(k) if k >= n => {
                            self.sfx(ctx, "confirm");
                            self.char_select()
                        }
                        Some(k) if stages::unlocked(&self.stage_ids, &self.profile, k) => {
                            // Preparatifs : reliques et defis, « Partir ! » deja selectionne
                            self.sfx(ctx, "confirm");
                            let id = self.stage_ids[k].clone();
                            self.set_map(ctx, &id);
                            State::Prep { tab: shop::TAB_RELICS, sel: 0 }
                        }
                        Some(_) => {
                            self.sfx(ctx, "error");
                            State::StageSelect { sel }
                        }
                        None => State::StageSelect { sel },
                    }
                }
            }
            State::Victory { t, mut sel } => {
                let t = t + dt;
                let next = self.next_stage();
                let items = stages::victory_items(next.is_some());
                if t <= stages::VICTORY_LOCK {
                    State::Victory { t, sel }
                } else {
                    let (pick, moved) = stages::input_victory(nav, view, items.len(), &mut sel);
                    if moved {
                        self.sfx(ctx, "cursor");
                    }
                    match pick.map(|k| items[k]) {
                        Some("ui.next_stage") => {
                            self.sfx(ctx, "confirm");
                            if let Some(id) = next {
                                self.set_map(ctx, &id);
                            }
                            self.restart(ctx);
                            return;
                        }
                        Some("ui.stage_select") if self.coop.is_some() => {
                            self.sfx(ctx, "confirm");
                            self.coop_back_to_lobby(ctx);
                            return;
                        }
                        Some("ui.stage_select") => {
                            self.sfx(ctx, "confirm");
                            self.quit_to_title(ctx);
                            let k = stages::suggested(&self.stage_ids, &self.profile);
                            State::StageSelect { sel: k }
                        }
                        Some(_) => {
                            self.sfx(ctx, "confirm");
                            self.coop_close();
                            self.quit_to_title(ctx);
                            return;
                        }
                        None => State::Victory { t, sel },
                    }
                }
            }
            State::Shop { tab, sel } => self.update_shop(ctx, nav, tab, sel, false).unwrap_or(State::Title { sel: TITLE_SHOP }),
            State::Prep { tab, sel } => match self.update_shop(ctx, nav, tab, sel, true) {
                Some(s) => s,
                // partie lancee : l'etat est deja celui de la partie
                None => return,
            },
            State::Records { tab, sel, armed } => self.update_records(ctx, nav, tab, sel, armed),
            State::Players { sel, armed } => self.update_players(ctx, nav, sel, armed),
            State::NameEntry { purpose, text, sel } => self.update_name(ctx, nav, purpose, text, sel),
            State::Avatars { sel, setup } => self.update_avatars(ctx, nav, sel, setup),
            State::Gallery { wing, from_title } => self.update_gallery(ctx, nav, wing, from_title, dt),
            State::Coop { sel } => self.update_coop_menu(ctx, nav, sel),
            State::Lobby { sel } => self.update_lobby(ctx, nav, sel),
            State::Scores { tab, sel, stage, version, players } => self.update_scores(ctx, nav, (tab, sel, stage), version, players),
            State::Controls { tab, sel, wait, from_title } => self.update_controls(ctx, nav, tab, sel, wait, from_title, dt),
            State::About { mut sel, toast } => {
                let toast = (toast - dt).max(0.0);
                let font = ctx.atlas.font("main");
                let (pick, moved) = title::input_about(nav, view, &self.lang, font, &mut sel);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if nav.back {
                    self.sfx(ctx, "confirm");
                    State::Title { sel: TITLE_ABOUT }
                } else {
                    match pick.map(|k| (k, title::about_url(&self.links, k))) {
                        Some((_, None)) => {
                            self.sfx(ctx, "confirm");
                            State::Title { sel: TITLE_ABOUT }
                        }
                        Some((_, Some(url))) => {
                            let ok = Links::open(url);
                            self.sfx(ctx, if ok { "confirm" } else { "error" });
                            State::About { sel, toast: if ok { 2.0 } else { 0.0 } }
                        }
                        None => State::About { sel, toast },
                    }
                }
            }
            State::Playing => {
                let lost_focus = !ctx.input.focused && self.opts.bench.is_none() && !self.opts.demo;
                if pause || lost_focus {
                    self.sfx(ctx, "pause");
                    ctx.audio.duck_music(0.35, 0.25);
                    State::Paused { sel: 0 }
                } else {
                    State::Playing
                }
            }
            State::Paused { mut sel } => {
                let (pick, moved) = menus::input_pause(nav, view, &mut sel, self.fallback.is_some());
                if moved {
                    self.sfx(ctx, "cursor");
                }
                match pick {
                    _ if nav.back => {
                        self.sfx(ctx, "unpause");
                        self.resume(ctx);
                        return;
                    }
                    Some(0) => {
                        self.sfx(ctx, "unpause");
                        self.resume(ctx);
                        return;
                    }
                    Some(1) => {
                        self.sfx(ctx, "confirm");
                        self.settings.fullscreen = ctx.fullscreen;
                        State::Options { sel: 0, from_title: false }
                    }
                    // coop : pas de nouvelle partie seul
                    Some(2) if self.coop_run() => {
                        self.sfx(ctx, "error");
                        State::Paused { sel }
                    }
                    Some(2) => {
                        // personnages debloques pendant la partie : montres avant la nouvelle partie
                        let root = ctx.root.clone();
                        self.commit_run(&root, false);
                        self.collect_unlocks();
                        if self.pending_unlocks.is_empty() {
                            self.restart(ctx);
                        } else {
                            let items = std::mem::take(&mut self.pending_unlocks);
                            self.state = State::Unlocks { items, t: 0.0, next: menus::AfterUnlock::Restart };
                        }
                        return;
                    }
                    Some(3) => {
                        self.sfx(ctx, "confirm");
                        // coop : quitter la partie, c'est quitter la salle
                        self.coop_close();
                        self.quit_to_title(ctx);
                        return;
                    }
                    // bonus de niveau garde : Fee <-> Rubis
                    Some(_) => {
                        self.fallback = Some(if self.fallback == Some(Offer::Heal) { Offer::Rupees } else { Offer::Heal });
                        self.sfx(ctx, "confirm");
                        State::Paused { sel }
                    }
                    None => State::Paused { sel },
                }
            }
            State::Options { mut sel, from_title } => {
                let (act, moved) = menus::input_options(nav, view, &mut sel);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                match act {
                    OptAction::Back => {
                        self.settings.save(&ctx.root);
                        self.sfx(ctx, "confirm");
                        if from_title {
                            State::Title { sel: TITLE_OPTIONS }
                        } else {
                            State::Paused { sel: 1 }
                        }
                    }
                    OptAction::Step(Opt::Controls, _) => {
                        self.sfx(ctx, "confirm");
                        let tab = (ctx.input.last_device == Device::Gamepad) as usize;
                        State::Controls { tab, sel: hub::TABS_ROW, wait: None, from_title }
                    }
                    // sauvegarde (export / import) : depuis l'ecran titre seulement (pas en pleine partie)
                    OptAction::Step(Opt::Save, _) if from_title => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        State::Records { tab: records::TAB_SAVE, sel: hub::TABS_ROW, armed: false }
                    }
                    OptAction::Step(Opt::Save, _) => {
                        self.sfx(ctx, "error");
                        State::Options { sel, from_title }
                    }
                    OptAction::Step(o, d) => {
                        self.change_option(ctx, o, d, None);
                        State::Options { sel, from_title }
                    }
                    OptAction::Set(o, v) => {
                        self.change_option(ctx, o, 0, Some(v));
                        State::Options { sel, from_title }
                    }
                    OptAction::None => State::Options { sel, from_title },
                }
            }
            State::LevelUp { offers, mut sel, t } => {
                let t = t + dt;
                let avail = [self.world.mods.rerolls > 0, self.world.mods.skips > 0];
                let (mut pick, moved) = menus::input_levelup_ex(nav, view, offers.len(), &mut sel, t, avail);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if self.opts.demo && t > 1.2 {
                    pick = Some(menus::LevelPick::Card(0));
                }
                match pick {
                    // Relancer : de nouveaux choix (les cartes rentrent a nouveau)
                    Some(menus::LevelPick::Reroll) => {
                        self.world.mods.rerolls -= 1;
                        self.coop_rerolls += 1;
                        self.sfx(ctx, "reroll");
                        let offers = self.levelup_offers(ctx);
                        let sel = if self.world.mods.rerolls > 0 { sel } else { 0 };
                        State::LevelUp { offers, sel, t: 0.15 }
                    }
                    // Passer : le niveau est consomme, sans rien en echange
                    Some(menus::LevelPick::Skip) => {
                        self.world.mods.skips -= 1;
                        self.take_level(None);
                        self.sfx(ctx, "skip");
                        self.resume(ctx);
                        return;
                    }
                    Some(menus::LevelPick::Card(k)) => {
                        // choix important (fusion possible, ou niveau maximal atteint) : petite fanfare
                        // joyeuse ; choix ordinaire : aucun son (seulement l'eclair)
                        let info = menus::offer_info(&self.world, &self.vis, &self.lang, offers[k]);
                        let big = info.evo.iter().any(|e| e.2) || (info.max > 1 && info.level >= info.max);
                        // fusion choisie : meme mise en scene que dans un coffre
                        let from = match offers[k] {
                            Offer::Evolve(into) => self.world.evolution_choices_all().into_iter().find(|e| e.1 == into).map(|e| (self.world.weapons[e.0].id, into)),
                            _ => None,
                        };
                        self.take_level(Some(offers[k]));
                        if matches!(offers[k], Offer::Heal | Offer::Rupees) {
                            self.fallback = Some(offers[k]);
                        }
                        if let Some(evo) = from {
                            self.state = State::Chest { reward: ChestReward { evolution: Some(evo), upgrades: Vec::new() }, big: false, t: 0.0 };
                            return;
                        }
                        if big {
                            self.sfx(ctx, "pick_big");
                        }
                        self.juice.flash([1.0, 1.0, 1.0, 0.25], 0.12);
                        self.resume(ctx);
                        // niveaux a la suite : l'ecran suivant arrive sans nouveau fondu, attente raccourcie
                        if let State::LevelUp { t, .. } = &mut self.state {
                            *t = 0.15;
                        }
                        return;
                    }
                    None => State::LevelUp { offers, sel, t },
                }
            }
            State::Reward { offers, mut sel, t } => {
                let t = t + dt;
                let (mut pick, moved) = menus::input_levelup(nav, view, offers.len(), &mut sel, t);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                // demo : la carte choisie (la premiere ; --menu reward : la relique)
                if self.opts.demo && t > 1.2 {
                    pick = Some(sel);
                }
                match pick {
                    Some(k) => {
                        if let Some(id) = reward::apply(&mut self.world, &self.meta, offers[k]) {
                            self.run_relics.push(id);
                            self.hud.relics = self.relic_icons(&ctx.atlas);
                        }
                        self.world.pending_rewards = self.world.pending_rewards.saturating_sub(1);
                        self.sfx(ctx, "pick_big");
                        self.juice.flash([1.0, 0.85, 1.0, 0.3], 0.15);
                        self.resume(ctx);
                        return;
                    }
                    None => State::Reward { offers, sel, t },
                }
            }
            State::Fortune { phase, sel, t } => match self.update_fortune(ctx, nav, phase, sel, t + dt, dt) {
                Some(s) => s,
                None => return,
            },
            State::Witch { sel, t } => match self.update_witch(ctx, nav, sel, t + dt) {
                Some(s) => s,
                None => return,
            },
            State::EvoChoice { options, big, mut sel, t } => {
                let t = t + dt;
                let (mut pick, moved) = menus::input_levelup(nav, view, options.len(), &mut sel, t);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if self.opts.demo && t > 1.2 {
                    pick = Some(0);
                }
                match pick {
                    Some(k) => {
                        self.sfx(ctx, "pick_big");
                        let reward = self.take_chest(big, Some(options[k].0));
                        State::Chest { reward, big, t: 0.0 }
                    }
                    None => State::EvoChoice { options, big, sel, t },
                }
            }
            State::Unlocks { mut items, t, next } => {
                let t2 = t + dt;
                // page affichee : les premiers ; les autres attendent la page suivante
                let n = items.len().min(menus::unlocks_per_page(view));
                // pages pleines : un son sur deux ou trois, pas une rafale
                let every = (0.3 / menus::unlock_gap(n)).ceil() as usize;
                for (i, it) in items[..n].iter().enumerate() {
                    let ti = menus::unlock_at(i, n);
                    if t < ti && t2 >= ti && i % every == 0 {
                        self.sfx(ctx, if matches!(it, menus::Unlock::Char(_)) { "char_unlock" } else { "weapon_unlock" });
                    }
                }
                let done = menus::unlocks_done(n);
                let pressed = (nav.confirm || nav.click) && t2 > menus::UNLOCK_LOCK;
                if pressed && t2 < done {
                    // appui pendant l'animation : tout apparait d'un coup (puis courte attente)
                    State::Unlocks { items, t: done, next }
                } else if (pressed && t2 >= done + menus::UNLOCK_SETTLE) || (self.opts.demo && t2 > done + 1.0) {
                    if items.len() > n {
                        self.sfx(ctx, "cursor");
                        items.drain(..n);
                        State::Unlocks { items, t: 0.0, next }
                    } else {
                        self.sfx(ctx, "confirm");
                        match next {
                            menus::AfterUnlock::Victory => State::Victory { t: 0.0, sel: 0 },
                            menus::AfterUnlock::Dead => State::Dead { t: 0.0, sel: 0 },
                            menus::AfterUnlock::Title => State::Title { sel: 0 },
                            menus::AfterUnlock::Restart => {
                                self.restart(ctx);
                                return;
                            }
                        }
                    }
                } else {
                    State::Unlocks { items, t: t2, next }
                }
            }
            State::Chest { reward, big, t } => {
                let t2 = t + dt;
                let evo = reward.evolution;
                if t < menus::CHEST_OPEN && t2 >= menus::CHEST_OPEN {
                    self.sfx(ctx, "chest_open");
                    if evo.is_some() {
                        self.sfx(ctx, "evolution");
                    } else {
                        self.sfx(ctx, "chest_fanfare");
                    }
                }
                if evo.is_some() && t < menus::EVO_MERGE && t2 >= menus::EVO_MERGE {
                    self.sfx(ctx, "evo_merge");
                    self.sfx(ctx, "chest_fanfare");
                }
                let done = if evo.is_some() { menus::EVO_DONE } else { menus::CHEST_DONE };
                let go = t2 >= done && (nav.confirm || nav.click || (self.opts.demo && t2 > done + 0.5));
                if go {
                    self.sfx(ctx, "confirm");
                    if let Some((_, into)) = evo {
                        // retour en jeu : l'arme evoluee eclate autour de Link, sa case brille
                        let pp = self.world.player.pos - Vec2::new(0.0, 8.0);
                        for k in 0..3 {
                            self.juice.ring(pp, 4.0 + k as f32 * 8.0, 60.0 + k as f32 * 30.0, 0.5 + k as f32 * 0.15, if k == 1 { GOLD } else { rgb(255, 130, 240) });
                        }
                        self.juice.debris(pp, Vec2::ZERO, 30, rgb(255, 130, 240), 170.0, true);
                        self.hud.evo_flash = Some((into, 3.0));
                    }
                    self.resume(ctx);
                    return;
                }
                State::Chest { reward, big, t: t2 }
            }
            State::Dying(t) => {
                let t = t + dt;
                if t > 1.8 && !self.pending_unlocks.is_empty() {
                    State::Unlocks { items: std::mem::take(&mut self.pending_unlocks), t: 0.0, next: menus::AfterUnlock::Dead }
                } else if t > 1.8 {
                    State::Dead { t: 0.0, sel: 0 }
                } else {
                    State::Dying(t)
                }
            }
            State::Dead { t, mut sel } => {
                let t = t + dt;
                if t <= menus::DEAD_LOCK {
                    State::Dead { t, sel }
                } else {
                    let (pick, moved) = menus::input_dead(nav, view, &mut sel);
                    if moved {
                        self.sfx(ctx, "cursor");
                    }
                    match pick {
                        // coop : retour au salon, pour la partie suivante
                        Some(0) if self.coop.is_some() => {
                            self.sfx(ctx, "confirm");
                            self.coop_back_to_lobby(ctx);
                            return;
                        }
                        Some(0) => {
                            self.sfx(ctx, "confirm");
                            self.restart(ctx);
                            return;
                        }
                        Some(_) => {
                            self.sfx(ctx, "confirm");
                            self.coop_close();
                            self.quit_to_title(ctx);
                            return;
                        }
                        None => State::Dead { t, sel },
                    }
                }
            }
            State::Fairy { page, t, ask } => match fairy_tip::update(ctx, &self.lang, nav, dt, page, t, ask) {
                fairy_tip::Step::Stay { page, t, ask } => State::Fairy { page, t, ask },
                fairy_tip::Step::Sfx(s, page, t, ask) => {
                    self.sfx(ctx, s);
                    State::Fairy { page, t, ask }
                }
                fairy_tip::Step::Done => {
                    self.sfx(ctx, "fairy");
                    State::Title { sel: TITLE_SHOP }
                }
            },
        };
        // apres deux defaites, une fee vient parler de la Boutique (une fois, l'ecran titre installe)
        if matches!(self.state, State::Title { .. }) && self.menu_t > 0.8 && fairy_tip::due(&self.profile) && !self.opts.uicheck && !self.opts.demo && !self.opts.scripted {
            self.sfx(ctx, "fairy");
            // une seule fois par profil, meme si le jeu est quitte pendant qu'elle parle
            self.profile.fairy_tip = true;
            self.profile.save(&ctx.root);
            self.state = State::Fairy { page: 0, t: 0.0, ask: None };
        }
    }

    /// Meteo, teinte de la carte et porte de sortie (apres le monde, avant l'interface).
    fn draw_stage_fx(&self, ctx: &Ctx, alpha: f32, list: &mut DrawList, cam: Vec2) {
        let w = &self.world;
        let white = self.vis.white;
        if let Some(at) = w.portal {
            weather::draw_portal(list, ctx, &self.weather_vis, &white, at, w.portal_t, ctx.real_time as f32);
        }
        let pp = w.player.prev.lerp(w.player.pos, alpha) - Vec2::new(0.0, 8.0);
        let ui = self.to_ui(pp, cam, ctx.view);
        weather::draw(list, ctx, &self.weather_vis, &white, w.map.weather, w.map.tint, cam, ui, w.time);
        self.draw_drink(ctx, list, ui);
        // fleche vers la porte quand elle est hors champ
        if let Some(at) = w.portal {
            let view = ctx.view;
            let p = self.to_ui(at, cam, view);
            let m = 20.0;
            if p.x < 0.0 || p.y < 0.0 || p.x > view.x || p.y > view.y {
                list.set(Space::Ui, Blend::Alpha);
                let c = Vec2::new(p.x.clamp(m, view.x - m), p.y.clamp(m + 60.0, view.y - m));
                let d = (p - c).normalize_or_zero();
                let pulse = 0.7 + 0.3 * (ctx.real_time as f32 * 5.0).sin();
                list.rect(&white, (c - Vec2::splat(4.0)).round(), Vec2::splat(8.0), [0.55, 1.0, 0.6, 0.8 * pulse]);
                for s in 0..5 {
                    let q = c + d * (8.0 + s as f32);
                    let ww = (5 - s) as f32;
                    list.rect(&white, (q - Vec2::splat(ww * 0.5)).round(), Vec2::splat(ww), [0.75, 1.0, 0.75, 0.95]);
                }
            }
        }
    }

    /// Stage suivant (deja ouvert), s'il existe.
    /// Personnage propose a l'ouverture du choix : le dernier joue, s'il est encore disponible.
    /// Ecran des personnages, sur le personnage du profil (sa page) s'il est disponible.
    fn char_select(&self) -> State {
        let want = if self.profile.character.is_empty() { &self.opts.character } else { &self.profile.character };
        let open = self.content.characters.get(want).is_some_and(|d| chars::unlocked(&self.content, d, &self.profile));
        let (page, sel) = chars::locate(&self.content, want).filter(|_| open).unwrap_or((0, 0));
        State::CharSelect { page, sel }
    }

    fn next_stage(&self) -> Option<String> {
        // coop : le stage suivant se choisit dans le salon
        if self.coop.is_some() {
            return None;
        }
        let k = self.stage_ids.iter().position(|s| *s == self.world.map_id)?;
        let id = self.stage_ids.get(k + 1)?;
        stages::unlocked(&self.stage_ids, &self.profile, k + 1).then(|| id.clone())
    }

    /// Coup d'epee : lame et trainee ; `cv` : visuels du heros, `hand` : sa main.
    fn draw_swing(&self, list: &mut DrawList, atlas: &engine::assets::Atlas, cv: &crate::visuals::CharVis, sw: &sim::world::Swing, hand: Vec2, alpha: f32) {
        let white = self.vis.white;
        let master = self.content.weapons[sw.weapon as usize].evolved;
        let t_r = (sw.t - DT * (1.0 - alpha)).max(0.0);
        let blade = atlas.frame(if master { cv.weapon_master } else { cv.weapon }, 0.0);
        let glow = if master { [0.6, 0.85, 1.0] } else { [1.0, 1.0, 0.9] };
        list.set(Space::World, Blend::Add);
        // croissant de lumiere le long de l'arc deja parcouru
        let a0 = sw.angle_at(0.0);
        let a1 = sw.angle_at(t_r);
        let steps = 18;
        for s in 0..=steps {
            let u = s as f32 / steps as f32;
            let a = a0 + (a1 - a0) * u;
            let fade = u * u;
            for (rr, sz, al) in [(sw.radius * 0.95, 2.0, 0.55), (sw.radius * 0.75, 1.0, 0.35)] {
                let q = hand + Vec2::from_angle(a) * rr;
                list.rect(&white, q.round(), Vec2::splat(sz), [glow[0], glow[1], glow[2], al * fade]);
            }
        }
        for k in 1..=4 {
            let a = sw.angle_at((t_r - k as f32 * 0.014).max(0.0));
            let prm = Params::default().rot(a).color([0.8, 0.9, 1.0, 0.5 - k as f32 * 0.1]);
            list.frame(blade, hand + Vec2::from_angle(a) * 4.0, &prm);
        }
        list.set(Space::World, Blend::Alpha);
        let a = sw.angle_at(t_r);
        let len_scale = sw.radius / 30.0;
        let prm = Params::default().rot(a).scale(Vec2::new(len_scale.max(1.0), 1.0));
        list.frame(blade, hand + Vec2::from_angle(a) * 4.0, &prm);
    }

    fn draw_world(&mut self, ctx: &Ctx, alpha: f32, list: &mut DrawList, cam: Vec2) {
        let atlas = &ctx.atlas;
        let w = &self.world;
        let half = ctx.view * 0.5;
        let lo = cam - half - Vec2::splat(48.0);
        let hi = cam + half + Vec2::splat(64.0);
        let in_view = |p: Vec2| p.x >= lo.x && p.y >= lo.y && p.x <= hi.x && p.y <= hi.y;
        let wt = w.time - DT * (1.0 - alpha);
        let rt = ctx.real_time as f32;

        // sol
        list.set(Space::World, Blend::Alpha);
        let tiles = &self.vis.ground;
        if !tiles.is_empty() {
            let t0 = ((cam - half) / 16.0).floor();
            let t1 = ((cam + half) / 16.0).ceil();
            for ty in t0.y as i32..t1.y as i32 {
                for tx in t0.x as i32..t1.x as i32 {
                    let f = &tiles[stages::ground_index(tx, ty, tiles.len())];
                    list.frame(f, Vec2::new(tx as f32 * 16.0, ty as f32 * 16.0), &Params::default());
                }
            }
        }

        // zones au sol : flammes, annonces des colonnes de feu
        let white = self.vis.white;
        for z in &w.zones {
            if !in_view(z.pos) {
                continue;
            }
            if !z.fired {
                // cercle d'annonce qui se resserre
                let u = (z.age / z.delay.max(0.01)).clamp(0.0, 1.0);
                let r = z.radius * (1.4 - 0.4 * u);
                let segs = 16;
                for k in 0..segs {
                    let a = k as f32 / segs as f32 * TAU + rt * 2.0;
                    let p = z.pos + Vec2::new(a.cos(), a.sin() * 0.5) * r;
                    list.rect(&white, p.round(), Vec2::splat(2.0), [1.0, 0.5, 0.2, 0.4 + 0.5 * u]);
                }
                continue;
            }
            let left = z.delay + z.life - z.age;
            // facon retro : la flamme clignote avant de s'eteindre
            if left >= 0.35 && expiring(left, 1.0) {
                continue;
            }
            let fade = (left / 0.35).clamp(0.0, 1.0);
            let sc = (z.radius / 12.0).clamp(1.0, 2.0).round();
            let anim = if left < 0.35 { self.vis.flame_fade } else { self.vis.flame };
            let t = if left < 0.35 { 0.35 - left } else { wt + z.pos.x * 0.01 };
            let f = atlas.frame(anim, t);
            list.frame(f, z.pos + Vec2::new(0.0, 2.0), &Params::default().scale(Vec2::splat(sc)).color([1.0, 1.0, 1.0, 0.6 + 0.4 * fade]));
        }

        // ombres
        let shadow = atlas.frame(self.vis.shadow, 0.0);
        let sh = Params::default().color([1.0, 1.0, 1.0, 0.55]);
        let h = &w.horde;
        for i in 0..h.len() {
            let p = h.prev[i].lerp(h.pos[i], alpha);
            if !in_view(p) {
                continue;
            }
            let big = self.vis.shadow_size[h.kind[i] as usize];
            let es = self.vis.enemy_scale[h.kind[i] as usize];
            let prm = if big >= 2 { sh.scale(Vec2::new(2.0, 1.0) * es) } else { sh.scale(Vec2::splat(es)) };
            list.frame(shadow, p, &prm);
        }
        let pp = w.player.prev.lerp(w.player.pos, alpha);
        list.frame(shadow, pp, &sh);
        for p in &w.projectiles {
            if p.kind == ProjKind::Lob && p.z > 0.5 {
                let gp = p.prev.lerp(p.pos, alpha) + Vec2::new(0.0, 4.0);
                list.frame(shadow, gp, &sh.scale(Vec2::new(0.75, 1.0)));
            }
        }
        for s in &w.shots {
            if s.lob {
                list.frame(shadow, s.prev.lerp(s.pos, alpha), &sh.scale(Vec2::new(0.75, 1.0)));
            }
        }

        // objets au sol
        let pk = &w.pickups;
        for i in 0..pk.len() {
            let p = pk.prev[i].lerp(pk.pos[i], alpha);
            if !in_view(p) {
                continue;
            }
            let k = pk.kind[i];
            let (anim, t) = match k {
                PickupKind::Heart => (self.vis.heart_small, 0.0),
                PickupKind::Chest => (self.vis.chest[0], 0.0),
                PickupKind::BigChest => (self.vis.chest[1], 0.0),
                PickupKind::Magnet => (self.vis.specials[0], 0.0),
                PickupKind::Fairy => (self.vis.specials[1], wt),
                PickupKind::Medallion => (self.vis.specials[2], 0.0),
                k => (self.vis.rupees[k as usize], wt + i as f32 * 0.37),
            };
            if pk.z[i] > 0.5 {
                list.frame(shadow, p, &sh.scale(Vec2::new(0.5, 1.0)));
            }
            let f = atlas.frame(anim, t);
            let mut pos = p - Vec2::new(0.0, pk.z[i].round());
            if k.is_special() {
                // objet rare : flotte et brille
                pos.y -= (3.0 + 2.0 * (rt * 4.0 + i as f32).sin()).round();
                let glow = 0.35 + 0.3 * (rt * 6.0).sin();
                list.frame(f, pos, &Params::default());
                list.set(Space::World, Blend::Add);
                list.frame(f, pos, &Params::default().color([1.0, 1.0, 0.8, glow]));
                for s in 0..3 {
                    let a = rt * 3.0 + s as f32 * TAU / 3.0;
                    let q = pos - Vec2::new(0.0, f.h as f32 * 0.5) + Vec2::new(a.cos() * 11.0, a.sin() * 5.0);
                    list.rect(&white, q.round(), Vec2::ONE, [1.0, 1.0, 0.7, 0.9]);
                }
                list.set(Space::World, Blend::Alpha);
                continue;
            }
            list.frame(f, pos, &Params::default());
            if k.is_chest() {
                // le coffre scintille pour attirer l'oeil
                let glow = 0.25 + 0.25 * (rt * 5.0).sin();
                list.set(Space::World, Blend::Add);
                list.frame(f, pos, &Params::default().color([1.0, 0.85, 0.3, glow]));
                list.set(Space::World, Blend::Alpha);
            }
        }

        self.draw_npcs(ctx, list, lo, hi);

        // personnages tries par profondeur (y)
        self.sort.clear();
        for i in 0..h.len() {
            let p = h.prev[i].lerp(h.pos[i], alpha);
            if !in_view(p) {
                continue;
            }
            self.sort.push((((p.y + 65536.0) * 16.0) as u32, i as u32));
        }
        self.sort.push((((pp.y + 65536.0) * 16.0) as u32, u32::MAX));
        // coop : allies (indices juste sous u32::MAX)
        for (k, h) in w.others.iter().enumerate() {
            let y = h.player.prev.lerp(h.player.pos, alpha).y;
            self.sort.push((((y + 65536.0) * 16.0) as u32, u32::MAX - 1 - k as u32));
        }
        // decor (les grands elements depassent : marge plus large en haut)
        self.prop_buf.clear();
        {
            let buf = &mut self.prop_buf;
            let broken = &w.broken;
            sim::props::for_each(&w.map.props, w.prop_seed, lo - Vec2::new(32.0, 0.0), hi + Vec2::new(32.0, 96.0), |p| {
                let cell = ((p.pos.x / sim::props::CELL).floor() as i32, (p.pos.y / sim::props::CELL).floor() as i32);
                if broken.is_empty() || !broken.contains(&cell) {
                    buf.push(p);
                }
            });
        }
        for (k, p) in self.prop_buf.iter().enumerate() {
            self.sort.push((((p.pos.y + 65536.0) * 16.0) as u32, PROP_BIT | k as u32));
        }
        self.sort.sort_unstable_by_key(|e| e.0);

        // images remanentes du dash (sous les personnages)
        self.juice.draw_under(list);
        list.set(Space::World, Blend::Alpha);
        let player = &w.player;
        let (panim, pflip) = self.vis.player.pick(player.face, player.moving);
        let pframe = *atlas.frame(panim, if player.moving { player.walk_t } else { 0.0 });
        for &(_, id) in &self.sort {
            if id < u32::MAX && id >= u32::MAX - crate::coop::MAX as u32 {
                self.draw_ally(list, atlas, (u32::MAX - 1 - id) as usize, alpha);
                continue;
            }
            if id != u32::MAX && id & PROP_BIT != 0 {
                let p = &self.prop_buf[(id & !PROP_BIT) as usize];
                let f = atlas.frame(self.vis.props[p.kind as usize], 0.0);
                let mut prm = Params::default();
                if let Some((r, g, b, a)) = w.map.props[p.kind as usize].tint {
                    prm = prm.flash([r, g, b], a);
                }
                // Link cache derriere un grand element : on le voit a travers
                let top = p.pos.y - f.h as f32;
                if f.h > 28 && pp.y < p.pos.y - 4.0 && pp.y > top + 8.0 && (pp.x - p.pos.x).abs() < f.w as f32 * 0.45 {
                    prm = prm.color([1.0, 1.0, 1.0, 0.55]);
                }
                list.frame(f, p.pos, &prm);
                continue;
            }
            if id == u32::MAX {
                // Link : clignote pendant l'invulnerabilite apres un coup
                let blink = player.hurt_t < self.content.rules.hurt_iframes && ((player.hurt_t * 24.0) as i32) % 2 == 1;
                if !blink || !player.alive {
                    let mut prm = Params::default().flip(pflip);
                    if player.hurt_t < 0.12 {
                        prm = prm.flash([1.0, 1.0, 1.0], 1.0);
                    }
                    if !player.alive {
                        prm = prm.color([1.0, 0.75, 0.75, 1.0]);
                    }
                    // Plume de Roc : le sprite monte et redescend, l'ombre reste au sol
                    let jump_h = if player.jump > 0.0 && player.jump_dur > 0.0 {
                        ((1.0 - player.jump / player.jump_dur) * std::f32::consts::PI).sin() * 20.0
                    } else {
                        0.0
                    };
                    list.frame(&pframe, pp - Vec2::new(0.0, jump_h.round()), &prm);
                }
                continue;
            }
            let i = id as usize;
            let mut p = h.prev[i].lerp(h.pos[i], alpha);
            let k = h.kind[i] as usize;
            let windup = h.state[i] == ST_WINDUP;
            let (anim, flip) = self.vis.enemies[k].pick(h.face[i], h.z[i], windup);
            let phase = (h.seed[i] & 1023) as f32 * 0.001;
            // fige pendant le gel et l'etourdissement
            let t = if h.held(i) { phase } else { wt + phase };
            let f = atlas.frame(anim, t);
            let es = self.vis.enemy_scale[k];
            let mut prm = Params::default().flip(flip);
            let hurt = h.hurt[i];
            if windup {
                // annonce d'attaque : tremble et clignote
                p.x += ((rt * 50.0 + phase * 10.0).sin()).round();
                if (rt * 12.0).fract() < 0.5 {
                    prm = prm.flash([1.0, 1.0, 1.0], 0.55);
                }
            }
            if h.freeze[i] > 0.0 {
                prm = prm.flash([0.55, 0.8, 1.0], 0.55);
            } else if h.burn[i] > 0.0 && (rt * 10.0 + phase * 7.0).fract() < 0.4 {
                prm = prm.flash([1.0, 0.5, 0.15], 0.35);
            }
            if hurt < 0.1 {
                prm = prm.flash([1.0, 1.0, 1.0], 1.0 - hurt / 0.1 * 0.5);
            }
            if hurt < 0.15 {
                let u = 1.0 - hurt / 0.15;
                prm.scale = Vec2::new(prm.scale.x * (1.0 + 0.25 * u), 1.0 - 0.2 * u);
            }
            prm.scale *= es;
            let pos = p - Vec2::new(0.0, h.z[i].round());
            // rencontres (Bob-omb, Pikit voleur) : grand anneau au sol, colonne de lumiere, halo et contour
            let special = if Some(k as u16) == w.bomb_kind {
                Some([1.0, 0.45, 0.15])
            } else if Some(k as u16) == w.thief_kind {
                Some([THIEF[0], THIEF[1], THIEF[2]])
            } else {
                None
            };
            if let Some(c) = special {
                let beat = 0.5 + 0.5 * (rt * 5.0 + phase).sin();
                let r = (9.0 + 2.0 * beat) * es;
                list.set(Space::World, Blend::Add);
                for s in 0..28 {
                    let a = s as f32 / 28.0 * TAU + rt;
                    let q = p + Vec2::new(a.cos() * r, a.sin() * r * 0.45);
                    list.rect(&white, q.round(), Vec2::splat(2.0), [c[0], c[1], c[2], 0.55 + 0.4 * beat]);
                }
                let col_h = 26.0 * es;
                for s in 0..6 {
                    let wd = (6.0 - s as f32) * es;
                    list.rect(&white, (p - Vec2::new(wd * 0.5, col_h)).round(), Vec2::new(wd, col_h), [c[0], c[1], c[2], 0.035 + 0.03 * beat]);
                }
                let halo = Params { scale: prm.scale * 1.3, ..prm }.flash(c, 1.0).color([1.0, 1.0, 1.0, 0.2 + 0.25 * beat]);
                list.frame(f, pos + Vec2::new(0.0, 2.0), &halo);
                list.set(Space::World, Blend::Alpha);
                let edge = Params { flash: [c[0], c[1], c[2], 1.0], ..prm };
                for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                    list.frame(f, pos + d * es, &edge);
                }
            }
            if Some(k as u16) == w.bomb_kind {
                list.set(Space::World, Blend::Add);
                // etincelles qui tournent
                for s in 0..4 {
                    let a = rt * 5.0 + s as f32 * TAU / 4.0;
                    let q = pos - Vec2::new(0.0, 12.0) + Vec2::new(a.cos() * 12.0, a.sin() * 6.0);
                    list.rect(&white, q.round(), Vec2::splat(2.0), [1.0, 0.9, 0.4, 0.9]);
                }
                list.set(Space::World, Blend::Alpha);
                // clignote en rouge comme une bombe sur le point d'exploser
                if (rt * 1.7 + phase).fract() < 0.12 {
                    if let Some(red) = self.vis.bomb_red {
                        list.frame(atlas.frame(red, 0.0), pos, &prm);
                        continue;
                    }
                }
            }
            let champion = w.is_champion(i);
            if champion {
                // champion errant : anneau violet au sol et halo
                let beat = 0.5 + 0.5 * (rt * 4.0).sin();
                let r = 15.0 + 2.0 * beat;
                for s in 0..20 {
                    let a = s as f32 / 20.0 * TAU - rt * 1.5;
                    let q = p + Vec2::new(a.cos() * r, a.sin() * r * 0.45);
                    list.rect(&white, q.round(), Vec2::splat(2.0), with_alpha(CHAMPION, 0.45 + 0.4 * beat));
                }
                list.set(Space::World, Blend::Add);
                let halo = Params { scale: prm.scale * 1.25, ..prm }.flash([0.75, 0.35, 1.0], 1.0).color([1.0, 1.0, 1.0, 0.18 + 0.2 * beat]);
                list.frame(f, pos + Vec2::new(0.0, 2.0), &halo);
                list.set(Space::World, Blend::Alpha);
            }
            if h.elite[i] {
                // contour dore (violet pour un champion) : silhouette decalee d'un pixel dans les 4 directions
                let gold = Params { flash: if champion { [0.85, 0.5, 1.0, 1.0] } else { [1.0, 0.82, 0.25, 1.0] }, ..prm };
                for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                    list.frame(f, pos + d * es, &gold);
                }
            }
            list.frame(f, pos, &prm);
            if champion {
                // marque au-dessus de la tete : petit losange qui flotte
                let top = pos - Vec2::new(0.0, f.h as f32 * es + 6.0 + 2.0 * (rt * 3.0).sin());
                for (dy, wd) in [(-3.0, 1.0), (-2.0, 3.0), (-1.0, 5.0), (0.0, 3.0), (1.0, 1.0)] {
                    list.rect(&white, (top + Vec2::new(-wd * 0.5, dy)).round(), Vec2::new(wd, 1.0), CHAMPION);
                }
            }
            if h.stun[i] > 0.0 {
                // petites etoiles qui tournent au-dessus de la tete
                let top = pos - Vec2::new(0.0, f.h as f32 * es + 2.0);
                for s in 0..2 {
                    let a = rt * 8.0 + s as f32 * std::f32::consts::PI;
                    let q = top + Vec2::new(a.cos() * 5.0, a.sin() * 1.5);
                    list.rect(&white, q.round(), Vec2::ONE, [1.0, 1.0, 0.5, 1.0]);
                }
            }
        }

        // epee : lame + trainee
        if let Some(sw) = player.swing {
            self.draw_swing(list, atlas, &self.vis.player, &sw, pp - Vec2::new(0.0, 9.0), alpha);
        }

        // chaines du grappin (sous la tete), depuis la main de chaque joueur (allies compris)
        let hands: Vec<Vec2> = std::iter::once(pp).chain(w.others.iter().map(|h| h.player.prev.lerp(h.player.pos, alpha))).map(|q| q - Vec2::new(0.0, 8.0)).collect();
        let mut first = 0;
        let owned = self.proj_ends.iter().enumerate().flat_map(|(who, &end)| {
            let r = first.min(w.projectiles.len())..end.min(w.projectiles.len());
            first = end;
            w.projectiles[r].iter().map(move |p| (who, p))
        });
        for (who, p) in owned {
            if p.beam || !self.vis.weapons[p.weapon as usize].chain {
                continue;
            }
            let hand = hands[who.min(hands.len() - 1)];
            let pos = p.prev.lerp(p.pos, alpha);
            let d = pos - hand;
            let n = (d.length() / 5.0) as usize;
            for s in 1..n {
                let q = hand + d * (s as f32 / n as f32);
                list.rect(&white, (q - Vec2::ONE).round(), Vec2::splat(2.0), [0.35, 0.35, 0.42, 1.0]);
                list.rect(&white, (q - Vec2::ONE).round(), Vec2::ONE, [0.85, 0.85, 0.95, 1.0]);
            }
        }
        // projectiles du joueur
        for p in &w.projectiles {
            let pos = p.prev.lerp(p.pos, alpha);
            if !in_view(pos) {
                continue;
            }
            // facon retro : un projectile qui va disparaitre clignote (pas le boomerang qui revient,
            // ni les bombes et blocs qui ont leur propre clignotement)
            if matches!(p.kind, ProjKind::Straight | ProjKind::Homing | ProjKind::Tornado) && expiring(p.life - p.age, (p.life * 0.3).min(0.5)) {
                continue;
            }
            let wv = &self.vis.weapons[p.weapon as usize];
            let (anim, orient) = match (p.beam, wv.beam, wv.sprite) {
                (true, Some(b), _) => b,
                (false, _, Some(s)) => (s, wv.orient),
                _ => continue,
            };
            let mut prm = Params::default();
            if let Orient::Angle(up) = orient {
                prm = prm.rot(p.rot + if up { FRAC_PI_2 } else { 0.0 });
            }
            if let Some(tint) = wv.tint {
                prm = prm.flash(tint, 0.45);
            }
            let f = atlas.frame(anim, wt + p.age);
            match p.kind {
                ProjKind::Lob => {
                    // bombe posee : la meche clignote de plus en plus vite
                    if p.age >= p.flight {
                        let u = ((p.age - p.flight) / p.life.max(0.01)).min(1.0);
                        if (p.age * (6.0 + u * 22.0)).fract() < 0.5 {
                            prm = prm.flash([1.0, 0.3, 0.2], 0.6);
                        }
                    }
                    list.frame(f, pos + Vec2::new(0.0, 4.0) - Vec2::new(0.0, p.z.round()), &prm);
                }
                ProjKind::Block => {
                    // le bloc apparait d'un coup puis clignote avant d'eclater
                    let pop = 1.0 + 0.6 * (1.0 - (p.age / 0.12).min(1.0));
                    prm = prm.scale(Vec2::splat(pop));
                    let u = p.age / p.life.max(0.01);
                    if u > 0.55 && (p.age * (10.0 + u * 20.0)).fract() < 0.5 {
                        prm = prm.flash([1.0, 0.9, 0.6], 0.7);
                    }
                    list.frame(f, pos, &prm);
                }
                ProjKind::Homing => {
                    list.frame(f, pos, &prm.flip(p.vel.x < 0.0));
                }
                ProjKind::Tornado => {
                    // tornade : grains en spirale, plus larges en haut (cone), qui tournent
                    let def = &self.content.weapons[p.weapon as usize];
                    let t = def.tint.unwrap_or((0.85, 0.95, 1.0));
                    let r = p.radius.max(6.0);
                    let layers = 12;
                    list.set(Space::World, Blend::Add);
                    for k in 0..layers {
                        let kf = k as f32 / (layers - 1) as f32;
                        let h = kf * r * 2.2;
                        let w = r * (0.25 + 0.75 * kf);
                        for s in 0..3 {
                            let a = p.rot * (1.0 + kf) + k as f32 * 0.9 + s as f32 * TAU / 3.0;
                            let q = pos + Vec2::new(0.0, 6.0) + Vec2::new(a.cos() * w, -h + a.sin() * w * 0.3);
                            let al = 0.55 + 0.4 * a.sin().abs();
                            list.rect(&white, q.round(), Vec2::splat(3.0), [t.0, t.1, t.2, al]);
                        }
                        if k % 3 == 1 {
                            let a = p.rot * 1.3 + k as f32;
                            let q = pos + Vec2::new(0.0, 6.0) + Vec2::new(a.cos() * w * 0.6, -h);
                            list.frame(f, q.round(), &Params::default().color([t.0, t.1, t.2, 0.8]));
                        }
                    }
                    list.set(Space::World, Blend::Alpha);
                }
                _ => {
                    list.frame(f, pos, &prm);
                    if p.beam {
                        let g = wv.beam_glow;
                        list.set(Space::World, Blend::Add);
                        list.frame(f, pos, &prm.color([g[0], g[1], g[2], 0.6]));
                        list.set(Space::World, Blend::Alpha);
                    }
                }
            }
        }

        // objets en orbite (Canne de Byrna, fees) et poudre magique, autour de chaque joueur (allies compris)
        let carriers: Vec<(&[sim::upgrades::WeaponSlot], Vec2)> =
            std::iter::once((&w.weapons[..], pp)).chain(w.others.iter().map(|h| (&h.weapons[..], h.player.prev.lerp(h.player.pos, alpha)))).collect();
        for &(weapons, pp) in &carriers {
            for ws in weapons {
                let def = &self.content.weapons[ws.id as usize];
                if def.kind != WeaponKind::Orbit || ws.active <= 0.0 {
                    continue;
                }
                let Some(anim) = self.vis.weapons[ws.id as usize].sprite else { continue };
                let phase = ws.phase - ws.stats.speed * DT * (1.0 - alpha);
                let a = (ws.active / 0.3).min(1.0);
                for (k, q) in ws.orbit_points(pp - Vec2::new(0.0, 6.0), phase).into_iter().enumerate() {
                    let f = atlas.frame(anim, wt + k as f32 * 0.13);
                    list.frame(f, q, &Params::default().color([1.0, 1.0, 1.0, a]));
                }
            }

            // poudre magique : nuage scintillant autour de Link
            for ws in weapons {
                let def = &self.content.weapons[ws.id as usize];
                if def.kind != WeaponKind::Aura {
                    continue;
                }
                let Some(anim) = self.vis.weapons[ws.id as usize].sprite else { continue };
                let r = ws.stats.area;
                let n = ((r * 0.7) as usize).clamp(12, 48);
                // fanal, jarre : moitie moins de grains (gros sprites de flamme)
                let n = if def.tint.is_some() { n / 2 } else { n };
                let tint = match def.tint {
                    Some(t) => [t.0, t.1, t.2],
                    None if def.evolved => [1.0, 0.7, 1.0],
                    None => [1.0, 1.0, 1.0],
                };
                list.set(Space::World, Blend::Add);
                // bord du nuage : anneau pointille qui tourne lentement (portee lisible)
                let ring = ((r * 0.45) as usize).clamp(20, 72);
                let edge = match def.tint {
                    Some(t) => [t.0, t.1, t.2],
                    None if def.evolved => [1.0, 0.45, 0.25],
                    None => [0.75, 0.6, 1.0],
                };
                for s in 0..ring {
                    let a = s as f32 / ring as f32 * TAU + rt * 0.4;
                    let q = pp - Vec2::new(0.0, 2.0) + Vec2::new(a.cos() * r, a.sin() * r * 0.6);
                    let al = 0.45 + 0.25 * (rt * 2.0 + s as f32 * 0.7).sin();
                    list.rect(&white, q.round(), Vec2::splat(2.0), [edge[0], edge[1], edge[2], al]);
                }
                for k in 0..n {
                    let kf = k as f32;
                    // fanal, jarre (teinte propre) : grains sur le pourtour, pour ne pas cacher le joueur
                    let ring_only = def.tint.is_some();
                    let rr = if ring_only { r * (0.7 + 0.3 * (kf * 0.618).fract()) } else { r * (0.3 + 0.7 * (kf * 0.618).fract()) };
                    let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let a = kf * 2.399 + rt * (0.7 + 0.5 * (kf * 0.37).fract()) * dir;
                    let q = pp - Vec2::new(0.0, 6.0) + Vec2::new(a.cos(), a.sin() * 0.6) * rr;
                    let f = atlas.frame(anim, rt + kf * 0.17);
                    let al = 0.7 + 0.3 * (rt * 3.0 + kf).sin().abs();
                    // un grain sur trois en x2 : le nuage se lit mieux dans la foule
                    let sc = if k % 3 == 0 && !ring_only { 2.0 } else { 1.0 };
                    list.frame(f, q.round(), &Params { scale: Vec2::splat(sc), ..Params::default() }.color([tint[0], tint[1], tint[2], al]));
                }
                list.set(Space::World, Blend::Alpha);
            }
        }

        // tirs ennemis
        for s in &w.shots {
            let pos = s.prev.lerp(s.pos, alpha);
            if !in_view(pos) {
                continue;
            }
            let Some(&(anim, orient)) = self.vis.shots.get(s.sprite as usize) else { continue };
            let mut prm = Params::default();
            if let Orient::Angle(up) = orient {
                prm = prm.rot(s.vel.y.atan2(s.vel.x) + if up { FRAC_PI_2 } else { 0.0 });
            }
            let f = atlas.frame(anim, wt + s.age);
            if !s.lob && expiring(s.life - s.age, (s.life * 0.3).min(0.5)) {
                continue;
            }
            if s.lob {
                // bombe du boss : clignote au sol avant d'exploser
                if s.age >= s.flight && (s.age * 14.0).fract() < 0.5 {
                    prm = prm.flash([1.0, 0.3, 0.2], 0.7);
                }
                list.frame(f, pos - Vec2::new(0.0, s.z.round()), &prm);
            } else {
                // lisibilite dans la foule : courte trainee, halo rouge qui pulse et contour sombre
                let dir = s.vel.normalize_or_zero();
                for k in 1..=2 {
                    let q = pos - dir * (5.0 * k as f32);
                    list.frame(f, q, &prm.color([1.0, 0.55, 0.45, 0.3 / k as f32]));
                }
                let beat = 0.5 + 0.5 * (rt * 10.0 + s.age * 3.0).sin();
                list.set(Space::World, Blend::Add);
                let halo = Params { scale: prm.scale * 1.7, ..prm }.flash([1.0, 0.25, 0.1], 1.0).color([1.0, 1.0, 1.0, 0.3 + 0.25 * beat]);
                list.frame(f, pos, &halo);
                list.set(Space::World, Blend::Alpha);
                let edge = Params { flash: [0.25, 0.0, 0.05, 1.0], ..prm };
                for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                    list.frame(f, pos + d, &edge);
                }
                list.frame(f, pos, &prm);
            }
        }

        // barres de vie sous les mini-boss : elites, champions errants, rencontres (voleur, Bob-omb) et
        // boss de mi-parcours
        list.set(Space::World, Blend::Alpha);
        for i in 0..h.len() {
            let k = h.kind[i] as usize;
            let boss = self.content.enemies[k].boss;
            let mid_boss = boss && w.final_uid != Some(h.uid[i]);
            let thief = Some(h.kind[i]) == w.thief_kind;
            let bomb = Some(h.kind[i]) == w.bomb_kind;
            if !(h.elite[i] || mid_boss || thief || bomb) || h.hp[i] <= 0.0 {
                continue;
            }
            let p = h.prev[i].lerp(h.pos[i], alpha);
            if !in_view(p) {
                continue;
            }
            let frac = (h.hp[i] / h.max_hp[i].max(1.0)).clamp(0.0, 1.0);
            let es = self.vis.enemy_scale[k];
            let bw = if boss { 44.0 } else { (18.0 * es).max(20.0) };
            let at = Vec2::new((p.x - bw * 0.5).round(), (p.y + 4.0 + 2.0 * es).round());
            let edge = if w.is_champion(i) {
                CHAMPION
            } else if thief {
                THIEF
            } else if bomb {
                rgb(255, 150, 80)
            } else if boss {
                rgb(255, 110, 110)
            } else {
                GOLD
            };
            list.rect(&white, at - Vec2::ONE, Vec2::new(bw + 2.0, 5.0), with_alpha(edge, 0.85));
            list.rect(&white, at, Vec2::new(bw, 3.0), [0.12, 0.02, 0.04, 1.0]);
            list.rect(&white, at, Vec2::new((bw * frac).round().max(1.0), 3.0), rgb(230, 40, 50));
            list.rect(&white, at, Vec2::new((bw * frac).round().max(1.0), 1.0), rgb(255, 140, 140));
        }

        // jauges sous Link
        self.hud.draw_under_player(list, w, &self.vis, pp);

        // petits nuages de poussiere en marchant
        if player.moving && !player.dashing() && player.alive {
            self.dust_t -= self.frame_dt;
            if self.dust_t <= 0.0 {
                self.dust_t = 0.3;
                self.juice.anim_ex(self.vis.dust, pp + Vec2::new(0.0, 1.0), 0.22, 0.0, 1.0, [1.0, 1.0, 1.0, 0.45], false);
            }
        }
        // nouvelles images remanentes pendant le dash
        if player.dashing() {
            self.ghost_t -= self.frame_dt;
            if self.ghost_t <= 0.0 {
                self.ghost_t = 0.025;
                self.juice.ghost(pframe, pp, pflip, 0.22, [0.45, 0.75, 1.0, 1.0]);
            }
        }
    }

    /// Fleche d'indication (sprite de la police d'A Link to the Past) au bord de l'ecran, tournee d'un
    /// quart de tour vers la direction dominante (pixels nets), teintee, qui avance et recule un peu.
    fn edge_arrow(&self, ctx: &Ctx, list: &mut DrawList, c: Vec2, d: Vec2, color: Color, blink: bool) {
        let card = if d.x.abs() >= d.y.abs() { Vec2::new(d.x.signum(), 0.0) } else { Vec2::new(0.0, d.y.signum()) };
        let rot = card.y.atan2(card.x);
        let push = (3.0 * (ctx.real_time as f32 * 5.0).sin().abs()).round();
        let at = (c + card * push).round();
        let a = if blink && (ctx.real_time * 3.0).fract() >= 0.65 { 0.65 } else { 1.0 };
        let pr = Params { rot, ..Default::default() };
        // contour sombre puis fleche agrandie et teintee : visible sur tous les decors
        for o in [Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0)] {
            list.frame(ctx.atlas.frame(self.vis.ui_arrow, 0.0), at + o * 2.0, &pr.scale(Vec2::splat(2.0)).color([0.0, 0.0, 0.0, 0.7 * a]));
        }
        list.frame(ctx.atlas.frame(self.vis.ui_arrow, 0.0), at, &pr.scale(Vec2::splat(2.0)).color(with_alpha(color, a)));
    }

    /// Point du monde -> pixels d'interface (camera au centre, zoom compris).
    pub(crate) fn to_ui(&self, p: Vec2, cam: Vec2, view: Vec2) -> Vec2 {
        (p - cam) * self.view_zoom + view * 0.5
    }

    /// Zoom en partie : la touche Zoom (E / RB, Options > Commandes) et la loupe tactile passent au
    /// niveau suivant, la molette rapproche ou eloigne ; la vue glisse vers le niveau choisi (garde dans
    /// les reglages).
    fn update_zoom(&mut self, ctx: &mut Ctx, dt: f32) {
        let n = ZOOMS.len() as u8;
        let tap = std::mem::take(&mut self.touch.zoom);
        let mut level = self.settings.zoom.min(n - 1);
        if matches!(self.state, State::Playing) {
            if tap || ctx.input.action_pressed(&self.bindings, "zoom") {
                level = (level + 1) % n;
            }
            // le glisser tactile emule aussi la molette (menus) : seule la vraie molette zoome
            if ctx.input.last_device == Device::Mouse {
                self.zoom_wheel += ctx.input.wheel();
                if self.zoom_wheel.abs() >= 1.0 {
                    level = if self.zoom_wheel > 0.0 { (level + 1).min(n - 1) } else { level.saturating_sub(1) };
                    self.zoom_wheel = 0.0;
                }
            }
        } else {
            self.zoom_wheel = 0.0;
        }
        if level != self.settings.zoom {
            self.settings.zoom = level;
            self.settings.save(&ctx.root);
        }
        let target = ZOOMS[level as usize];
        self.zoom += (target - self.zoom) * (1.0 - (-dt * 10.0).exp());
        let tile = 16.0 * ctx.scale as f32;
        self.view_zoom = (self.zoom * tile).round() / tile;
    }

    /// Point du bord de l'ecran vers une cible hors champ (None si elle est visible), a l'ecart du HUD.
    fn edge_point(view: Vec2, ui: Vec2) -> Option<(Vec2, Vec2)> {
        if ui.x >= 0.0 && ui.y >= 0.0 && ui.x <= view.x && ui.y <= view.y {
            return None;
        }
        let m = 22.0;
        let c = Vec2::new(ui.x.clamp(m, view.x - m), ui.y.clamp(m + 74.0, view.y - 40.0));
        Some((c, (ui - c).normalize_or_zero()))
    }

    /// Fleches au bord de l'ecran vers ce qui est hors champ : champion, Pikit voleur, Bob-omb, coffres.
    fn draw_chest_markers(&self, ctx: &Ctx, list: &mut DrawList, cam: Vec2) {
        let view = ctx.view;
        let pk = &self.world.pickups;
        list.set(Space::Ui, Blend::Alpha);
        let h = &self.world.horde;
        let font = ctx.atlas.font("main");
        let blink = (ctx.real_time * 3.0).fract() < 0.65;
        if let (Some(e), Some(cp)) = (self.world.encounter.as_ref(), self.world.champion_pos()) {
            let ui = self.to_ui(cp, cam, view);
            match Self::edge_point(view, ui) {
                Some((c, d)) => self.edge_arrow(ctx, list, c, d, CHAMPION, true),
                None if !e.engaged => {
                    let s = format!("{}s", e.t.ceil() as i32);
                    crate::hud::text(list, font, &s, ui + Vec2::new(0.0, 6.0), 1.0, with_alpha(CHAMPION, if blink { 1.0 } else { 0.7 }), engine::draw::Align::Center);
                }
                None => {}
            }
        }
        if let (Some(th), Some(tp)) = (self.world.thief.as_ref(), self.world.thief_pos()) {
            let ui = self.to_ui(tp, cam, view);
            match Self::edge_point(view, ui) {
                Some((c, d)) => self.edge_arrow(ctx, list, c, d, THIEF, true),
                None => {
                    let s = match th.stolen {
                        0 => format!("{}s", th.t.ceil() as i32),
                        n => format!("{}s  -{n}", th.t.ceil() as i32),
                    };
                    crate::hud::text(list, font, &s, ui + Vec2::new(0.0, 8.0), 1.0, with_alpha(THIEF, if blink { 1.0 } else { 0.7 }), engine::draw::Align::Center);
                }
            }
        }
        if let Some(bk) = self.world.bomb_kind {
            for i in 0..h.len() {
                if h.kind[i] == bk {
                    if let Some((c, d)) = Self::edge_point(view, self.to_ui(h.pos[i], cam, view)) {
                        self.edge_arrow(ctx, list, c, d, rgb(255, 140, 90), true);
                    }
                }
            }
        }
        for i in 0..pk.len() {
            if pk.kind[i].is_chest() {
                if let Some((c, d)) = Self::edge_point(view, self.to_ui(pk.pos[i], cam, view)) {
                    self.edge_arrow(ctx, list, c, d, GOLD, false);
                }
            }
        }
    }

    fn draw_overlays(&self, ctx: &Ctx, list: &mut DrawList) {
        let view = ctx.view;
        let white = &self.vis.white;
        list.set(Space::Ui, Blend::Alpha);
        let (c, t, d) = self.juice.flash;
        if t > 0.0 {
            list.rect(white, Vec2::ZERO, view, with_alpha(c, t / d));
        }
        let lang = &self.lang;
        match &self.state {
            State::Title { .. } | State::About { .. } | State::Fairy { .. } => {}
            State::Paused { sel } => menus::draw_pause(list, ctx, &self.vis, lang, *sel, &self.world, &self.profile.evolutions, self.fallback),
            State::Options { sel, .. } => {
                let code_idx = LANGS.iter().position(|&c| c == self.settings.lang).unwrap_or(0);
                let v = OptValues {
                    lang_name: &self.lang_names[code_idx],
                    video: ctx.video,
                    aspect: ctx.aspect,
                    filter: self.settings.filter,
                    master: self.settings.master,
                    music: self.settings.music,
                    sfx: self.settings.sfx,
                    fullscreen: ctx.fullscreen,
                    updates: self.settings.updates,
                };
                menus::draw_options(list, ctx, &self.vis, lang, *sel, &v);
            }
            State::LevelUp { offers, sel, t } => menus::draw_levelup(list, ctx, &self.world, &self.vis, lang, offers, *sel, *t),
            State::Chest { reward, big, t } => menus::draw_chest(list, ctx, &self.world, &self.vis, lang, reward, *big, *t),
            State::EvoChoice { options, sel, t, .. } => menus::draw_evo_choice(list, ctx, &self.world, &self.vis, lang, options, *sel, *t),
            State::Unlocks { items, t, .. } => menus::draw_unlocks(list, ctx, &self.content, &self.vis, lang, items, *t),
            State::Reward { offers, sel, t } => {
                let infos: Vec<_> = offers.iter().map(|&o| reward::info(&self.world, &self.vis, &ctx.atlas, lang, &self.meta, o)).collect();
                menus::draw_cards_themed(list, ctx, &self.vis, lang, (lang.get("ui.reward.title"), lang.get("ui.reward.sub"), CHAMPION), &infos, *sel, *t, CHAMPION);
            }
            State::Fortune { phase, sel, t } => self.draw_fortune(ctx, list, phase, *sel, *t),
            State::Witch { sel, t } => self.draw_witch(ctx, list, *sel, *t),
            State::Dying(t) => {
                let a = (t / 1.8).min(1.0) * 0.55;
                list.rect(white, Vec2::ZERO, view, [0.25, 0.0, 0.0, a]);
            }
            State::Dead { t, sel } => menus::draw_dead(list, ctx, &self.world, &self.vis, lang, *t, *sel, self.last_score, self.last_banked),
            State::Victory { t, sel } => {
                let items = stages::victory_items(self.next_stage().is_some());
                let mut notes = Vec::new();
                if let Some(id) = &self.last_unlocked {
                    notes.push(lang.get("ui.new_stage").replace("{s}", &crate::hud::hl_new(&stages::stage_name(lang, id))));
                }
                // une ligne par sorte de deblocage : la liste reste courte meme apres une grosse partie
                let relics: Vec<String> = self.last_relics.iter().map(|id| crate::hud::hl(lang.get(&format!("relic.{id}.name")))).collect();
                if !relics.is_empty() {
                    let key = if relics.len() > 1 { "ui.relics_found" } else { "ui.relic_found" };
                    notes.push(lang.get(key).replace("{s}", &relics.join(", ")));
                }
                let mut new_chars: Vec<String> = self.last_chars.iter().take(5).map(|id| crate::hud::hl_new(&chars::name(lang, id))).collect();
                if self.last_chars.len() > 5 {
                    new_chars.push(format!("+{}", self.last_chars.len() - 5));
                }
                if !new_chars.is_empty() {
                    let key = if new_chars.len() > 1 { "ui.new_characters" } else { "ui.new_character" };
                    notes.push(lang.get(key).replace("{s}", &new_chars.join(", ")));
                }
                stages::draw_victory(list, ctx, &self.world, lang, *t, *sel, &items, self.last_banked, self.last_score, &notes);
            }
            State::CharSelect { .. }
            | State::SkinBuy { .. }
            | State::StageSelect { .. }
            | State::Shop { .. }
            | State::Prep { .. }
            | State::Records { .. }
            | State::Players { .. }
            | State::NameEntry { .. }
            | State::Avatars { .. }
            | State::Gallery { .. }
            | State::Coop { .. }
            | State::Lobby { .. }
            | State::Scores { .. } => {}
            State::Controls { .. } => self.draw_hub(ctx, list),
            State::Playing => {
                self.touch.draw(list, ctx, lang, self.world.player.auto_attack);
                let p = &self.world.player;
                if p.alive && p.hp < p.max_hp * 0.3 {
                    let beat = 0.5 + 0.5 * (ctx.real_time as f32 * 6.0).sin();
                    let a = 0.16 + 0.14 * beat;
                    for k in 0..6 {
                        let th = 3.0;
                        let o = k as f32 * th;
                        let c = [0.85, 0.05, 0.05, a * (1.0 - k as f32 / 6.0)];
                        list.rect(white, Vec2::new(o, o), Vec2::new(view.x - 2.0 * o, th), c);
                        list.rect(white, Vec2::new(o, view.y - o - th), Vec2::new(view.x - 2.0 * o, th), c);
                        list.rect(white, Vec2::new(o, o + th), Vec2::new(th, view.y - 2.0 * o - 2.0 * th), c);
                        list.rect(white, Vec2::new(view.x - o - th, o + th), Vec2::new(th, view.y - 2.0 * o - 2.0 * th), c);
                    }
                }
            }
        }
        if self.debug {
            let fr = ctx.atlas.font("main");
            let s = &ctx.stats;
            let w = &self.world;
            let lines = [
                format!("{:.0} ips  image {:.2} ms", s.fps, s.frame_ms),
                format!("sim {:.2} ms  dessin {:.2} ms  rendu {:.2} ms", s.tick_ms, s.draw_ms, s.render_ms),
                format!("ennemis {}  objets {}  sprites {}", w.horde.len(), w.pickups.len(), s.instances),
                format!("projectiles {}  zones {}  tirs {}", w.projectiles.len(), w.zones.len(), w.shots.len()),
                format!("ralentis {} accordes, {} refuses", self.director.stats.0, self.director.stats.1),
                format!("x{}  vue {}x{}  {}", ctx.scale, view.x as u32, view.y as u32, if ctx.vsync { "synchro" } else { "libre" }),
                ctx.gpu.clone(),
            ];
            let y0 = view.y - 14.0 * lines.len() as f32 - 30.0;
            list.rect(white, Vec2::new(view.x - 260.0, y0 - 3.0), Vec2::new(256.0, 14.0 * lines.len() as f32 + 4.0), [0.0, 0.0, 0.0, 0.55]);
            for (k, l) in lines.iter().enumerate() {
                list.text(fr, l, Vec2::new(view.x - 256.0, y0 + k as f32 * 14.0), rgb(200, 255, 200), engine::draw::Align::Left);
            }
        }
    }
}

impl Game {
    /// Curseur de la souris : la fee de l'ecran de selection des fichiers d'A Link to the Past, qui
    /// flotte doucement ; pointe = centre de la fee. Cache quand on joue au clavier ou a la manette.
    fn draw_cursor(&self, ctx: &Ctx, list: &mut DrawList) {
        let i = &ctx.input;
        if i.last_device != Device::Mouse || !i.mouse_inside || !i.focused {
            return;
        }
        let m = ctx.mouse_ui().round();
        list.set(Space::Ui, Blend::Alpha);
        let t = ctx.real_time as f32;
        let f = ctx.atlas.frame(self.vis.cursor, t);
        // ombre puis fee
        list.frame(f, m + Vec2::new(1.0, 1.0), &Params::default().color([0.0, 0.0, 0.0, 0.45]));
        list.frame(f, m, &Params::default());
    }

    fn draw_scene(&mut self, ctx: &Ctx, alpha: f32, list: &mut DrawList) -> Frame {
        if self.state.in_title() {
            // dernier stage (Ganon) gagne par ce joueur : logo illumine
            let last = self.stage_ids.last().and_then(|id| self.profile.stages.get(id));
            self.title_vis.champion = last.is_some_and(|r| r.wins > 0) || self.opts.menu.as_deref() == Some("champion");
            let white = self.vis.white;
            match &self.state {
                State::About { sel, toast } => {
                    title::draw_about(list, ctx, &self.title_vis, &self.lang, &white, &self.links, *sel, self.menu_t, *toast)
                }
                State::Title { sel } => {
                    let items = self.title_items();
                    title::draw_title(list, ctx, &self.title_vis, &self.lang, &white, &items, *sel, self.profile.avatar, self.menu_t)
                }
                State::Fairy { page, t, ask } => {
                    // decor seul : le menu revient a la fin, curseur sur la Boutique
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    fairy_tip::draw(list, ctx, &self.lang, &white, *page, *t, *ask);
                }
                State::Shop { .. } | State::Prep { .. } | State::Records { .. } | State::Controls { .. } | State::Scores { .. } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    self.draw_hub(ctx, list);
                }
                State::Players { sel, armed } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    let views: Vec<Option<players::SlotView>> = (0..profiles::SLOTS)
                        .map(|k| {
                            if k == self.profile.slot {
                                Some(players::SlotView::of(&self.profile))
                            } else {
                                self.slots.get(k).and_then(|s| s.as_ref()).map(players::SlotView::of)
                            }
                        })
                        .collect();
                    let toast = if self.hub_toast.1 > 0.0 { self.hub_toast.0.as_str() } else { "" };
                    players::draw(list, ctx, &self.lang, &views, self.profile.slot, *sel, *armed, self.stage_ids.len(), toast, self.menu_t);
                }
                State::Gallery { .. } => {
                    if let Some(room) = &self.gallery {
                        let left = self.content.characters.get("link").is_some_and(|d| d.side_faces_left);
                        gallery::draw(list, ctx, room, &self.content, &self.meta, &self.vis, &self.profile, &self.lang, left, self.menu_t);
                    }
                    if let Some((k, t0)) = self.beast {
                        self.draw_beast(ctx, list, k, (ctx.real_time - t0) as f32);
                    }
                    if let Some((ct, k, t0)) = self.card {
                        self.draw_card(ctx, list, ct, k, (ctx.real_time - t0) as f32);
                    }
                }
                State::Coop { sel } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    self.draw_coop_menu(ctx, list, *sel);
                }
                State::Lobby { sel } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    self.draw_lobby(ctx, list, *sel);
                }
                State::Avatars { sel, .. } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    players::draw_avatars(list, ctx, &self.lang, &self.profile.name, *sel, self.menu_t);
                }
                State::NameEntry { purpose, text, sel } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    let (key, avatar) = match purpose {
                        NamePurpose::Create(k) => ("ui.name_new", self.free_avatar(*k)),
                        NamePurpose::Rename => ("ui.name_rename", self.profile.avatar),
                        NamePurpose::Setup => ("ui.name_setup", self.profile.avatar),
                        NamePurpose::Pseudo { .. } => ("ui.name_pseudo", self.profile.avatar),
                        NamePurpose::Room => ("ui.coop.code", self.profile.avatar),
                    };
                    let hint = match purpose {
                        NamePurpose::Setup => "ui.name_hint_setup",
                        NamePurpose::Room => "ui.coop.code_hint",
                        _ => "ui.name_hint",
                    };
                    players::draw_name(list, ctx, &self.lang, (self.lang.get(key), hint), text, *sel, Some(avatar), self.menu_t);
                }
                State::CharSelect { page, sel } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    let ids = chars::page_roster(&self.content, *page);
                    let browse = self.skin_browse.as_ref().filter(|b| ids.get(*sel) == Some(&b.0)).map(|b| b.1);
                    chars::draw_select(list, ctx, &self.lang, &self.content, &self.meta, &self.profile, (*page, *sel, self.char_top, self.char_lock), self.menu_t, browse);
                }
                State::SkinBuy { page, sel, skin, ans } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    chars::draw_select(list, ctx, &self.lang, &self.content, &self.meta, &self.profile, (*page, *sel, self.char_top, true), self.menu_t, Some(*skin));
                    let ids = chars::page_roster(&self.content, *page);
                    let q = fairy_tip::question(ctx, &self.skin_question(&ids[*sel], *skin));
                    q.draw(list, ctx, &white, [self.lang.get("ui.skin_no"), self.lang.get("ui.skin_yes")], *ans);
                }
                State::StageSelect { sel } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    stages::draw_select(list, ctx, &self.lang, &self.content, &self.profile, &self.stage_ids, *sel, self.menu_t);
                }
                _ => {
                    // options ouvertes depuis le titre : le decor reste visible dessous
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    self.draw_overlays(ctx, list);
                }
            }
            return Frame { camera: Vec2::ZERO, zoom: 1.0, clear: [0.0, 0.0, 0.0] };
        }
        let w = &self.world;
        let pp = w.player.prev.lerp(w.player.pos, alpha) - Vec2::new(0.0, 8.0);
        let cam = match self.cam {
            None => pp,
            Some(c) => c + (pp - c) * (1.0 - (-self.frame_dt * 12.0).exp()),
        };
        self.cam = Some(cam);
        let cam_shaken = cam + self.juice.shake;
        // coop : projectiles et zones des allies dessines avec ceux du joueur, le temps du dessin
        let own = (self.world.projectiles.len(), self.world.zones.len());
        self.proj_ends.clear();
        self.proj_ends.push(own.0);
        for k in 0..self.world.others.len() {
            let (p, z) = (self.world.others[k].projectiles.clone(), self.world.others[k].zones.clone());
            self.world.projectiles.extend(p);
            self.world.zones.extend(z);
            self.proj_ends.push(self.world.projectiles.len());
        }
        self.draw_world(ctx, alpha, list, cam_shaken);
        self.world.projectiles.truncate(own.0);
        self.world.zones.truncate(own.1);
        self.juice.draw_over(list, &ctx.atlas, &self.vis.white);
        self.draw_stage_fx(ctx, alpha, list, cam_shaken);
        self.juice.draw_numbers(list, &ctx.atlas);
        self.draw_chest_markers(ctx, list, cam_shaken);
        self.draw_npc_markers(ctx, list, cam_shaken);
        self.draw_ally_tags(ctx, list, cam_shaken, alpha);
        let mult = self.coop_rules.map_or_else(|| self.meta.bank_mult(&self.profile), |r| r.bank);
        self.hud.banked = (bank_amount(self.world.player.rupees, false, mult) as f32 * self.run_bank_mult).round() as u64;
        self.hud.busy = !matches!(self.state, State::Playing | State::Dying(_));
        self.hud.paused = matches!(self.state, State::Paused { .. });
        self.hud.draw(list, ctx, &self.world, &self.vis, &self.lang);
        self.draw_coop_overlay(ctx, list);
        self.draw_overlays(ctx, list);
        self.draw_choice_timer(ctx, list);
        Frame { camera: cam_shaken, zoom: self.view_zoom, clear: [0.0, 0.0, 0.0] }
    }
}

impl GameTrait for Game {
    fn frame(&mut self, ctx: &mut Ctx, dt: f32) {
        self.frame_dt = dt;
        if RUN_CHECKPOINTS {
            let lost_focus = self.was_focused && !ctx.input.focused;
            self.was_focused = ctx.input.focused;
            if lost_focus || ctx.real_time >= self.checkpoint_at {
                self.checkpoint_at = ctx.real_time + 10.0;
                self.save_run_checkpoint(ctx);
            }
        }
        let i = &ctx.input;
        let (f3, f4, f12) = (i.key_pressed(KeyCode::F3), i.key_pressed(KeyCode::F4), i.key_pressed(KeyCode::F12));
        if f3 {
            self.debug = !self.debug;
        }
        if f4 {
            let v = !ctx.vsync;
            ctx.set_vsync(v);
        }
        if f12 && cfg!(not(target_arch = "wasm32")) {
            let dir = ctx.root.join("screenshots");
            let _ = std::fs::create_dir_all(&dir);
            let p = dir.join(format!("capture_{}.png", (ctx.real_time * 1000.0) as u64));
            ctx.capture(p);
        }
        // bouton Select de la manette : plein ecran (comme F11)
        if ctx.input.action_pressed(&self.bindings, "fullscreen") {
            let on = !ctx.fullscreen;
            ctx.set_fullscreen(on);
            self.settings.fullscreen = on;
            self.settings.save(&ctx.root);
        }
        if self.opts.uicheck {
            self.opts.uicheck = false;
            self.ui_check(ctx);
            ctx.quit();
            return;
        }
        let nav = self.nav.read(ctx, &self.bindings, dt);
        self.menu_t += dt;
        self.poll_updates();
        self.coop_frame(ctx, dt);
        self.update_menus(ctx, &nav, dt);
        self.title_ambience(ctx);
        self.update_zoom(ctx, dt);

        self.juice.hitstop = (self.juice.hitstop - dt).max(0.0);
        if matches!(self.state, State::Playing) {
            self.director.update(dt);
            self.mass_fx_cd = (self.mass_fx_cd - dt).max(0.0);
        }
        ctx.time_scale = match self.state {
            _ if self.state.frozen() => 0.0,
            _ if self.juice.hitstop > 0.0 => 0.0,
            State::Dying(_) => 0.3,
            _ => self.director.scale(),
        };
        // coop : le temps ne ralentit pas pour un seul joueur (menus et effets compris)
        if self.coop_run() {
            ctx.time_scale = 1.0;
        }
        self.juice.update(dt * ctx.time_scale, dt);
        self.hud.update(dt, &self.world);
        self.rupee_chain_t -= dt * ctx.time_scale;
        if self.rupee_chain_t <= 0.0 {
            self.rupee_chain = 0;
        }
        // coop : la taille d'ecran de la partie est commune (lancement)
        if !self.coop_run() {
            self.world.view_half = ctx.view * 0.5;
        }

        // bip de vie basse facon ALttP
        let p = &self.world.player;
        if matches!(self.state, State::Playing) && p.alive && p.hp <= p.max_hp * 0.25 {
            self.low_hp_t -= dt;
            if self.low_hp_t <= 0.0 {
                self.low_hp_t = 0.9;
                self.sfx(ctx, "low_hp");
            }
        } else {
            self.low_hp_t = 0.0;
        }

        // bilan periodique en test de charge
        if self.opts.bench.is_some() {
            self.report_t += dt;
            if self.report_t >= 2.0 {
                self.report_t = 0.0;
                let s = &ctx.stats;
                log::info!(
                    "[charge] {} ennemis | {:.0} ips ({:.2} ms) | sim {:.2} ms | dessin {:.2} ms | rendu {:.2} ms | {} sprites",
                    self.world.horde.len(),
                    s.fps,
                    s.frame_ms,
                    s.tick_ms,
                    s.draw_ms,
                    s.render_ms,
                    s.instances
                );
            }
        }
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        if self.coop_run() {
            self.coop_tick(ctx);
            return;
        }
        if self.state.frozen() {
            return;
        }
        let input = self.read_input(ctx);
        // les appuis accumules pendant un menu ne doivent pas declencher de dash au retour
        self.suppress = false;
        self.world.tick(&input);
        self.on_events(ctx);
        self.bench_refill();
        if matches!(self.state, State::Playing) {
            self.check_pending(ctx);
        }
    }

    fn draw(&mut self, ctx: &Ctx, alpha: f32, list: &mut DrawList) -> Frame {
        crate::hud::set_text_time(ctx.real_time as f32);
        let f = self.draw_scene(ctx, alpha, list);
        self.draw_cursor(ctx, list);
        f
    }

    /// Android en arriere-plan : le systeme peut fermer l'appli sans prevenir.
    fn suspend(&mut self, ctx: &mut Ctx) {
        if RUN_CHECKPOINTS {
            self.save_run_checkpoint(ctx);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx) {
        // fenetre fermee en pleine partie : comptee comme un abandon (rubis en banque, bestiaire,
        // statistiques), comme avec « Ecran titre ». Avant, toute la partie etait perdue.
        if !self.opts.scripted {
            let root = ctx.root.clone();
            self.commit_run(&root, false);
        }
        if self.opts.bench.is_some() {
            let s = &ctx.stats;
            println!(
                "BILAN charge : {} ennemis, {:.0} ips (image {:.2} ms), sim {:.2} ms/pas, dessin {:.2} ms, rendu {:.2} ms, {} sprites, x{} ({}x{})",
                self.world.horde.len(),
                s.fps,
                s.frame_ms,
                s.tick_ms,
                s.draw_ms,
                s.render_ms,
                s.instances,
                ctx.scale,
                ctx.view.x,
                ctx.view.y
            );
        }
        if self.opts.demo || self.opts.bench.is_some() {
            let w = &self.world;
            let names: Vec<String> = w
                .weapons
                .iter()
                .map(|s| format!("{} {}", self.content.weapon_ids[s.id as usize], s.level))
                .chain(w.passives.iter().map(|s| format!("{} {}", self.content.passive_ids[s.id as usize], s.level)))
                .collect();
            println!(
                "BILAN partie : {:.0} s, niveau {}, {} ennemis vaincus, vie {:.0}/{:.0}, ralentis {}/{} (accordes/refuses) | {}",
                w.time,
                w.player.level,
                w.player.kills,
                w.player.hp,
                w.player.max_hp,
                self.director.stats.0,
                self.director.stats.1,
                names.join(", ")
            );
        }
    }
}

/// Rubis rouges (monnaie de la boutique) mis en banque pour `rupees` ramasses : un quart, x1.5 en cas
/// de victoire, puis les bonus (difficulte, sceaux, boutique).
pub fn bank_amount(rupees: u32, won: bool, mult: f32) -> u64 {
    let base = (rupees as u64 / 4) * if won { 3 } else { 2 } / 2;
    (base as f32 * mult).round() as u64
}

/// Clignotement retro d'un objet qui va disparaitre : vrai une image sur deux (environ 8 fois
/// par seconde) pendant les `window` dernieres secondes de sa vie (`left` : temps restant).
fn expiring(left: f32, window: f32) -> bool {
    left < window && left > 0.0 && ((left * 16.0) as i32) % 2 == 1
}
