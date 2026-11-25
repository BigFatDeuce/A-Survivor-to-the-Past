//! Le jeu : relie la simulation, le rendu, l'audio, les menus et le juice.
use crate::chars;
use crate::content::{load_bindings, load_content, AudioConfig};
use crate::director::{Director, Moment};
use crate::hub;
use crate::hud::{Hud, GOLD};
use crate::juice::{Juice, NumKind};
use crate::lang::{Lang, LANGS};
use crate::menus::{self, Nav, NavIn, Opt, OptAction, OptValues};
use crate::meta::Meta;
use crate::profile::Profile;
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
use engine::draw::{rgb, with_alpha, Params, WHITE};
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
}

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
    /// Champion errant vaincu : recompense au choix.
    Reward { offers: Vec<Reward>, sel: usize, t: f32 },
    Dying(f32),
    Dead { t: f32, sel: usize },
    /// Choix du personnage, puis du stage (sur le decor du titre).
    CharSelect { sel: usize },
    StageSelect { sel: usize },
    /// Porte franchie : stage gagne.
    Victory { t: f32, sel: usize },
    /// Boutique (Kakariko, Grande Fee, reliques, defis).
    Shop { tab: usize, sel: usize },
    /// Profil (bestiaire, armurerie, statistiques, sauvegarde) ; `armed` : import a confirmer.
    Records { tab: usize, sel: usize, armed: bool },
    /// Commandes (depuis les options) ; `wait` : action en attente d'une touche et temps restant.
    Controls { tab: usize, sel: usize, wait: Option<(usize, f32)>, from_title: bool },
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
                | State::StageSelect { .. }
                | State::Shop { .. }
                | State::Records { .. }
                | State::Controls { from_title: true, .. }
                | State::Options { from_title: true, .. }
        )
    }
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
    meta: Meta,
    /// Message de la boutique / du profil et son temps restant (s).
    hub_toast: (String, f32),
}

/// Position des entrees fixes du menu titre (retour depuis un sous-ecran).
const TITLE_SHOP: usize = 1;
const TITLE_PROFILE: usize = 2;
const TITLE_OPTIONS: usize = 3;
const TITLE_ABOUT: usize = 4;
/// Fichier de repli de l'export du profil (presse-papiers indisponible).
const CODE_FILE: &str = "profile_code.txt";

/// Marque des elements de decor dans le tri par profondeur.
const PROP_BIT: u32 = 0x8000_0000;
const DUST: [f32; 4] = [0.86, 0.8, 0.58, 1.0];
const FIRE: [f32; 4] = [1.0, 0.6, 0.2, 1.0];
const ICE: [f32; 4] = [0.7, 0.88, 1.0, 1.0];
/// Couleur des champions errants (rencontres).
const CHAMPION: [f32; 4] = [0.85, 0.5, 1.0, 1.0];

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
            meta: Meta::load(&ctx.root),
            hub_toast: (String::new(), 0.0),
        };
        g.stage_ids = stages::stage_ids(&g.content);
        g.run_kills = vec![0; g.content.enemies.len()];
        if let Some(v) = g.opts.video {
            g.settings.video = v;
        }
        // essais (demo, charge, depart avance) : directement en partie, sinon ecran titre
        // couleurs choisies (profil, ou --skin pour les essais) : sprite du personnage
        let mut skins = g.profile.clone();
        if let Some(k) = g.opts.skin {
            skins.skins.insert(g.opts.character.clone(), k);
        }
        chars::apply_skins(&mut g.content, &skins);
        let map = g.opts.map.clone();
        g.set_map(ctx, &map);
        let o = &g.opts;
        let direct = o.play || o.demo || o.bench.is_some() || o.start_time > 0.0 || !o.give.is_empty();
        g.state = match g.opts.menu.as_deref() {
            Some("title") => State::Title { sel: 0 },
            Some("about") => State::About { sel: 0, toast: 0.0 },
            Some("pause") => State::Paused { sel: 1 },
            Some("options") => State::Options { sel: 1, from_title: false },
            Some("title_options") => State::Options { sel: 1, from_title: true },
            Some("gameover") => State::Dead { t: 1.0, sel: 0 },
            Some("stages") => State::StageSelect { sel: 0 },
            Some("chars") => State::CharSelect { sel: g.char_sel() },
            Some("victory") => State::Victory { t: 3.0, sel: 0 },
            Some("shop") => State::Shop { tab: 0, sel: 0 },
            Some("profile") => State::Records { tab: 0, sel: 0, armed: false },
            Some("controls") => State::Controls { tab: 0, sel: 0, wait: None, from_title: true },
            _ if direct => State::Playing,
            _ => State::Title { sel: 0 },
        };
        if g.state.in_title() && !g.opts.uicheck && g.settings.updates != UpdateMode::Off {
            g.updater.check();
        }
        if g.state.in_title() {
            g.menu_t = if g.opts.menu.is_some() { 1.0 } else { 0.0 };
            let id = g.audio_cfg.menu_music.clone();
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
            rec.best_difficulty = rec.best_difficulty.max(p.difficulty as i32);
        }
        let banked = bank_amount(w.player.rupees, won, self.meta.bank_mult(p));
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
        self.last_chars = chars::newly_unlocked(&self.content, &before, &self.profile);
        self.profile.save(root);
        banked
    }

    /// Annonce les personnages debloques par la partie qui vient de finir.
    fn announce_chars(&mut self, ctx: &mut Ctx) {
        if let Some(id) = self.last_chars.first().cloned() {
            self.sfx(ctx, "char_unlock");
            let s = self.lang.get("ui.new_character").replace("{s}", &chars::name(&self.lang, &id));
            self.hud.banner(&s, GOLD, 4.5);
        }
    }

    /// Verification des placements : tous les ecrans, toutes les langues, plusieurs formats de vue.
    fn ui_check(&mut self, ctx: &mut Ctx) {
        use crate::uicheck::{Report, VIEWS};
        let real_view = ctx.view;
        let c = self.content.clone();
        // monde rempli au maximum : 6 armes, 6 objets, gros compteurs, boss
        let mut picks = 0;
        for k in 0..c.weapons.len() as u16 {
            if !c.weapons[k as usize].evolved && picks < 6 {
                self.world.apply_offer(Offer::NewWeapon(k));
                for _ in 0..8 {
                    self.world.apply_offer(Offer::UpgradeWeapon(k));
                }
                picks += 1;
            }
        }
        for k in 0..c.passives.len().min(6) as u16 {
            self.world.apply_offer(Offer::NewPassive(k));
        }
        let p = &mut self.world.player;
        p.level = 99;
        p.kills = 99_999;
        p.rupees = 9_999;
        self.world.time = 599.0;
        if let Some(h) = c.enemy_index("hinox") {
            let at = self.world.player.pos + Vec2::new(60.0, 0.0);
            let i = self.world.spawn_enemy(h, at);
            self.world.boss_uid = Some(self.world.horde.uid[i]);
        }
        let mut offers: Vec<Offer> = (0..c.weapons.len() as u16).map(Offer::NewWeapon).collect();
        offers.extend((0..c.passives.len() as u16).map(Offer::NewPassive));
        offers.extend((0..c.weapons.len() as u16).filter(|&k| self.world.weapon_level(k).is_some()).map(Offer::UpgradeWeapon));
        offers.extend([Offer::Heal, Offer::Rupees]);
        let evo = c.weapons.iter().enumerate().find_map(|(k, w)| {
            let e = w.evolution.as_ref()?;
            Some((k as u16, c.weapon_index(&e.into)?))
        });
        self.menu_t = 5.0;
        // profil fictif : premier stage gagne, le suivant vient de s'ouvrir
        if let Some(first) = self.stage_ids.first().cloned() {
            let r = self.profile.stages.entry(first).or_default();
            r.wins = 1;
            r.runs = 3;
            r.best_time = 512.0;
        }
        self.last_banked = 1234;
        self.last_unlocked = self.stage_ids.get(1).cloned();
        self.last_relics = self.meta.relics.iter().take(2).map(|r| r.id.clone()).collect();
        // boutique : un peu de tout (niveaux, benediction, relique equipee, Sceau actif)
        self.profile.rupees = 98765;
        if let Some(it) = self.meta.shop.first() {
            self.profile.shop.insert(it.id.clone(), it.max);
        }
        if let Some(it) = self.meta.shop.get(1) {
            self.profile.shop.insert(it.id.clone(), 2);
        }
        if let Some(b) = self.meta.blessings.first() {
            self.profile.blessings.insert(b.id.clone());
        }
        if let Some(r) = self.meta.relics.first() {
            self.profile.relics.insert(r.id.clone());
            self.profile.equipped.push(r.id.clone());
        }
        if let Some(s) = self.meta.seals.first() {
            self.profile.seals.insert(s.id.clone());
        }
        for (k, id) in self.content.enemy_ids.iter().enumerate() {
            if k % 2 == 0 {
                self.profile.bestiary.insert(id.clone(), 123456);
            }
        }
        for id in self.content.weapon_ids.iter().chain(&self.content.passive_ids).step_by(2) {
            self.profile.armory.insert(id.clone(), 99);
        }
        self.profile.totals.kills = 9_876_543;
        self.profile.totals.time = 360000.0;
        let mut rep = Report::default();
        for code in LANGS {
            self.lang = Lang::load(&ctx.root, code);
            let boss = self.lang.get("ui.boss").to_string();
            for &(w, h) in &VIEWS {
                let view = Vec2::new(w, h);
                ctx.view = view;
                self.world.view_half = view * 0.5;
                self.hud.banner(&boss, GOLD, 5.0);
                let mut screens: Vec<(String, State)> = vec![
                    ("titre".into(), State::Title { sel: 0 }),
                    ("titre_maj_dispo".into(), State::Title { sel: 3 }),
                    ("titre_maj_telechargement".into(), State::Title { sel: 0 }),
                    ("titre_maj_prete".into(), State::Title { sel: 3 }),
                    ("titre_maj_echec".into(), State::Title { sel: 0 }),
                    ("a_propos".into(), State::About { sel: 1, toast: 0.0 }),
                    ("a_propos_lien_ouvert".into(), State::About { sel: 0, toast: 1.0 }),
                    ("options_titre".into(), State::Options { sel: 0, from_title: true }),
                    ("jeu".into(), State::Playing),
                    ("pause".into(), State::Paused { sel: 0 }),
                    ("options".into(), State::Options { sel: 3, from_title: false }),
                    ("fin".into(), State::Dead { t: 2.0, sel: 0 }),
                ];
                for (k, chunk) in offers.chunks(3).enumerate() {
                    screens.push((format!("niveau_{k}"), State::LevelUp { offers: chunk.to_vec(), sel: 0, t: 1.0 }));
                }
                let ups: Vec<Offer> = offers.iter().copied().filter(|o| matches!(o, Offer::UpgradeWeapon(_))).take(3).collect();
                screens.push(("coffre".into(), State::Chest { reward: ChestReward { evolution: None, upgrades: ups }, big: false, t: 2.0 }));
                // recompenses de rencontre : toutes les reliques, benedictions et ameliorations
                let mut rw: Vec<Reward> = (0..self.meta.relics.len()).filter(|&k| self.meta.relics[k].encounter).map(Reward::Relic).collect();
                rw.extend(reward::BLESSINGS.iter().map(|&(s, v)| Reward::Blessing(s, v)));
                for slot in 0..self.world.weapons.len() {
                    let boost = sim::upgrades::WeaponBoost { amount: 1, cooldown: 0.15, ..Default::default() };
                    rw.push(Reward::Boost { slot, boost });
                }
                for (k, chunk) in rw.chunks(3).enumerate() {
                    screens.push((format!("recompense_{k}"), State::Reward { offers: chunk.to_vec(), sel: 0, t: 1.0 }));
                }
                // choix du stage (gagne, ouvert, verrouille, retour) et victoire
                for (k, name) in [(0usize, "stages_gagne"), (1, "stages_ouvert"), (2, "stages_verrouille")] {
                    screens.push((name.into(), State::StageSelect { sel: k }));
                }
                screens.push(("stages_retour".into(), State::StageSelect { sel: self.stage_ids.len() }));
                let nc = chars::roster(&self.content).len();
                for k in 0..=nc {
                    screens.push((format!("persos_{k}"), State::CharSelect { sel: k }));
                }
                screens.push(("persos_verrouille".into(), State::CharSelect { sel: nc.saturating_sub(1) }));
                screens.push(("victoire".into(), State::Victory { t: 3.0, sel: 0 }));
                for tab in 0..shop::TABS.len() {
                    screens.push((format!("boutique_{tab}"), State::Shop { tab, sel: 1 }));
                }
                screens.push(("boutique_retour".into(), State::Shop { tab: 0, sel: 99 }));
                for tab in 0..records::TABS.len() {
                    screens.push((format!("profil_{tab}"), State::Records { tab, sel: 1, armed: tab == 3 }));
                }
                for tab in 0..controls::TABS.len() {
                    let wait = Some((1, 1.0));
                    screens.push((format!("commandes_{tab}"), State::Controls { tab, sel: 1, wait, from_title: tab == 0 }));
                }
                screens.push(("commandes_reset".into(), State::Controls { tab: 0, sel: controls::RESET, wait: None, from_title: true }));
                if let Some(e) = evo {
                    screens.push(("evolution".into(), State::Chest { reward: ChestReward { evolution: Some(e), upgrades: Vec::new() }, big: true, t: 2.0 }));
                }
                for (name, st) in screens {
                    self.updater.force_status(match name.as_str() {
                        "titre_maj_dispo" => UpdateStatus::Available(update::Release::sample("10.20.30")),
                        "titre_maj_telechargement" => UpdateStatus::Downloading { version: "10.20.30".into(), done: 99, total: 100 },
                        "titre_maj_prete" => UpdateStatus::Ready("10.20.30".into()),
                        "titre_maj_echec" => UpdateStatus::Failed(String::new()),
                        _ => UpdateStatus::Idle,
                    });
                    self.state = st;
                    // personnage encore verrouille : texte d'indice et avancement
                    let kills = self.profile.totals.kills;
                    if name == "persos_verrouille" {
                        self.profile.totals.kills = 1234;
                    }
                    let mut list = DrawList::default();
                    list.probe = Some(Vec::new());
                    self.draw(ctx, 1.0, &mut list);
                    self.profile.totals.kills = kills;
                    rep.analyze(list.probe.as_deref().unwrap_or(&[]), view, ctx.atlas.font("main"), &name, code);
                }
            }
        }
        ctx.view = real_view;
        self.state = State::Title { sel: 0 };
        let text = rep.print();
        println!("{text}");
    }

    /// Retour a l'ecran titre (partie abandonnee).
    fn to_title(&mut self, ctx: &mut Ctx) {
        let root = ctx.root.clone();
        self.commit_run(&root, false);
        self.state = State::Title { sel: 0 };
        self.menu_t = 0.0;
        self.thunder = None;
        self.juice.clear();
        self.director.clear();
        ctx.audio.stop_music(0.3);
        let id = self.audio_cfg.menu_music.clone();
        self.play_music(ctx, &id, 0.8);
        ctx.audio.duck_music(1.0, 0.2);
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
    }

    fn restart(&mut self, ctx: &mut Ctx) {
        let root = ctx.root.clone();
        self.commit_run(&root, false);
        self.run_open = true;
        self.run_relics.clear();
        self.run_kills.iter_mut().for_each(|k| *k = 0);
        self.run_bosses = 0;
        self.run_elites = 0;
        self.run_chests = 0;
        self.thunder = None;
        ctx.audio.stop_ambience(0.5);
        self.world = Self::make_world(&self.content, &self.opts, ctx.view);
        self.apply_mods();
        self.juice.clear();
        self.hud = Hud::new();
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
                dash: self.world.ticks % 150 == 0,
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
        let mv = (mv.normalize_or_zero() + i.stick(false)).clamp_length_max(1.0);
        let attack = i.action(b, "attack");
        let dash = !self.suppress && i.action_pressed(b, "dash");
        let toggle_auto = !self.suppress && i.action_pressed(b, "toggle_auto");
        let aim = match i.last_device {
            Device::Mouse => {
                let cam = self.cam.unwrap_or(self.world.player.pos);
                let off = (Vec2::new(ctx.view.x, ctx.view.y) * ctx.scale as f32 * 0.5).round();
                let m = cam + (i.mouse_px - off) / ctx.scale as f32;
                Some(m - (self.world.player.pos - Vec2::new(0.0, 8.0)))
            }
            Device::Gamepad => {
                let r = i.stick(true);
                (r.length_squared() > 0.05).then_some(r)
            }
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
        let h = &w.horde;
        for i in 0..h.len() {
            let d = me - h.pos[i];
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
                Event::PlayerHurt { pos, .. } => {
                    self.sfx(ctx, "hurt");
                    self.juice.add_trauma(0.5, 0.75);
                    self.juice.hitstop(0.07);
                    self.juice.flash([0.9, 0.1, 0.1, 0.3], 0.25);
                    self.juice.debris(pos - Vec2::new(0.0, 8.0), Vec2::ZERO, 8, rgb(230, 40, 40), 100.0, false);
                    self.hud.heart_shake = 1.0;
                }
                Event::PlayerDied { pos } => {
                    let root = ctx.root.clone();
                    self.commit_run(&root, false);
                    self.announce_chars(ctx);
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
                Event::Slam { pos, radius, .. } => {
                    self.juice.ring(pos - Vec2::new(0.0, 2.0), 6.0, radius, 0.35, rgb(240, 220, 160));
                    let n = (radius / 8.0) as u32;
                    for k in 0..n {
                        let a = k as f32 / n as f32 * TAU;
                        let p = pos + Vec2::from_angle(a) * radius * 0.7;
                        self.juice.anim(self.vis.dust, p, 0.25);
                    }
                    self.juice.debris(pos, Vec2::ZERO, 10, DUST, 140.0, false);
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
                    self.sfx(ctx, "windup");
                }
                Event::EnemyShot { .. } => self.sfx(ctx, "enemy_shot"),
                Event::ShotBlast { pos, radius } => {
                    let sc = (radius / 18.0).clamp(1.0, 3.0).round();
                    self.juice.anim_ex(self.vis.explosion, pos - Vec2::new(0.0, 4.0), 0.36, 0.0, sc, WHITE, false);
                    self.juice.ring(pos - Vec2::new(0.0, 4.0), 4.0, radius, 0.3, rgb(255, 90, 60));
                    self.juice.add_trauma(0.2, 0.5);
                    self.sfx(ctx, "shot_blast");
                }
                Event::Revived { pos } => {
                    self.sfx(ctx, "fairy");
                    let s = self.lang.get("ui.fairy_revive").to_string();
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
                Event::BossDied { pos, last, .. } => {
                    self.run_bosses += 1;
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
                    self.announce_chars(ctx);
                    ctx.audio.stop_music(0.3);
                    self.play_music(ctx, "great_victory", 0.0);
                    self.state = State::Victory { t: 0.0, sel: 0 };
                }
                Event::ChestDropped { pos, big } => {
                    self.sfx(ctx, "chest_drop");
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
            while self.world.pending_rewards > 0 {
                self.world.pending_rewards -= 1;
                let o = reward::roll(&self.world, &self.meta, &self.profile, &self.run_relics, &mut self.rng);
                if let Some(id) = reward::apply(&mut self.world, &self.meta, o[0]) {
                    self.run_relics.push(id);
                }
            }
            return false;
        }
        if self.world.pending_levels > 0 {
            let offers = self.world.roll_offers();
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
            let big = self.world.pending_chests.remove(0);
            let reward = self.world.open_chest(big);
            self.state = State::Chest { reward, big, t: 0.0 };
            ctx.audio.duck_music(0.35, 0.2);
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
            (TitleItem::Shop, l.get("ui.shop").to_string()),
            (TitleItem::Profile, l.get("ui.profile").to_string()),
            (TitleItem::Options, l.get("ui.options").to_string()),
            (TitleItem::About, l.get("ui.about").to_string()),
        ];
        let upd = match self.updater.status() {
            UpdateStatus::Available(r) => Some(l.get("ui.update.install").replace("{v}", &r.version)),
            UpdateStatus::Downloading { done, total, .. } => {
                let p = if total > 0 { (done * 100 / total).min(99) } else { 0 };
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
            Opt::Controls | Opt::Back => {}
        }
        self.sfx(ctx, "cursor");
    }

    /// Logique des menus (temps reel).
    fn update_shop(&mut self, ctx: &mut Ctx, nav: &NavIn, mut tab: usize, mut sel: usize) -> State {
        let ntabs = shop::TABS.len();
        let (rows, _) = shop::rows(&self.meta, &self.profile, &self.lang, &ctx.atlas, tab);
        let n = rows.len();
        let input = hub::input(nav, ctx.view, ntabs, n, &mut sel);
        let side = match input {
            hub::Input::Left(k) => Some((k, -1)),
            hub::Input::Right(k) => Some((k, 1)),
            _ => None,
        };
        if let Some((k, dir)) = side {
            if k < n && shop::row_uses_sides(tab, k) {
                self.shop_act(ctx, tab, k, dir);
            } else {
                tab = (tab as i32 + dir).rem_euclid(ntabs as i32) as usize;
                sel = 0;
                self.sfx(ctx, "cursor");
            }
            return State::Shop { tab, sel };
        }
        match input {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                return State::Title { sel: TITLE_SHOP };
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                if k != tab {
                    tab = k;
                    sel = 0;
                    self.sfx(ctx, "cursor");
                }
            }
            hub::Input::Pick(k) => self.shop_act(ctx, tab, k, 0),
            _ => {}
        }
        State::Shop { tab, sel }
    }

    fn shop_act(&mut self, ctx: &mut Ctx, tab: usize, k: usize, dir: i32) {
        match shop::act(&self.meta, &mut self.profile, tab, k, dir) {
            shop::Outcome::Done => {
                self.sfx(ctx, if tab == 0 || tab == 1 { "buy" } else { "equip" });
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
        let ntabs = records::TABS.len();
        let (rows, _) = records::rows(&self.content, &self.vis, &self.profile, &self.lang, tab, armed);
        let n = rows.len();
        let input = hub::input(nav, ctx.view, ntabs, n, &mut sel);
        if !matches!(input, hub::Input::None | hub::Input::Pick(_)) {
            armed = false;
        }
        match input {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                return State::Title { sel: TITLE_PROFILE };
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                if k != tab {
                    tab = k;
                    sel = 0;
                    self.sfx(ctx, "cursor");
                }
            }
            hub::Input::Left(_) | hub::Input::Right(_) => {
                let dir = if matches!(input, hub::Input::Left(_)) { -1 } else { 1 };
                tab = (tab as i32 + dir).rem_euclid(ntabs as i32) as usize;
                sel = 0;
                self.sfx(ctx, "cursor");
            }
            hub::Input::Pick(k) if tab == 3 && k == records::SAVE_EXPORT => {
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
            hub::Input::Pick(k) if tab == 3 && k == records::SAVE_IMPORT => {
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
                            self.profile = p;
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
        match hub::input(nav, ctx.view, controls::TABS.len(), n, &mut sel) {
            hub::Input::Back => {
                self.sfx(ctx, "confirm");
                let sel = menus::OPTIONS.iter().position(|o| *o == Opt::Controls).unwrap_or(0);
                return State::Options { sel, from_title };
            }
            hub::Input::Moved => self.sfx(ctx, "cursor"),
            hub::Input::Tab(k) => {
                if k != tab {
                    tab = k;
                    sel = 0;
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

    fn draw_hub(&self, ctx: &Ctx, list: &mut DrawList) {
        let lang = &self.lang;
        let (title, tabs, tab, sel, rows, details, corner) = match self.state {
            State::Shop { tab, sel } => {
                let (r, d) = shop::rows(&self.meta, &self.profile, lang, &ctx.atlas, tab);
                let corner = Some((self.vis.icon_rupees, self.profile.rupees.to_string()));
                (lang.get("ui.shop"), &shop::TABS[..], tab, sel, r, d, corner)
            }
            State::Records { tab, sel, armed } => {
                let (r, d) = records::rows(&self.content, &self.vis, &self.profile, lang, tab, armed);
                (lang.get("ui.profile"), &records::TABS[..], tab, sel, r, d, None)
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
    }

    fn update_menus(&mut self, ctx: &mut Ctx, nav: &NavIn, dt: f32) {
        let view = ctx.view;
        self.hub_toast.1 = (self.hub_toast.1 - dt).max(0.0);
        let pause = ctx.input.action_pressed(&self.bindings, "pause");
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
                        State::CharSelect { sel: self.char_sel() }
                    }
                    Some(TitleItem::Shop) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        State::Shop { tab: 0, sel: 0 }
                    }
                    Some(TitleItem::Profile) => {
                        self.sfx(ctx, "confirm");
                        self.hub_toast.1 = 0.0;
                        State::Records { tab: 0, sel: 0, armed: false }
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
            State::CharSelect { mut sel } => {
                let ids = chars::roster(&self.content);
                let n = ids.len();
                let (pick, moved) = chars::input_select(nav, view, n, &mut sel);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                // gauche / droite : couleur du personnage (s'il en propose plusieurs)
                if (nav.left || nav.right) && sel < n {
                    let id = ids[sel].clone();
                    let def = &self.content.characters[&id];
                    let m = def.skins.len();
                    if m > 1 && chars::unlocked(def, &self.profile) {
                        let k = chars::skin(def, &self.profile, &id);
                        let k = if nav.right { (k + 1) % m } else { (k + m - 1) % m };
                        self.profile.skins.insert(id, k);
                        self.profile.save(&ctx.root);
                        chars::apply_skins(&mut self.content, &self.profile);
                        self.sfx(ctx, "cursor");
                    }
                }
                if nav.back {
                    self.sfx(ctx, "confirm");
                    State::Title { sel: 0 }
                } else {
                    match pick {
                        Some(k) if k >= n => {
                            self.sfx(ctx, "confirm");
                            State::Title { sel: 0 }
                        }
                        Some(k) if chars::unlocked(&self.content.characters[&ids[k]], &self.profile) => {
                            self.sfx(ctx, "confirm");
                            self.opts.character = ids[k].clone();
                            if self.profile.character != ids[k] {
                                self.profile.character = ids[k].clone();
                                self.profile.save(&ctx.root);
                            }
                            let map = self.opts.map.clone();
                            self.set_map(ctx, &map);
                            State::StageSelect { sel: stages::suggested(&self.stage_ids, &self.profile) }
                        }
                        Some(_) => {
                            self.sfx(ctx, "error");
                            State::CharSelect { sel }
                        }
                        None => State::CharSelect { sel },
                    }
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
                    State::CharSelect { sel: self.char_sel() }
                } else {
                    match pick {
                        Some(k) if k >= n => {
                            self.sfx(ctx, "confirm");
                            State::CharSelect { sel: self.char_sel() }
                        }
                        Some(k) if stages::unlocked(&self.stage_ids, &self.profile, k) => {
                            self.sfx(ctx, "confirm");
                            let id = self.stage_ids[k].clone();
                            self.set_map(ctx, &id);
                            ctx.audio.stop_music(0.6);
                            self.restart(ctx);
                            return;
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
                        Some("ui.stage_select") => {
                            self.sfx(ctx, "confirm");
                            self.to_title(ctx);
                            let k = stages::suggested(&self.stage_ids, &self.profile);
                            State::StageSelect { sel: k }
                        }
                        Some(_) => {
                            self.sfx(ctx, "confirm");
                            self.to_title(ctx);
                            return;
                        }
                        None => State::Victory { t, sel },
                    }
                }
            }
            State::Shop { tab, sel } => self.update_shop(ctx, nav, tab, sel),
            State::Records { tab, sel, armed } => self.update_records(ctx, nav, tab, sel, armed),
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
                let (pick, moved) = menus::input_pause(nav, view, &mut sel);
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
                    Some(2) => {
                        self.restart(ctx);
                        return;
                    }
                    Some(_) => {
                        self.sfx(ctx, "confirm");
                        self.to_title(ctx);
                        return;
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
                        State::Controls { tab, sel: 0, wait: None, from_title }
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
                let (mut pick, moved) = menus::input_levelup(nav, view, offers.len(), &mut sel, t);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if self.opts.demo && t > 1.2 {
                    pick = Some(0);
                }
                match pick {
                    Some(k) => {
                        self.world.apply_offer(offers[k]);
                        self.world.pending_levels = self.world.pending_levels.saturating_sub(1);
                        self.sfx(ctx, "pick");
                        self.juice.flash([1.0, 1.0, 1.0, 0.25], 0.12);
                        self.resume(ctx);
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
                if self.opts.demo && t > 1.2 {
                    pick = Some(0);
                }
                match pick {
                    Some(k) => {
                        if let Some(id) = reward::apply(&mut self.world, &self.meta, offers[k]) {
                            self.run_relics.push(id);
                        }
                        self.world.pending_rewards = self.world.pending_rewards.saturating_sub(1);
                        self.sfx(ctx, "reward");
                        self.juice.flash([1.0, 0.85, 1.0, 0.3], 0.15);
                        self.resume(ctx);
                        return;
                    }
                    None => State::Reward { offers, sel, t },
                }
            }
            State::Chest { reward, big, t } => {
                let t2 = t + dt;
                if t < menus::CHEST_OPEN && t2 >= menus::CHEST_OPEN {
                    self.sfx(ctx, "chest_open");
                    if reward.evolution.is_some() {
                        self.sfx(ctx, "evolution");
                    }
                }
                let go = t2 >= menus::CHEST_DONE && (nav.confirm || nav.click || (self.opts.demo && t2 > menus::CHEST_DONE + 0.5));
                if go {
                    self.sfx(ctx, "confirm");
                    self.resume(ctx);
                    return;
                }
                State::Chest { reward, big, t: t2 }
            }
            State::Dying(t) => {
                let t = t + dt;
                if t > 1.8 {
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
                        Some(0) => {
                            self.sfx(ctx, "confirm");
                            self.restart(ctx);
                            return;
                        }
                        Some(_) => {
                            self.sfx(ctx, "confirm");
                            self.to_title(ctx);
                            return;
                        }
                        None => State::Dead { t, sel },
                    }
                }
            }
        };
    }

    /// Meteo, teinte de la carte et porte de sortie (apres le monde, avant l'interface).
    fn draw_stage_fx(&self, ctx: &Ctx, alpha: f32, list: &mut DrawList, cam: Vec2) {
        let w = &self.world;
        let white = self.vis.white;
        if let Some(at) = w.portal {
            weather::draw_portal(list, ctx, &self.weather_vis, &white, at, w.portal_t, ctx.real_time as f32);
        }
        let pp = w.player.prev.lerp(w.player.pos, alpha) - Vec2::new(0.0, 8.0);
        let ui = pp - cam + ctx.view * 0.5;
        weather::draw(list, ctx, &self.weather_vis, &white, w.map.weather, w.map.tint, cam, ui, w.time);
        // fleche vers la porte quand elle est hors champ
        if let Some(at) = w.portal {
            let view = ctx.view;
            let p = at - cam + view * 0.5;
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
    fn char_sel(&self) -> usize {
        let ids = chars::roster(&self.content);
        let want = if self.profile.character.is_empty() { &self.opts.character } else { &self.profile.character };
        ids.iter()
            .position(|id| id == want && chars::unlocked(&self.content.characters[id], &self.profile))
            .unwrap_or(0)
    }

    fn next_stage(&self) -> Option<String> {
        let k = self.stage_ids.iter().position(|s| *s == self.world.map_id)?;
        let id = self.stage_ids.get(k + 1)?;
        stages::unlocked(&self.stage_ids, &self.profile, k + 1).then(|| id.clone())
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
                    list.frame(&pframe, pp, &prm);
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
            if Some(k as u16) == w.bomb_kind {
                let beat = 0.5 + 0.5 * (rt * 7.0 + phase).sin();
                // anneau au sol qui pulse
                let r = 11.0 + 3.0 * beat;
                for s in 0..14 {
                    let a = s as f32 / 14.0 * TAU + rt;
                    let q = pos + Vec2::new(a.cos() * r, a.sin() * r * 0.45);
                    list.rect(&white, q.round(), Vec2::ONE, [1.0, 0.45, 0.15, 0.5 + 0.4 * beat]);
                }
                list.set(Space::World, Blend::Add);
                let halo = Params { scale: prm.scale * 1.3, ..prm }.flash([1.0, 0.22, 0.05], 1.0).color([1.0, 1.0, 1.0, 0.16 + 0.24 * beat]);
                list.frame(f, pos + Vec2::new(0.0, 2.0), &halo);
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
            let master = self.content.weapons[sw.weapon as usize].evolved;
            let t_r = (sw.t - DT * (1.0 - alpha)).max(0.0);
            let blade = atlas.frame(if master { self.vis.player.weapon_master } else { self.vis.player.weapon }, 0.0);
            let hand = pp - Vec2::new(0.0, 9.0);
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

        // chaines du grappin (sous la tete)
        let hand = pp - Vec2::new(0.0, 8.0);
        for p in &w.projectiles {
            if p.beam || !self.vis.weapons[p.weapon as usize].chain {
                continue;
            }
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

        // objets en orbite (Canne de Byrna, fees)
        for (slot, ws) in w.weapons.iter().enumerate() {
            let def = &self.content.weapons[ws.id as usize];
            if def.kind != WeaponKind::Orbit || ws.active <= 0.0 {
                continue;
            }
            let Some(anim) = self.vis.weapons[ws.id as usize].sprite else { continue };
            let phase = ws.phase - ws.stats.speed * DT * (1.0 - alpha);
            let a = (ws.active / 0.3).min(1.0);
            for (k, q) in w.orbit_points(slot, pp - Vec2::new(0.0, 6.0), phase).into_iter().enumerate() {
                let f = atlas.frame(anim, wt + k as f32 * 0.13);
                list.frame(f, q, &Params::default().color([1.0, 1.0, 1.0, a]));
            }
        }

        // poudre magique : nuage scintillant autour de Link
        for ws in &w.weapons {
            let def = &self.content.weapons[ws.id as usize];
            if def.kind != WeaponKind::Aura {
                continue;
            }
            let Some(anim) = self.vis.weapons[ws.id as usize].sprite else { continue };
            let r = ws.stats.area;
            let n = ((r * 0.7) as usize).clamp(12, 48);
            let tint = if def.evolved { [1.0, 0.7, 1.0] } else { [1.0, 1.0, 1.0] };
            list.set(Space::World, Blend::Add);
            // bord du nuage : anneau pointille qui tourne lentement (portee lisible)
            let ring = ((r * 0.45) as usize).clamp(20, 72);
            let edge = if def.evolved { [1.0, 0.45, 0.25] } else { [0.75, 0.6, 1.0] };
            for s in 0..ring {
                let a = s as f32 / ring as f32 * TAU + rt * 0.4;
                let q = pp - Vec2::new(0.0, 2.0) + Vec2::new(a.cos() * r, a.sin() * r * 0.6);
                let al = 0.45 + 0.25 * (rt * 2.0 + s as f32 * 0.7).sin();
                list.rect(&white, q.round(), Vec2::splat(2.0), [edge[0], edge[1], edge[2], al]);
            }
            for k in 0..n {
                let kf = k as f32;
                let rr = r * (0.3 + 0.7 * (kf * 0.618).fract());
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let a = kf * 2.399 + rt * (0.7 + 0.5 * (kf * 0.37).fract()) * dir;
                let q = pp - Vec2::new(0.0, 6.0) + Vec2::new(a.cos(), a.sin() * 0.6) * rr;
                let f = atlas.frame(anim, rt + kf * 0.17);
                let al = 0.7 + 0.3 * (rt * 3.0 + kf).sin().abs();
                // un grain sur trois en x2 : le nuage se lit mieux dans la foule
                let sc = if k % 3 == 0 { 2.0 } else { 1.0 };
                list.frame(f, q.round(), &Params { scale: Vec2::splat(sc), ..Params::default() }.color([tint[0], tint[1], tint[2], al]));
            }
            list.set(Space::World, Blend::Alpha);
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
            if s.lob {
                // bombe du boss : clignote au sol avant d'exploser
                if s.age >= s.flight && (s.age * 14.0).fract() < 0.5 {
                    prm = prm.flash([1.0, 0.3, 0.2], 0.7);
                }
                list.frame(f, pos - Vec2::new(0.0, s.z.round()), &prm);
            } else {
                list.frame(f, pos, &prm);
            }
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

    /// Fleche d'indication (sprite de la police d'A Link to the Past) a cote d'un repere au bord de l'ecran :
    /// tournee d'un quart de tour vers la direction dominante (pixels nets), elle avance et recule un peu.
    fn edge_arrow(&self, ctx: &Ctx, list: &mut DrawList, c: Vec2, d: Vec2, dist: f32) {
        let card = if d.x.abs() >= d.y.abs() { Vec2::new(d.x.signum(), 0.0) } else { Vec2::new(0.0, d.y.signum()) };
        let rot = card.y.atan2(card.x);
        let push = (2.0 * (ctx.real_time as f32 * 5.0).sin().abs()).round();
        let at = (c + card * (dist + push)).round();
        list.frame(ctx.atlas.frame(self.vis.ui_arrow, 0.0), at, &Params { rot, ..Default::default() });
    }

    /// Fleches au bord de l'ecran vers les coffres et la Bob-omb hors champ.
    fn draw_chest_markers(&self, ctx: &Ctx, list: &mut DrawList, cam: Vec2) {
        let view = ctx.view;
        let pk = &self.world.pickups;
        list.set(Space::Ui, Blend::Alpha);
        let h = &self.world.horde;
        if let (Some(e), Some(cp)) = (self.world.encounter.as_ref(), self.world.champion_pos()) {
            let ui = cp - cam + view * 0.5;
            let m = 22.0;
            let on_screen = ui.x >= 0.0 && ui.y >= 0.0 && ui.x <= view.x && ui.y <= view.y;
            let blink = (ctx.real_time * 3.0).fract() < 0.65;
            let font = ctx.atlas.font("main");
            if !on_screen {
                // a l'ecart du HUD : coeurs et armes en haut, mode d'attaque en bas
                let c = Vec2::new(ui.x.clamp(m + 18.0, view.x - m - 18.0), ui.y.clamp(m + 74.0, view.y - 56.0));
                if let Some(i) = h.uid.iter().position(|&u| u == e.uid) {
                    let (anim, _) = self.vis.enemies[h.kind[i] as usize].pick(FACE_DOWN, 0.0, false);
                    list.frame(ctx.atlas.frame(anim, ctx.real_time as f32), c + Vec2::new(0.0, 8.0), &Params::default().color([1.0, 1.0, 1.0, if blink { 1.0 } else { 0.7 }]));
                }
                let d = (ui - c).normalize_or_zero();
                self.edge_arrow(ctx, list, c, d, 20.0);
                // distance (en metres de jeu : 16 pixels) et temps restant
                let dist = ((cp - self.world.player.pos).length() / 16.0).round() as i32;
                let mut s = format!("{dist}m");
                if !e.engaged {
                    s = format!("{s} {}s", e.t.ceil() as i32);
                }
                // texte du cote oppose a la fleche
                let ty = if d.y > 0.4 { -36.0 } else { 16.0 };
                crate::hud::text(list, font, &s, c + Vec2::new(0.0, ty), 1.0, CHAMPION, engine::draw::Align::Center);
            } else if !e.engaged {
                let s = format!("{}s", e.t.ceil() as i32);
                crate::hud::text(list, font, &s, ui + Vec2::new(0.0, 6.0), 1.0, with_alpha(CHAMPION, if blink { 1.0 } else { 0.7 }), engine::draw::Align::Center);
            }
        }
        if let Some(bk) = self.world.bomb_kind {
            for i in 0..h.len() {
                if h.kind[i] != bk {
                    continue;
                }
                let ui = h.pos[i] - cam + view * 0.5;
                let m = 18.0;
                if ui.x >= 0.0 && ui.y >= 0.0 && ui.x <= view.x && ui.y <= view.y {
                    continue;
                }
                let c = Vec2::new(ui.x.clamp(m, view.x - m), ui.y.clamp(m + 60.0, view.y - m));
                let (anim, _) = self.vis.enemies[bk as usize].pick(FACE_DOWN, 0.0, false);
                let blink = (ctx.real_time * 4.0).fract() < 0.6;
                list.frame(ctx.atlas.frame(anim, ctx.real_time as f32), c + Vec2::new(0.0, 10.0), &Params::default().color([1.0, 1.0, 1.0, if blink { 1.0 } else { 0.6 }]));
                let d = (ui - c).normalize_or_zero();
                self.edge_arrow(ctx, list, c, d, 18.0);
            }
        }
        for i in 0..pk.len() {
            if !pk.kind[i].is_chest() {
                continue;
            }
            let ui = pk.pos[i] - cam + view * 0.5;
            let m = 18.0;
            if ui.x >= 0.0 && ui.y >= 0.0 && ui.x <= view.x && ui.y <= view.y {
                continue;
            }
            let c = Vec2::new(ui.x.clamp(m, view.x - m), ui.y.clamp(m + 60.0, view.y - m));
            let blink = (ctx.real_time * 3.0).fract() < 0.7;
            let anim = self.vis.chest[(pk.kind[i] == PickupKind::BigChest) as usize];
            list.frame(ctx.atlas.frame(anim, 0.0), c + Vec2::new(0.0, 6.0), &Params::default().color([1.0, 1.0, 1.0, if blink { 1.0 } else { 0.6 }]));
            // fleche orientee vers le coffre
            let d = (ui - c).normalize_or_zero();
            self.edge_arrow(ctx, list, c, d, 16.0);
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
            State::Title { .. } | State::About { .. } => {}
            State::Paused { sel } => menus::draw_pause(list, ctx, &self.vis, lang, *sel),
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
            State::Reward { offers, sel, t } => {
                let infos: Vec<_> = offers.iter().map(|&o| reward::info(&self.world, &self.vis, &ctx.atlas, lang, &self.meta, o)).collect();
                menus::draw_cards(list, ctx, &self.vis, lang, (lang.get("ui.reward.title"), lang.get("ui.choose"), CHAMPION), &infos, *sel, *t);
            }
            State::Dying(t) => {
                let a = (t / 1.8).min(1.0) * 0.55;
                list.rect(white, Vec2::ZERO, view, [0.25, 0.0, 0.0, a]);
            }
            State::Dead { t, sel } => menus::draw_dead(list, ctx, &self.world, &self.vis, lang, *t, *sel),
            State::Victory { t, sel } => {
                let items = stages::victory_items(self.next_stage().is_some());
                let mut notes = Vec::new();
                if let Some(id) = &self.last_unlocked {
                    notes.push(lang.get("ui.new_stage").replace("{s}", &stages::stage_name(lang, id)));
                }
                for id in &self.last_relics {
                    notes.push(lang.get("ui.relic_found").replace("{s}", lang.get(&format!("relic.{id}.name"))));
                }
                for id in &self.last_chars {
                    notes.push(lang.get("ui.new_character").replace("{s}", &chars::name(lang, id)));
                }
                stages::draw_victory(list, ctx, &self.world, lang, *t, *sel, &items, self.last_banked, &notes);
            }
            State::CharSelect { .. } | State::StageSelect { .. } | State::Shop { .. } | State::Records { .. } => {}
            State::Controls { .. } => self.draw_hub(ctx, list),
            State::Playing => {
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

impl GameTrait for Game {
    fn frame(&mut self, ctx: &mut Ctx, dt: f32) {
        self.frame_dt = dt;
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
        if self.opts.uicheck {
            self.opts.uicheck = false;
            self.ui_check(ctx);
            ctx.quit();
            return;
        }
        let nav = self.nav.read(ctx, &self.bindings, dt);
        self.menu_t += dt;
        self.poll_updates();
        self.update_menus(ctx, &nav, dt);
        self.title_ambience(ctx);

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
        self.juice.update(dt * ctx.time_scale, dt);
        self.hud.update(dt, &self.world);
        self.rupee_chain_t -= dt * ctx.time_scale;
        if self.rupee_chain_t <= 0.0 {
            self.rupee_chain = 0;
        }
        self.world.view_half = ctx.view * 0.5;

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
        if self.state.in_title() {
            let white = self.vis.white;
            match &self.state {
                State::About { sel, toast } => {
                    title::draw_about(list, ctx, &self.title_vis, &self.lang, &white, &self.links, *sel, self.menu_t, *toast)
                }
                State::Title { sel } => {
                    let items = self.title_items();
                    title::draw_title(list, ctx, &self.title_vis, &self.lang, &white, &items, *sel, self.menu_t)
                }
                State::Shop { .. } | State::Records { .. } | State::Controls { .. } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    self.draw_hub(ctx, list);
                }
                State::CharSelect { sel } => {
                    title::draw_backdrop(list, ctx, &self.title_vis, self.menu_t);
                    let ids = chars::roster(&self.content);
                    chars::draw_select(list, ctx, &self.lang, &self.content, &self.profile, &ids, *sel, self.menu_t);
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
            return Frame { camera: Vec2::ZERO, clear: [0.0, 0.0, 0.0] };
        }
        let w = &self.world;
        let pp = w.player.prev.lerp(w.player.pos, alpha) - Vec2::new(0.0, 8.0);
        let cam = match self.cam {
            None => pp,
            Some(c) => c + (pp - c) * (1.0 - (-self.frame_dt * 12.0).exp()),
        };
        self.cam = Some(cam);
        let cam_shaken = cam + self.juice.shake;
        self.draw_world(ctx, alpha, list, cam_shaken);
        self.juice.draw_over(list, &ctx.atlas, &self.vis.white);
        self.draw_stage_fx(ctx, alpha, list, cam_shaken);
        self.juice.draw_numbers(list, &ctx.atlas);
        self.draw_chest_markers(ctx, list, cam_shaken);
        self.hud.banked = bank_amount(self.world.player.rupees, false, self.meta.bank_mult(&self.profile));
        self.hud.draw(list, ctx, &self.world, &self.vis, &self.lang);
        self.draw_overlays(ctx, list);
        Frame { camera: cam_shaken, clear: [0.0, 0.0, 0.0] }
    }

    fn exit(&mut self, ctx: &mut Ctx) {
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
