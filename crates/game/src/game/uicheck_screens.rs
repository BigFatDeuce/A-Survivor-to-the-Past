//! Verification des placements (`--uicheck`) : remplit le monde au maximum, puis dessine chaque
//! ecran dans toutes les langues et plusieurs formats de vue, et signale chevauchements, textes
//! qui depassent et caracteres manquants. Sorti de game.rs (outil de developpement).
use super::*;

impl Game {
    /// Verification des placements : tous les ecrans, toutes les langues, plusieurs formats de vue.
    pub(super) fn ui_check(&mut self, ctx: &mut Ctx) {
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
        // pire cas de l'ecran de victoire : toutes les reliques et beaucoup de personnages d'un coup
        self.last_relics = self.meta.relics.iter().map(|r| r.id.clone()).collect();
        self.last_chars = chars::roster(&self.content).into_iter().take(8).collect();
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
        // joueurs et classements : noms les plus larges, scores sur le premier stage, classement mondial plein
        self.profile.name = "WWWWWWWWWWWW".into();
        self.profile.avatar = 19;
        let first = self.stage_ids.first().cloned().unwrap_or_default();
        for k in 0..profiles::KEEP_SCORES {
            let rec = ScoreRecord {
                score: 9_876_543 - k as u64 * 1000,
                time: 3599.0,
                kills: 99_999,
                level: 99,
                bosses: 9,
                won: k % 2 == 0,
                character: self.opts.character.clone(),
                difficulty: 3,
                mult: 2.5,
                kw: Some(3.0),
                version: "10.20.30".into(),
                published: k == 1,
                team: Vec::new(),
            };
            self.profile.add_score(&first, rec);
        }
        self.last_score = Some((9_876_543, true));
        self.slots = (0..profiles::SLOTS)
            .map(|k| {
                let mut p = Profile::fresh(k, "WWWWWWWWWWWW", k * 3);
                p.scores = self.profile.scores.clone();
                p.totals.time = 360000.0;
                p.totals.kills = 9_876_543;
                Some(p)
            })
            .collect();
        let entries = (1..=100)
            .map(|r| online::Entry {
                rank: r,
                name: "WWWWWWWWWWWW".into(),
                score: 99_999_999 - r as u64,
                character: self.opts.character.clone(),
                time: 3599.0,
                kills: 99_999,
                won: true,
                difficulty: 3,
                // titres longs (tous les niveaux) dans le detail
                title: r % 4,
            })
            .collect();
        let board = online::Board { version: "10.20.30".into(), versions: vec!["10.20.30".into(), "10.20.29".into()], scores: entries };
        // coop a 4 : pseudos les plus larges, tous les personnages
        let team = vec!["WWWWWWWWWWWW".to_string(); 4];
        let team_chars = [self.opts.character.as_str(); 4].join(",");
        let mut coop_board = board.clone();
        for e in &mut coop_board.scores {
            e.name = team.join(crate::scores::TEAM_SEP);
            e.character = team_chars.clone();
        }
        for k in 0..profiles::KEEP_SCORES {
            let rec = ScoreRecord { score: 99_999_999 - k as u64, time: 3599.0, kills: 99_999, level: 99, bosses: 9, won: true, character: team_chars.clone(), difficulty: 3, mult: 2.5, kw: Some(3.0), version: "10.20.30".into(), published: k == 1, team: team.clone() };
            self.profile.add_score(&first, rec);
        }
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
                    ("titre_maj_dispo".into(), State::Title { sel: TITLE_UPDATE }),
                    ("titre_maj_telechargement".into(), State::Title { sel: 0 }),
                    ("titre_maj_prete".into(), State::Title { sel: TITLE_UPDATE }),
                    ("titre_maj_echec".into(), State::Title { sel: 0 }),
                    ("a_propos".into(), State::About { sel: 1, toast: 0.0 }),
                    ("a_propos_lien_ouvert".into(), State::About { sel: 0, toast: 1.0 }),
                    ("options_titre".into(), State::Options { sel: 0, from_title: true }),
                    ("jeu".into(), State::Playing),
                    ("pause".into(), State::Paused { sel: 0 }),
                    ("pause_bonus".into(), State::Paused { sel: 4 }),
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
                // 0.6.3 : sorciere (pire cas : deux bonus aux textes longs, prix a 4 chiffres) et diseuse
                use crate::fortune::Phase;
                use sim::defs::Stat;
                let long = crate::witch::Potion { color: 0, bonus: vec![(Stat::Cooldown, -0.14), (Stat::DashCooldown, -0.2)], malus: (Stat::Growth, -0.12), price: 9999 };
                let mut offer = vec![long];
                offer.extend(crate::witch::roll(&mut Rng::new(5), 2.35, 1.8, 120.0).into_iter().take(2));
                self.witch_offer = offer;
                self.fortune_played = [true, false];
                screens.push(("sorciere".into(), State::Witch { sel: 0, t: 1.0 }));
                screens.push(("sorciere_rien".into(), State::Witch { sel: 3, t: 1.0 }));
                screens.push(("diseuse".into(), State::Fortune { phase: Phase::Menu, sel: 1, t: 1.0 }));
                screens.push(("diseuse_mise".into(), State::Fortune { phase: Phase::Stake, sel: 0, t: 1.0 }));
                screens.push(("diseuse_coffres".into(), State::Fortune { phase: Phase::Chests { stake: 2000 }, sel: 1, t: 1.0 }));
                for (k, mults) in [[3, 1, 0], [0, 3, 1], [1, 0, 3]].into_iter().enumerate() {
                    screens.push((format!("diseuse_ouverts_{k}"), State::Fortune { phase: Phase::Opened { stake: 2000, mults, pick: 0 }, sel: 0, t: 3.0 }));
                }
                for win in [true, false] {
                    screens.push((format!("diseuse_double_{win}"), State::Fortune { phase: Phase::Double { win }, sel: 0, t: 4.0 }));
                }
                screens.push(("diseuse_boule".into(), State::Fortune { phase: Phase::Double { win: true }, sel: 0, t: 0.8 }));
                // choix du stage (gagne, ouvert, verrouille, retour) et victoire
                for (k, name) in [(0usize, "stages_gagne"), (1, "stages_ouvert"), (2, "stages_verrouille")] {
                    screens.push((name.into(), State::StageSelect { sel: k }));
                }
                screens.push(("stages_retour".into(), State::StageSelect { sel: self.stage_ids.len() }));
                // chaque personnage de chaque univers, puis Retour et le choix de l'univers
                for page in 0..chars::GROUPS.len() {
                    let nc = chars::page_roster(&self.content, page).len();
                    for k in 0..nc + 2 {
                        screens.push((format!("persos_{page}_{k}"), State::CharSelect { page, sel: k }));
                    }
                    // choix de la couleur (personnage valide) : clavier et souris
                    let ids = chars::page_roster(&self.content, page);
                    if let Some(k) = ids.iter().position(|id| self.content.characters[id].skins.len() > 1) {
                        screens.push((format!("persos_couleur_{page}"), State::CharSelect { page, sel: k }));
                        screens.push((format!("persos_couleur_souris_{page}"), State::CharSelect { page, sel: k }));
                    }
                }
                screens.push(("victoire".into(), State::Victory { t: 3.0, sel: 0 }));
                for tab in shop::SHOP {
                    screens.push((format!("boutique_{tab}"), State::Shop { tab, sel: 1 }));
                }
                for tab in shop::PREP {
                    screens.push((format!("preparatifs_{tab}"), State::Prep { tab, sel: hub::GO_ROW }));
                    screens.push((format!("preparatifs_{tab}_ligne"), State::Prep { tab, sel: 1 }));
                }
                screens.push(("boutique_retour".into(), State::Shop { tab: 0, sel: 99 }));
                for tab in 0..records::TABS.len() {
                    screens.push((format!("profil_{tab}"), State::Records { tab, sel: 1, armed: tab == records::TAB_SAVE }));
                }
                // fiches de la Collection : chaque arme, fusion, objet, trophee et ennemi
                for tab in [records::TAB_WEAPONS, records::TAB_FUSIONS, records::TAB_ITEMS, records::TAB_TROPHIES] {
                    for k in records::card_items(&c, &self.meta, &self.profile, tab).0 {
                        screens.push((format!("fiche_{tab}_{k}"), State::Records { tab, sel: 1, armed: false }));
                    }
                }
                for k in 0..c.enemies.len() {
                    screens.push((format!("fiche_0_{k}"), State::Records { tab: 0, sel: 1, armed: false }));
                    screens.push((format!("fiche_0_{k}_achat"), State::Records { tab: 0, sel: 1, armed: false }));
                }
                for k in 0..chars::roster(&c).len() {
                    screens.push((format!("fiche_{}_{k}", records::CARD_CHARS), State::Records { tab: records::TAB_GALLERY, sel: 0, armed: false }));
                }
                screens.push(("joueurs".into(), State::Players { sel: 1, armed: false }));
                screens.push(("joueurs_supprimer".into(), State::Players { sel: players::DELETE, armed: true }));
                screens.push(("nom".into(), State::NameEntry { purpose: NamePurpose::Rename, text: "WWWWWWWWWWWW".into(), sel: players::DONE }));
                screens.push(("nom_pseudo".into(), State::NameEntry { purpose: NamePurpose::Create(4), text: String::new(), sel: 0 }));
                screens.push(("nom_creation".into(), State::NameEntry { purpose: NamePurpose::Setup, text: "WWWWWWWWWWWW".into(), sel: 0 }));
                for wing in 0..gallery::kinds(&c).len() {
                    screens.push((format!("galerie_{wing}"), State::Gallery { wing, from_title: false }));
                }
                for sel in [0, 9, players::AVATARS.len() - 1] {
                    screens.push((format!("portraits_{sel}"), State::Avatars { sel, setup: true }));
                }
                for tab in 0..scores::TABS.len() {
                    screens.push((format!("classement_{tab}"), State::Scores { tab, sel: 1, stage: 0, version: String::new(), players: 1 }));
                }
                // coop a 4 : equipe dans le detail (Mes parties) et classement mondial a part
                for tab in [scores::MINE, scores::WORLD] {
                    screens.push((format!("classement_coop_{tab}"), State::Scores { tab, sel: scores::head(tab), stage: 0, version: String::new(), players: 4 }));
                }
                for tab in 0..controls::TABS.len() {
                    let wait = Some((1, 1.0));
                    screens.push((format!("commandes_{tab}"), State::Controls { tab, sel: 1, wait, from_title: tab == 0 }));
                }
                screens.push(("commandes_reset".into(), State::Controls { tab: 0, sel: controls::RESET, wait: None, from_title: true }));
                // fusion proposee et fusion ultime au choix de niveau : bandeau, recette, halo
                for (name, into) in [("niveau_fusion", "combat_sword"), ("niveau_fusion_ultime", "master_sword")] {
                    if let Some(k) = self.world.content.weapon_index(into) {
                        let mut o = vec![Offer::Evolve(k)];
                        o.extend(offers.iter().copied().take(2));
                        screens.push((name.into(), State::LevelUp { offers: o, sel: 1, t: 1.0 }));
                    }
                }
                if let Some(e) = evo {
                    screens.push(("evolution".into(), State::Chest { reward: ChestReward { evolution: Some(e), upgrades: Vec::new() }, big: true, t: menus::EVO_DONE + 0.5 }));
                    screens.push(("evolution_approche".into(), State::Chest { reward: ChestReward { evolution: Some(e), upgrades: Vec::new() }, big: true, t: 1.3 }));
                    let ups: Vec<Offer> = (0..3).filter_map(|k| self.world.weapons.get(k).map(|w| Offer::UpgradeWeapon(w.id))).collect();
                    screens.push(("evolution_grand_coffre".into(), State::Chest { reward: ChestReward { evolution: Some(e), upgrades: ups }, big: true, t: menus::EVO_DONE + 0.5 }));
                }
                let mut few: Vec<menus::Unlock> = chars::roster(&c).into_iter().skip(1).take(3).map(menus::Unlock::Char).collect();
                few.extend(c.weapons.iter().enumerate().filter(|(_, w)| w.unlock.is_some()).take(2).map(|(k, _)| menus::Unlock::Weapon(k as u16)));
                screens.push(("deblocages".into(), State::Unlocks { items: few, t: 10.0, next: menus::AfterUnlock::Victory }));
                let mut many: Vec<menus::Unlock> = chars::roster(&c).into_iter().skip(1).take(6).map(menus::Unlock::Char).collect();
                many.extend(c.weapons.iter().enumerate().filter(|(_, w)| w.unlock.is_some()).map(|(k, _)| menus::Unlock::Weapon(k as u16)));
                screens.push(("deblocages_nombreux".into(), State::Unlocks { items: many, t: 20.0, next: menus::AfterUnlock::Dead }));
                // page suivante : tous les personnages d'un coup (bien plus qu'une page)
                let all: Vec<menus::Unlock> = chars::roster(&c).into_iter().map(menus::Unlock::Char).collect();
                screens.push(("deblocages_pages".into(), State::Unlocks { items: all, t: 20.0, next: menus::AfterUnlock::Victory }));
                for (name, talk) in [("fee", fairy_tip::Talk::Shop), ("fee_intro", fairy_tip::Talk::Intro), ("fee_bravo", fairy_tip::Talk::Victory)] {
                    for page in 0..talk.pages() {
                        screens.push((format!("{name}_{page}"), State::Fairy { talk, page, t: 30.0, ask: None }));
                    }
                }
                screens.push(("fin_ganon".into(), State::GanonDeath { t: 5.0, pos: Vec2::ZERO }));
                screens.push(("fee_passer".into(), State::Fairy { talk: fairy_tip::Talk::Shop, page: 2, t: 30.0, ask: Some(1) }));
                // choix entre deux armes qui partagent un objet partenaire (arc et lance-pierre : carquois)
                let pair: Vec<(usize, u16)> = ["silver_bow", "hyper_slingshot"].iter().filter_map(|id| c.weapon_index(id)).enumerate().collect();
                if pair.len() == 2 {
                    screens.push(("evolution_choix".into(), State::EvoChoice { options: pair, big: false, sel: 1, t: 2.0 }));
                }
                screens.push(("jeu_annonce_arme".into(), State::Playing));
                // coop en ligne : menu, code, salon (en connexion, plein), partie (allie qui choisit, a terre)
                screens.push(("coop_menu".into(), State::Coop { sel: 0 }));
                screens.push(("coop_code".into(), State::NameEntry { purpose: NamePurpose::Room, text: "KXQZ".into(), sel: 0 }));
                screens.push(("coop_salon".into(), State::Lobby { sel: 1 }));
                screens.push(("coop_salon_connexion".into(), State::Lobby { sel: 0 }));
                screens.push(("coop_salon_invite".into(), State::Lobby { sel: 0 }));
                screens.push(("coop_partie".into(), State::Playing));
                screens.push(("coop_partie_terre".into(), State::Playing));
                // choix en pleine partie : protection restante, puis finie
                let pick: Vec<Offer> = offers.iter().copied().take(3).collect();
                screens.push(("coop_partie_choix".into(), State::LevelUp { offers: pick.clone(), sel: 0, t: 1.0 }));
                screens.push(("coop_partie_choix_expose".into(), State::LevelUp { offers: pick, sel: 0, t: 1.0 }));
                // ASTP_UICHECK=<debut> : seulement les ecrans dont le nom commence ainsi (mise au point)
                let only = std::env::var("ASTP_UICHECK").unwrap_or_default();
                for (name, st) in screens.into_iter().filter(|(n, _)| n.starts_with(&only)) {
                    self.hud.toasts.clear();
                    if name == "jeu_annonce_arme" {
                        if let Some(k) = c.weapon_index("ocarina") {
                            self.hud.toasts.push(crate::hud::Toast {
                                icon: self.vis.weapons[k as usize].icon,
                                title: self.lang.get("ui.weapon_unlocked").to_string(),
                                name: self.lang.get("weapon.ocarina.name").to_string(),
                                sub: self.lang.get("ui.weapon_unlocked_sub").to_string(),
                                t: 1.0,
                                life: 6.5,
                            });
                        }
                    }
                    self.updater.force_status(match name.as_str() {
                        "titre_maj_dispo" => UpdateStatus::Available(update::Release::sample("10.20.30")),
                        "titre_maj_telechargement" => UpdateStatus::Downloading { version: "10.20.30".into(), done: 99, total: 100 },
                        "titre_maj_prete" => UpdateStatus::Ready("10.20.30".into()),
                        "titre_maj_echec" => UpdateStatus::Failed(String::new()),
                        _ => UpdateStatus::Idle,
                    });
                    self.state = st;
                    // fiche ouverte (« fiche_<onglet>_<element> » ; onglet 0 : bestiaire)
                    self.card = None;
                    self.beast = None;
                    // « _achat » : question d'achat de la figurine
                    let (short, ask) = name.strip_suffix("_achat").map_or((name.as_str(), false), |s| (s, true));
                    self.fig_ask = ask.then_some(0);
                    if let Some((t, k)) = short.strip_prefix("fiche_").and_then(|r| r.split_once('_')).and_then(|(t, k)| Some((t.parse::<usize>().ok()?, k.parse::<usize>().ok()?))) {
                        if t == 0 {
                            self.beast = Some((k, ctx.real_time - 10.0));
                        } else {
                            self.card = Some((t, k, ctx.real_time - 10.0));
                        }
                    }
                    // galerie : Link devant l'aile de l'ecran
                    self.gallery = match self.state {
                        State::Gallery { wing, .. } => Some(gallery::Room::new(&self.content, &self.meta, &self.profile, wing)),
                        _ => None,
                    };
                    let solo_world = self.coop_screen(ctx, &name);
                    // personnage encore verrouille : texte d'indice et avancement
                    let kills = self.profile.totals.kills;
                    if name == "persos_verrouille" {
                        self.profile.totals.kills = 1234;
                    }
                    self.char_lock = name.starts_with("persos_couleur");
                    let device = ctx.input.last_device;
                    if name.starts_with("persos_couleur_souris") {
                        ctx.input.last_device = engine::input::Device::Mouse;
                    }
                    let players = if name.starts_with("classement_coop") { 4 } else { 1 };
                    self.online.force(&first, "", players, NetStatus::Done(if players > 1 { coop_board.clone() } else { board.clone() }));
                    let mut list = DrawList::default();
                    list.probe = Some(Vec::new());
                    self.draw(ctx, 1.0, &mut list);
                    ctx.input.last_device = device;
                    self.char_lock = false;
                    self.profile.totals.kills = kills;
                    if let Some(w) = solo_world {
                        self.world = w;
                    }
                    self.coop_close();
                    self.others_vis.clear();
                    self.coop_names.clear();
                    rep.analyze(list.probe.as_deref().unwrap_or(&[]), view, ctx.atlas.font("main"), &name, code);
                }
            }
        }
        ctx.view = real_view;
        self.gallery = None;
        self.card = None;
        self.beast = None;
        self.fig_ask = None;
        self.hud.toasts.clear();
        self.state = State::Title { sel: 0 };
        let text = rep.print();
        println!("{text}");
    }

    /// Ecrans de la coop : salle factice (4 joueurs aux noms longs) ; partie a 4 avec le monde commun.
    /// Rend le monde solo a remettre apres le dessin.
    pub(super) fn coop_screen(&mut self, ctx: &Ctx, name: &str) -> Option<World> {
        if !name.starts_with("coop_") || name == "coop_menu" || name == "coop_code" {
            return None;
        }
        let players = [("WWWWWWWWWWWW", "link", true), ("Zelda", "zelda", false), ("MMMMMMMMMMMM", "co_monoco", true), ("Impa", "sheik", true)];
        let mut s = crate::coop::Session::demo(&players);
        match name {
            "coop_salon_connexion" => s.status = crate::coop::Status::Connecting,
            "coop_salon_invite" => s.slot = 1,
            _ => {}
        }
        if !name.starts_with("coop_partie") {
            self.coop = Some(s);
            return None;
        }
        let heroes: Vec<crate::coop::Hero> = players
            .iter()
            .enumerate()
            .map(|(k, p)| crate::coop::Hero { slot: k as u8, name: p.0.into(), character: p.1.into(), skin: String::new(), mods: sim::mods::RunMods::default() })
            .collect();
        let start = crate::coop::Start { seed: 3, map: self.world.map_id.clone(), delay: 4, view: [240.0, 135.0], heroes, difficulty: 1, seals: Vec::new() };
        let mut w = crate::coop::build_world(&self.content, &start, 0);
        for (k, h) in w.others.iter_mut().enumerate() {
            h.player.pos = Vec2::new(-60.0 + k as f32 * 60.0, 30.0);
            h.player.prev = h.player.pos;
        }
        // un allie choisit sa recompense, un autre est a terre (a moitie releve)
        w.others[0].pending_levels = 1;
        w.others[0].player.choose_t = 4.0;
        if name.starts_with("coop_partie_choix") {
            w.pending_levels = 1;
            w.player.choose_t = if name.ends_with("expose") { 12.0 } else { 3.2 };
        }
        w.others[1].player.alive = false;
        w.others[1].player.revive = 1.5;
        if name == "coop_partie_terre" {
            w.player.alive = false;
        }
        self.others_vis = start.heroes.iter().map(|h| self.ally_vis(&ctx.atlas, &h.character, &h.skin)).collect();
        self.coop_names = start.heroes.iter().map(|h| h.name.clone()).collect();
        s.run = Some(crate::coop::Run::new(start, 0));
        self.coop = Some(s);
        Some(std::mem::replace(&mut self.world, w))
    }
}
