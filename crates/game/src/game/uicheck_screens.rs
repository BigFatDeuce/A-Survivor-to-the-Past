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
                version: "10.20.30".into(),
                published: k == 1,
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
        self.online.force(&first, "", NetStatus::Done(board));
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
                }
                screens.push(("victoire".into(), State::Victory { t: 3.0, sel: 0 }));
                for tab in 0..shop::TABS.len() {
                    screens.push((format!("boutique_{tab}"), State::Shop { tab, sel: 1 }));
                }
                screens.push(("boutique_retour".into(), State::Shop { tab: 0, sel: 99 }));
                for tab in 0..records::TABS.len() {
                    screens.push((format!("profil_{tab}"), State::Records { tab, sel: 1, armed: tab == records::TAB_SAVE }));
                }
                screens.push(("joueurs".into(), State::Players { sel: 1, armed: false }));
                screens.push(("joueurs_supprimer".into(), State::Players { sel: players::DELETE, armed: true }));
                screens.push(("nom".into(), State::NameEntry { purpose: NamePurpose::Rename, text: "WWWWWWWWWWWW".into(), sel: players::DONE }));
                screens.push(("nom_pseudo".into(), State::NameEntry { purpose: NamePurpose::Create(4), text: String::new(), sel: 0 }));
                for tab in 0..scores::TABS.len() {
                    screens.push((format!("classement_{tab}"), State::Scores { tab, sel: 1, stage: 0, version: String::new() }));
                }
                for tab in 0..controls::TABS.len() {
                    let wait = Some((1, 1.0));
                    screens.push((format!("commandes_{tab}"), State::Controls { tab, sel: 1, wait, from_title: tab == 0 }));
                }
                screens.push(("commandes_reset".into(), State::Controls { tab: 0, sel: controls::RESET, wait: None, from_title: true }));
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
                // choix entre deux armes qui partagent un objet partenaire (arc et lance-pierre : carquois)
                let pair: Vec<(usize, u16)> = ["silver_bow", "hyper_slingshot"].iter().filter_map(|id| c.weapon_index(id)).enumerate().collect();
                if pair.len() == 2 {
                    screens.push(("evolution_choix".into(), State::EvoChoice { options: pair, big: false, sel: 1, t: 2.0 }));
                }
                screens.push(("jeu_annonce_arme".into(), State::Playing));
                for (name, st) in screens {
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
        self.hud.toasts.clear();
        self.state = State::Title { sel: 0 };
        let text = rep.print();
        println!("{text}");
    }
}
