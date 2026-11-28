//! Coop en ligne dans le jeu : menu, salon, lancement, pas de simulation synchronises, choix rejoues chez
//! les autres, dessin des allies (la synchronisation elle-meme est dans `crate::coop`).
use super::*;
use crate::coop::{self, Cmd, Msg, Session, Status as CoopStatus};
use crate::hud::text;
use engine::draw::Align;
use crate::visuals::CharVis;

/// Lignes du menu Coop.
const COOP_ITEMS: [&str; 3] = ["ui.coop.create", "ui.coop.join", "ui.back"];
const SOFT: Color = [0.75, 0.75, 0.85, 1.0];
const BAD: Color = rgb(255, 120, 110);
const GOOD: Color = rgb(140, 230, 140);
/// Secondes d'attente des commandes des autres avant de l'afficher.
const STALL_SHOWN: f32 = 0.3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LobbyItem {
    Character,
    Launch,
    Leave,
}

fn coop_y(view: Vec2, k: usize) -> f32 {
    (view.y * 0.4).round() + k as f32 * 22.0
}

impl Game {
    /// Partie de coop en cours (synchronisee).
    pub(super) fn coop_run(&self) -> bool {
        self.coop.as_ref().is_some_and(|s| s.run.is_some())
    }

    /// Plus personne debout, ou porte franchie : la suite ne depend plus des commandes.
    fn run_over(&self) -> bool {
        self.world.won || !self.world.any_alive()
    }

    /// Personnage d'un joueur avec sa couleur (une des couleurs du personnage, sinon celle du profil local).
    pub(super) fn ally_vis(&self, atlas: &engine::assets::Atlas, character: &str, skin: &str) -> CharVis {
        let def = self.content.characters.get(character).unwrap_or(&self.content.characters["link"]);
        if !def.skins.iter().any(|s| s.0 == skin) {
            return CharVis::new(atlas, def);
        }
        let mut d = def.clone();
        d.sprite = skin.to_string();
        CharVis::new(atlas, &d)
    }

    /// Nom d'un heros de la partie de coop.
    pub(super) fn hero_name(&self, hero: u8) -> String {
        self.coop_names.get(hero as usize).cloned().unwrap_or_default()
    }

    /// Fiche du joueur pour le salon : ses modificateurs une fois son personnage et ses preparatifs choisis.
    fn coop_info(&self, view: Vec2) -> coop::Info {
        coop::Info {
            name: self.profile.name.clone(),
            character: self.opts.character.clone(),
            // couleur du profil (deja appliquee au contenu, voir `chars::apply_skins`)
            skin: self.content.characters.get(&self.opts.character).map(|d| d.sprite.clone()).unwrap_or_default(),
            mods: self.coop_ready.then(|| self.meta.run_mods(&self.profile)),
            view: [view.x * 0.5, view.y * 0.5],
            version: title::VERSION.to_string(),
        }
    }

    pub(super) fn coop_open(&mut self, room: &str) {
        self.coop_ready = false;
        self.coop = Some(Session::open(room));
    }

    /// Quitte la salle (la liaison se ferme).
    pub(super) fn coop_close(&mut self) {
        self.coop = None;
        self.coop_ready = false;
        self.coop_rules = None;
    }

    /// Preparatifs termines (« Partir ! ») : pret dans le salon.
    pub(super) fn coop_set_ready(&mut self, ctx: &Ctx) {
        self.coop_ready = true;
        let info = self.coop_info(ctx.view);
        if let Some(s) = &mut self.coop {
            s.set_info(info);
        }
    }

    /// Choix local : envoye aux autres et joue au meme pas partout, ici compris (`Run::schedule`).
    fn coop_cmd(&mut self, c: Cmd) {
        if let Some(s) = &mut self.coop {
            if let Some(m) = s.run.as_mut().map(|r| r.schedule(c)) {
                s.send(&m);
            }
        }
    }

    /// Coop : un choix local attend encore son pas.
    pub(super) fn coop_queued(&self) -> bool {
        self.coop.as_ref().and_then(|s| s.run.as_ref()).is_some_and(|r| r.queued())
    }

    /// Niveau choisi (carte, ou passe) : joue tout de suite en solo ; en coop, a son pas dans tous les jeux.
    pub(super) fn take_level(&mut self, offer: Option<Offer>) {
        let rerolls = std::mem::take(&mut self.coop_rerolls);
        if self.coop_run() {
            self.coop_cmd(Cmd::Level { offer, rerolls });
            return;
        }
        if let Some(o) = offer {
            self.world.apply_offer(o);
        }
        self.world.pending_levels = self.world.pending_levels.saturating_sub(1);
    }

    /// Coffre ouvert (`pick` : arme a faire fusionner) : tout de suite en solo ; en coop, contenu tire
    /// maintenant et coffre ouvert a son pas dans tous les jeux.
    pub(super) fn take_chest(&mut self, big: bool, pick: Option<usize>) -> sim::upgrades::ChestReward {
        if !self.coop_run() {
            return self.world.open_chest_choice(big, pick);
        }
        let reward = self.world.roll_chest(big, pick);
        self.coop_cmd(Cmd::Chest { reward: reward.clone() });
        reward
    }

    pub(super) fn update_coop_menu(&mut self, ctx: &mut Ctx, nav: &NavIn, mut sel: usize) -> State {
        let view = ctx.view;
        let (pick, moved) = menus::input_menu(nav, view, COOP_ITEMS.len(), &mut sel, |k| coop_y(view, k), 260.0);
        if moved {
            self.sfx(ctx, "cursor");
        }
        if nav.back {
            self.sfx(ctx, "confirm");
            return State::Title { sel: TITLE_COOP };
        }
        match pick {
            Some(0) => {
                self.sfx(ctx, "confirm");
                self.coop_open("new");
                State::Lobby { sel: 0 }
            }
            Some(1) => {
                self.sfx(ctx, "confirm");
                State::NameEntry { purpose: NamePurpose::Room, text: String::new(), sel: 0 }
            }
            Some(_) => {
                self.sfx(ctx, "confirm");
                State::Title { sel: TITLE_COOP }
            }
            None => State::Coop { sel },
        }
    }

    /// Lignes du salon : (action, texte, possible).
    pub(super) fn lobby_items(&self) -> Vec<(LobbyItem, String, bool)> {
        let l = &self.lang;
        let Some(s) = &self.coop else { return Vec::new() };
        if s.status != CoopStatus::Lobby {
            return vec![(LobbyItem::Leave, l.get("ui.back").to_string(), true)];
        }
        let mut v = vec![(LobbyItem::Character, l.get("ui.coop.character").to_string(), true)];
        if s.is_host() {
            v.push((LobbyItem::Launch, l.get("ui.coop.launch").to_string(), s.players().len() >= 2 && s.all_ready()));
        }
        v.push((LobbyItem::Leave, l.get("ui.coop.leave").to_string(), true));
        v
    }

    pub(super) fn update_lobby(&mut self, ctx: &mut Ctx, nav: &NavIn, mut sel: usize) -> State {
        if self.coop.is_none() {
            return State::Coop { sel: 0 };
        }
        let view = ctx.view;
        let items = self.lobby_items();
        let n = items.len();
        sel = sel.min(n.saturating_sub(1));
        let (pick, moved) = menus::input_menu(nav, view, n, &mut sel, |k| lobby_y(view, n, k), 260.0);
        if moved {
            self.sfx(ctx, "cursor");
        }
        let act = if nav.back { Some(LobbyItem::Leave) } else { pick.map(|k| items[k].0) };
        match act {
            Some(LobbyItem::Character) => {
                self.sfx(ctx, "confirm");
                self.coop_ready = false;
                let info = self.coop_info(view);
                if let Some(s) = &mut self.coop {
                    s.set_info(info);
                }
                self.char_select()
            }
            Some(LobbyItem::Launch) if pick.is_some_and(|k| items[k].2) => {
                self.sfx(ctx, "confirm");
                if self.coop_launch(ctx) {
                    return std::mem::replace(&mut self.state, State::Playing);
                }
                State::Lobby { sel }
            }
            Some(LobbyItem::Launch) => {
                self.sfx(ctx, "error");
                State::Lobby { sel }
            }
            Some(LobbyItem::Leave) => {
                self.sfx(ctx, "confirm");
                self.coop_close();
                State::Coop { sel: 0 }
            }
            None => State::Lobby { sel },
        }
    }

    /// Hote : lance la partie avec les joueurs du salon (son stage, sa difficulte, ses Sceaux).
    fn coop_launch(&mut self, ctx: &mut Ctx) -> bool {
        let seed = self.rng.next_u32();
        let seals: Vec<String> = self.profile.seals.iter().cloned().collect();
        let Some(st) = self.coop.as_ref().and_then(|s| s.make_start(&self.opts.map, seed, self.profile.difficulty, seals)) else { return false };
        if let Some(s) = &self.coop {
            s.send(&Msg::Start(st.clone()));
        }
        self.start_coop(ctx, st);
        true
    }

    /// Armes que le joueur local n'a pas encore gagnees (pas proposees).
    pub(super) fn locked_weapons(&self) -> Vec<u16> {
        let p = &self.profile;
        (0..self.content.weapons.len())
            .filter(|&k| self.content.weapons[k].unlock.is_some() && !p.weapons.contains(&self.content.weapon_ids[k]))
            .map(|k| k as u16)
            .collect()
    }

    /// Lancement (par l'hote, ou recu) : la meme partie dans tous les jeux.
    pub(super) fn start_coop(&mut self, ctx: &mut Ctx, start: coop::Start) {
        let Some(slot) = self.coop.as_ref().map(|s| s.slot) else { return };
        let Some(me) = start.heroes.iter().position(|h| h.slot == slot) else {
            // arrive pendant le lancement : la partie part sans ce joueur
            log::warn!("coop : partie lancee sans ce joueur");
            return;
        };
        let map = start.map.clone();
        self.set_map(ctx, &map);
        self.opts.character = start.heroes[me].character.clone();
        // remise a zero de la partie, puis le monde commun remplace le monde solo
        self.restart(ctx);
        self.world = coop::build_world(&self.content, &start, me as u8);
        self.world.shot_hit = self.shot_hit.clone();
        self.world.locked_weapons = self.locked_weapons();
        self.others_vis = start.heroes.iter().map(|h| self.ally_vis(&ctx.atlas, &h.character, &h.skin)).collect();
        self.coop_names = start.heroes.iter().map(|h| h.name.clone()).collect();
        self.coop_chars = start.heroes.iter().map(|h| h.character.as_str()).collect::<Vec<_>>().join(",");
        // banque : difficulte et Sceaux de l'hote pour tous
        let mut rules = self.profile.clone();
        rules.difficulty = start.difficulty;
        rules.seals = start.seals.iter().cloned().collect();
        self.coop_rules = Some(CoopRules { difficulty: start.difficulty, bank: self.meta.bank_mult(&rules), score: self.meta.score_mult(&rules), kw: self.meta.kill_weight(&rules) });
        log::info!("coop : partie lancee, {} joueurs, heros local {me}, avance {} pas", start.heroes.len(), start.delay);
        // invite : partie deja creee a la reception du lancement (avec les commandes arrivees depuis)
        if let Some(s) = &mut self.coop {
            if s.run.as_ref().is_none_or(|r| r.start.seed != start.seed || r.me != me as u8) {
                s.run = Some(coop::Run::new(start, me as u8));
            }
        }
    }

    /// Fin de partie de coop (ecran de defaite ou de victoire) : retour au salon, pret pour la suivante.
    pub(super) fn coop_back_to_lobby(&mut self, ctx: &mut Ctx) {
        self.quit_to_title(ctx);
        if let Some(s) = &mut self.coop {
            s.run = None;
            s.send(&Msg::Lobby);
        }
        self.coop_rules = None;
        self.coop_set_ready(ctx);
        if matches!(self.state, State::Title { .. }) {
            self.state = State::Lobby { sel: 0 };
        }
    }

    /// Chaque image : messages de la salle, choix des autres arrives a leur pas, departs, liaison perdue.
    pub(super) fn coop_frame(&mut self, ctx: &mut Ctx, dt: f32) {
        let Some(mut sess) = self.coop.take() else { return };
        let info = self.coop_info(ctx.view);
        sess.update(ctx.real_time, dt, &info);
        if let Some(st) = sess.started.take() {
            self.coop = Some(sess);
            self.start_coop(ctx, st);
            return;
        }
        // essais (--demo --menu coop_auto) : l'hote lance des que deux joueurs sont prets
        let auto = self.opts.demo && self.opts.menu.as_deref().is_some_and(|m| m.starts_with("coop_auto"));
        if auto && matches!(self.state, State::Lobby { .. }) && sess.status == CoopStatus::Lobby && sess.run.is_none() && sess.is_host() && sess.players().len() >= 2 && sess.all_ready() {
            self.coop = Some(sess);
            self.coop_launch(ctx);
            return;
        }
        let mut lost = None;
        if let Some(run) = &mut sess.run {
            for h in run.step.retire_due(self.world.ticks) {
                self.world.retire(h);
            }
            if run.desync {
                lost = Some("ui.coop.desync");
            }
        }
        if let CoopStatus::Closed(_) = &sess.status {
            if sess.run.is_some() {
                lost = Some("ui.coop.lost");
            }
        }
        let over = self.run_over();
        if let Some(why) = lost {
            sess.close(why);
            sess.run = None;
            self.coop = Some(sess);
            // partie interrompue : comptee comme quittee ; le salon montre la raison
            if !over {
                self.quit_to_title(ctx);
            }
            self.coop_rules = None;
            if !matches!(self.state, State::Unlocks { .. } | State::Dead { .. } | State::Dying(_) | State::Victory { .. }) {
                self.state = State::Lobby { sel: 0 };
            }
            return;
        }
        self.coop = Some(sess);
    }

    /// Pas de simulation de la coop (appele au rythme fixe du moteur) : un pas des que les commandes de
    /// tous sont la (deux pour rattraper un retard) ; la partie continue pendant les choix de chacun.
    pub(super) fn coop_tick(&mut self, ctx: &mut Ctx) {
        let Some(mut sess) = self.coop.take() else { return };
        let Some(run) = sess.run.as_mut() else {
            self.coop = Some(sess);
            return;
        };
        let mut out = Vec::new();
        if self.run_over() {
            // partie finie pour tous : le decor continue seul, sans synchronisation
            let n = self.world.hero_count();
            self.world.tick_all(&vec![PlayerInput::default(); n]);
            self.on_events(ctx);
        } else {
            let mut stalled = true;
            for k in 0..2 {
                for h in run.step.retire_due(self.world.ticks) {
                    self.world.retire(h);
                }
                // choix (de tous, le sien compris) dont le pas est arrive
                for (h, c) in run.due_cmds(self.world.ticks) {
                    coop::apply_cmd(&mut self.world, h, &c);
                }
                if !run.step.ready() {
                    break;
                }
                stalled = false;
                let input = if matches!(self.state, State::Playing) { self.read_input(ctx) } else { PlayerInput::default() };
                self.suppress = false;
                run.local_input(&input);
                let inputs = run.step.pop();
                self.world.tick_all(&inputs);
                out.extend(run.sum(self.world.ticks, self.world.checksum()));
                self.on_events(ctx);
                if k == 1 || self.run_over() || !run.step.behind(run.me as usize) {
                    break;
                }
            }
            run.stall = if stalled { run.stall + DT } else { 0.0 };
            out.extend(run.flush(stalled));
        }
        for m in &out {
            sess.send(m);
        }
        self.coop = Some(sess);
        if matches!(self.state, State::Playing) && !self.run_over() {
            self.check_pending(ctx);
        }
    }

    /// Messages de la partie de coop : joueurs attendus, choix des autres, joueur local a terre.
    pub(super) fn draw_coop_overlay(&self, ctx: &Ctx, list: &mut DrawList) {
        let Some(run) = self.coop.as_ref().and_then(|s| s.run.as_ref()) else { return };
        if self.run_over() {
            return;
        }
        let view = ctx.view;
        let main = ctx.atlas.font("main");
        let w = &self.world;
        let mut lines: Vec<(String, Color)> = Vec::new();
        let names = |v: Vec<u8>| v.iter().map(|&h| self.hero_name(h)).collect::<Vec<_>>().join(", ");
        // allies en plein choix (la partie continue, ils sont proteges un moment)
        let busy: Vec<u8> = w.others.iter().filter(|h| h.player.choosing() && h.player.alive).map(|h| h.player.slot).collect();
        if !busy.is_empty() {
            lines.push((self.lang.get("ui.coop.choosing").replace("{s}", &names(busy)), GOLD));
        }
        if run.stall > STALL_SHOWN {
            let waiting = run.step.waiting(run.me as usize);
            lines.push((self.lang.get("ui.coop.waiting").replace("{s}", &names(waiting)), SOFT));
        }
        if !w.player.alive && !w.player.gone {
            lines.push((self.lang.get("ui.coop.down").to_string(), BAD));
        }
        list.set(Space::Ui, Blend::Alpha);
        for (k, (s, c)) in lines.iter().enumerate() {
            text(list, main, s, Vec2::new((view.x * 0.5).round(), (view.y * 0.66).round() + k as f32 * 14.0), 1.0, *c, Align::Center);
        }
    }

    /// Choix en cours du joueur local (coop) : protection restante, puis l'avertissement qu'elle est finie.
    pub(super) fn draw_choice_timer(&self, ctx: &Ctx, list: &mut DrawList) {
        let p = &self.world.player;
        if !self.coop_run() || !p.choosing() || !p.alive || self.run_over() {
            return;
        }
        let left = sim::world::CHOICE_SHIELD - p.choose_t;
        let (s, c) = if left > 0.0 { (self.lang.get("ui.coop.shield").replace("{n}", &format!("{}", left.ceil() as u32)), GOLD) } else { (self.lang.get("ui.coop.exposed").to_string(), BAD) };
        list.set(Space::Ui, Blend::Alpha);
        text(list, ctx.atlas.font("main"), &s, Vec2::new((ctx.view.x * 0.5).round(), 6.0), 1.0, c, Align::Center);
    }

    /// Allies : nom, vie et relevage au-dessus d'eux (gardes au bord de l'ecran quand ils en sortent).
    pub(super) fn draw_ally_tags(&self, ctx: &Ctx, list: &mut DrawList, cam: Vec2, alpha: f32) {
        if self.world.others.is_empty() {
            return;
        }
        let view = ctx.view;
        let main = ctx.atlas.font("main");
        let white = &self.vis.white;
        list.set(Space::Ui, Blend::Alpha);
        for h in &self.world.others {
            let p = &h.player;
            let at = self.to_ui(p.prev.lerp(p.pos, alpha) - Vec2::new(0.0, 30.0), cam, view);
            let at = Vec2::new(at.x.clamp(30.0, view.x - 30.0), at.y.clamp(64.0, view.y - 40.0)).round();
            let c = if p.gone {
                [0.5, 0.5, 0.55, 0.8]
            } else if p.alive && p.choosing() {
                GOLD
            } else if p.alive {
                WHITE
            } else {
                BAD
            };
            text(list, main, &self.hero_name(p.slot), at, 1.0, c, Align::Center);
            let (bw, y) = (24.0, at.y + 12.0);
            list.rect(white, Vec2::new(at.x - bw * 0.5, y), Vec2::new(bw, 2.0), [0.1, 0.1, 0.12, 0.8]);
            let (frac, bar) = if p.alive { ((p.hp / p.max_hp.max(1.0)).clamp(0.0, 1.0), GOOD) } else { ((p.revive / sim::world::REVIVE_TIME).clamp(0.0, 1.0), GOLD) };
            list.rect(white, Vec2::new(at.x - bw * 0.5, y), Vec2::new((bw * frac).round(), 2.0), bar);
        }
    }

    /// Allie dans le monde (trie avec les ennemis) : `k` = indice dans `World::others`.
    pub(super) fn draw_ally(&self, list: &mut DrawList, atlas: &engine::assets::Atlas, k: usize, alpha: f32) {
        let Some(h) = self.world.others.get(k) else { return };
        let p = &h.player;
        let Some(cv) = self.others_vis.get(p.slot as usize) else { return };
        let pos = p.prev.lerp(p.pos, alpha);
        let (anim, flip) = cv.pick(p.face, p.moving);
        let f = atlas.frame(anim, if p.moving { p.walk_t } else { 0.0 });
        let blink = p.hurt_t < self.content.rules.hurt_iframes && ((p.hurt_t * 24.0) as i32) % 2 == 1;
        if blink && p.alive {
            return;
        }
        let mut prm = Params::default().flip(flip);
        if !p.alive {
            prm = prm.color(if p.gone { [0.4, 0.4, 0.45, 0.5] } else { [1.0, 0.6, 0.6, 0.9] });
        }
        list.frame(f, pos, &prm);
        if let Some(sw) = p.swing {
            self.draw_swing(list, atlas, cv, &sw, pos - Vec2::new(0.0, 9.0), alpha);
        }
    }

    /// Ecran du menu Coop.
    pub(super) fn draw_coop_menu(&self, ctx: &Ctx, list: &mut DrawList, sel: usize) {
        let view = ctx.view;
        let main = ctx.atlas.font("main");
        text(list, main, self.lang.get("ui.coop"), Vec2::new(view.x * 0.5, (view.y * 0.14).round()), 3.0, WHITE, Align::Center);
        text(list, main, self.lang.get("ui.coop.about"), Vec2::new(view.x * 0.5, (view.y * 0.14).round() + 40.0), 1.0, SOFT, Align::Center);
        let items: Vec<(String, bool)> = COOP_ITEMS.iter().map(|k| (self.lang.get(k).to_string(), true)).collect();
        menus::draw_items(list, ctx, &items, sel, |k| coop_y(view, k));
    }

    /// Salon : code, joueurs (personnage, pret), actions.
    pub(super) fn draw_lobby(&self, ctx: &Ctx, list: &mut DrawList, sel: usize) {
        let Some(s) = &self.coop else { return };
        let view = ctx.view;
        let atlas = &ctx.atlas;
        let main = atlas.font("main");
        let white = &self.vis.white;
        let l = &self.lang;
        let cx = (view.x * 0.5).round();
        let top = (view.y * 0.1).round();
        let head = if s.status == CoopStatus::Lobby { l.get("ui.coop.room").replace("{c}", &crate::hud::hl(&s.room)) } else { l.get("ui.coop").to_string() };
        text(list, main, &head, Vec2::new(cx, top), 2.0, WHITE, Align::Center);
        let (line, c) = match &s.status {
            CoopStatus::Connecting => (l.get("ui.coop.connecting").to_string(), SOFT),
            CoopStatus::Closed(e) => (l.get(e).to_string(), BAD),
            CoopStatus::Lobby if s.is_host() => (l.get("ui.coop.share").to_string(), SOFT),
            CoopStatus::Lobby => (l.get("ui.coop.wait_host").to_string(), SOFT),
        };
        text(list, main, &line, Vec2::new(cx, top + 26.0), 1.0, c, Align::Center);
        if s.status == CoopStatus::Lobby {
            let w = 300.0f32.min(view.x - 32.0);
            let x0 = ((view.x - w) * 0.5).round();
            for (k, slot) in s.players().into_iter().enumerate() {
                let y = top + 46.0 + k as f32 * 26.0;
                let me = slot == s.slot;
                list.rect(white, Vec2::new(x0, y), Vec2::new(w, 22.0), with_alpha([0.08, 0.08, 0.14, 1.0], 0.85));
                crate::hud::frame_rect(list, white, Vec2::new(x0, y), Vec2::new(w, 22.0), if me { with_alpha(GOLD, 0.7) } else { [0.4, 0.4, 0.5, 0.6] });
                let info = s.infos[slot as usize].as_ref();
                let name = info.map_or("…".to_string(), |i| i.name.clone());
                if let Some(i) = info.filter(|i| self.content.characters.contains_key(&i.character)) {
                    let (anim, _) = self.ally_vis(atlas, &i.character, &i.skin).pick(FACE_DOWN, false);
                    list.frame(atlas.frame(anim, 0.0), Vec2::new(x0 + 14.0, y + 20.0), &Params::default());
                }
                let label = if slot == s.host() { format!("{name} ({})", l.get("ui.coop.host")) } else { name };
                let ty = crate::hud::mid_y(main, y, 22.0, 1.0);
                text(list, main, &label, Vec2::new(x0 + 28.0, ty), 1.0, if me { GOLD } else { WHITE }, Align::Left);
                let ready = info.is_some_and(|i| i.mods.is_some());
                let (st, sc) = if ready { (l.get("ui.coop.ready"), GOOD) } else { (l.get("ui.coop.picking"), SOFT) };
                text(list, main, st, Vec2::new(x0 + w - 8.0, ty), 1.0, sc, Align::Right);
            }
            if s.rtt > 0.0 {
                let ping = l.get("ui.coop.ping").replace("{n}", &format!("{:.0}", s.rtt));
                text(list, main, &ping, Vec2::new(cx, top + 46.0 + 4.0 * 26.0 + 2.0), 1.0, SOFT, Align::Center);
            }
        }
        let items: Vec<(String, bool)> = self.lobby_items().into_iter().map(|(_, s, ok)| (s, ok)).collect();
        let n = items.len();
        menus::draw_items(list, ctx, &items, sel, |k| lobby_y(view, n, k));
    }
}

/// Actions du salon, en bas de l'ecran.
fn lobby_y(view: Vec2, n: usize, k: usize) -> f32 {
    (view.y - 26.0 - (n - 1 - k) as f32 * 22.0).round()
}
