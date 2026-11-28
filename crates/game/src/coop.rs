//! Coop en ligne, jusqu'a 4 joueurs. Une salle du serveur (objet durable du Worker) relaie les messages ;
//! chaque jeu fait tourner toute la partie avec les commandes de tous (synchronisation par pas : un pas
//! ne se joue que quand les commandes de chaque joueur pour ce pas sont arrivees). Les commandes locales
//! partent `delay` pas a l'avance pour cacher le trajet par le serveur ; la simulation est deterministe
//! (meme graine, trigonometrie `libm`), une empreinte reguliere de l'etat repere une desynchronisation.
//!
//! Le serveur prefixe chaque message relaye par la place de l'expediteur (`"2 {...}"`), ou par `S` pour
//! ses propres messages (arrivee, depart). Les places du serveur (0-3) peuvent avoir des trous ; dans la
//! partie, les heros sont numerotes 0..n dans l'ordre des places (`Start::heroes`).
use sim::glam::Vec2;
use serde::{Deserialize, Serialize};
use sim::mods::RunMods;
use sim::upgrades::{ChestReward, Offer};
use sim::world::{PlayerInput, World};
use std::collections::VecDeque;

pub const MAX: usize = sim::horde::MAX_HEROES;
/// Pas par message de commandes (deux fois moins de messages pour le serveur).
const BATCH: usize = 2;
/// Pas entre deux empreintes de l'etat.
const SUM_EVERY: u64 = 120;
/// Lettres des codes de salle (sans I, O, 0, 1 : pas de confusion a la lecture).
pub const CODE_CHARS: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ";
pub const CODE_LEN: usize = 4;

// ---------------------------------------------------------------------------------------------
// Commandes compactes : deplacement et visee quantifies (les memes valeurs dans tous les jeux,
// y compris pour le joueur local).

pub fn pack(i: &PlayerInput) -> u32 {
    let q = |v: f32| ((v.clamp(-1.0, 1.0) * 127.0).round() as i8) as u8 as u32;
    let mut flags = i.attack as u32 | (i.dash as u32) << 1 | (i.toggle_auto as u32) << 2;
    let aim = match i.aim {
        Some(a) if a.length_squared() > 0.01 => {
            flags |= 8;
            ((a.y.atan2(a.x) / std::f32::consts::TAU * 256.0).round() as i32).rem_euclid(256) as u32
        }
        _ => 0,
    };
    q(i.move_dir.x) | q(i.move_dir.y) << 8 | aim << 16 | flags << 24
}

pub fn unpack(v: u32) -> PlayerInput {
    let d = |b: u32| (b as u8 as i8) as f32 / 127.0;
    let flags = v >> 24;
    let mv = Vec2::new(d(v & 255), d(v >> 8 & 255));
    PlayerInput {
        move_dir: if mv.length_squared() > 1.0 { mv.normalize() } else { mv },
        aim: (flags & 8 != 0).then(|| Vec2::from_angle((v >> 16 & 255) as f32 / 256.0 * std::f32::consts::TAU)),
        attack: flags & 1 != 0,
        dash: flags & 2 != 0,
        toggle_auto: flags & 4 != 0,
    }
}

// ---------------------------------------------------------------------------------------------
// Messages

/// Ce qu'un joueur montre dans le salon (renvoye a chaque arrivee d'un joueur).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Info {
    pub name: String,
    pub character: String,
    /// Couleur choisie (sprite) : les autres le voient avec.
    #[serde(default)]
    pub skin: String,
    /// Personnage et preparatifs choisis : les modificateurs de sa progression.
    pub mods: Option<RunMods>,
    /// Demi-taille de son ecran (la partie prend la plus grande : les ennemis apparaissent hors de tous les ecrans).
    pub view: [f32; 2],
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hero {
    /// Place sur le serveur.
    pub slot: u8,
    pub name: String,
    pub character: String,
    #[serde(default)]
    pub skin: String,
    pub mods: RunMods,
}

/// Lancement par l'hote : tout ce qu'il faut pour construire la meme partie partout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Start {
    pub seed: u32,
    pub map: String,
    pub delay: u32,
    pub view: [f32; 2],
    pub heroes: Vec<Hero>,
    /// Difficulte et Sceaux de l'hote (multiplicateurs de la banque pour tous).
    pub difficulty: usize,
    pub seals: Vec<String>,
}

/// Choix d'un joueur (niveau, coffre), joue au meme pas dans tous les jeux (la partie continue pendant
/// qu'il choisit).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Cmd {
    /// Carte choisie (`None` : niveau passe) et relances utilisees.
    Level { offer: Option<Offer>, rerolls: u32 },
    Chest { reward: ChestReward },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Msg {
    Info(Info),
    Start(Start),
    /// Commandes des pas `k`, `k + 1`...
    In { k: u64, i: Vec<u32> },
    /// Choix a jouer au pas `k`.
    Cmd { k: u64, c: Cmd },
    /// Empreinte apres le pas `k` (en hexadecimal : un entier de 64 bits ne passe pas par JavaScript).
    Sum { k: u64, h: String },
    /// Fin de partie : retour au salon.
    Lobby,
}

/// Messages du serveur.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Server {
    /// Entree dans la salle : sa place et celles deja occupees.
    Hello { slot: u8, room: String, peers: Vec<u8> },
    Join { s: u8 },
    Leave { s: u8 },
    /// Refus : `full`, `started`, `version`, `no_room`.
    Error { e: String },
}

/// Un message recu, decode.
pub enum Incoming {
    Server(Server),
    Peer(u8, Msg),
}

pub fn decode(raw: &str) -> Option<Incoming> {
    let (from, json) = raw.split_once(' ')?;
    if from == "S" {
        return serde_json::from_str(json).ok().map(Incoming::Server);
    }
    let s: u8 = from.parse().ok().filter(|&s: &u8| (s as usize) < MAX)?;
    serde_json::from_str(json).ok().map(|m| Incoming::Peer(s, m))
}

/// Pas d'avance des commandes d'apres le pire aller-retour mesure (ms) : le trajet d'un joueur a
/// l'autre passe par le serveur, plus le regroupement par deux et un peu de marge.
pub fn delay_for(rtt_ms: f32) -> u32 {
    ((rtt_ms / (1000.0 / 60.0)).ceil() as u32 + BATCH as u32 + 2).clamp(4, 30)
}

// ---------------------------------------------------------------------------------------------
// Synchronisation par pas

pub struct Lockstep {
    pub delay: u64,
    /// Prochain pas a jouer.
    pub next: u64,
    /// Commandes recues de chaque heros, a partir du pas `next`.
    q: Vec<VecDeque<u32>>,
    /// Heros partis : dernier pas dont on a les commandes.
    left: Vec<Option<u64>>,
    retired: Vec<bool>,
}

impl Lockstep {
    /// `n` heros ; les `delay` premiers pas se jouent sans commande.
    pub fn new(n: usize, delay: u64, next: u64) -> Lockstep {
        Lockstep { delay, next, q: vec![VecDeque::from(vec![0; delay as usize]); n], left: vec![None; n], retired: vec![false; n] }
    }

    /// Commandes du heros `h` pour les pas `k`... ; refusees si elles ne suivent pas les precedentes.
    pub fn push(&mut self, h: usize, k: u64, inputs: &[u32]) -> bool {
        let Some(q) = self.q.get_mut(h) else { return false };
        if k != self.next + q.len() as u64 || self.left[h].is_some() {
            return false;
        }
        q.extend(inputs);
        true
    }

    /// Pas suivant de ce heros (pour ses propres commandes).
    pub fn tail(&self, h: usize) -> u64 {
        self.next + self.q[h].len() as u64
    }

    pub fn ready(&self) -> bool {
        (0..self.q.len()).all(|h| !self.q[h].is_empty() || self.left[h].is_some())
    }

    /// En retard sur tous les autres : un pas de plus pour les rattraper.
    pub fn behind(&self, me: usize) -> bool {
        let mut others = (0..self.q.len()).filter(|&h| h != me && self.left[h].is_none()).peekable();
        others.peek().is_some() && others.all(|h| self.q[h].len() as u64 > self.delay + 2)
    }

    /// Commandes du pas `next` (neutres pour un heros parti).
    pub fn pop(&mut self) -> Vec<PlayerInput> {
        self.next += 1;
        self.q.iter_mut().map(|q| q.pop_front().map(unpack).unwrap_or_default()).collect()
    }

    pub fn leave(&mut self, h: usize) {
        if h < self.q.len() && self.left[h].is_none() {
            self.left[h] = Some(self.tail(h) - 1);
        }
    }

    /// Heros a retirer de la partie maintenant (etat apres le pas `ticks`) : meme pas partout, celui de
    /// sa derniere commande.
    pub fn retire_due(&mut self, ticks: u64) -> Vec<u8> {
        let mut v = Vec::new();
        for h in 0..self.q.len() {
            if !self.retired[h] && self.left[h].is_some_and(|l| l <= ticks) {
                self.retired[h] = true;
                v.push(h as u8);
            }
        }
        v
    }

    /// Heros dont on attend les commandes (sans soi).
    pub fn waiting(&self, me: usize) -> Vec<u8> {
        (0..self.q.len()).filter(|&h| h != me && self.q[h].is_empty() && self.left[h].is_none()).map(|h| h as u8).collect()
    }
}

/// Joue le choix d'un heros (au meme pas dans tous les jeux, le sien compris).
pub fn apply_cmd(world: &mut World, hero: u8, cmd: &Cmd) {
    // relances et passes : deja decomptes par le menu du joueur local
    let remote = hero != world.local;
    world.front(hero);
    if world.player.slot == hero {
        match cmd {
            Cmd::Level { offer, rerolls } => {
                if remote {
                    world.mods.rerolls = world.mods.rerolls.saturating_sub(*rerolls);
                }
                match offer {
                    Some(o) => world.apply_offer(*o),
                    None if remote => world.mods.skips = world.mods.skips.saturating_sub(1),
                    None => {}
                }
                world.pending_levels = world.pending_levels.saturating_sub(1);
            }
            Cmd::Chest { reward } => {
                if !world.pending_chests.is_empty() {
                    world.pending_chests.remove(0);
                }
                world.apply_chest(reward);
            }
        }
        // plus rien a choisir : fin du choix des ce pas (protection et attaques)
        if world.pending_levels == 0 && world.pending_chests.is_empty() {
            world.player.choose_t = 0.0;
        }
    }
    world.front(world.local);
}

/// Construit la partie commune ; `me` : heros local.
pub fn build_world(content: &sim::defs::Content, start: &Start, me: u8) -> World {
    let seed = start.seed as u64;
    let first = &start.heroes[0];
    let mut w = World::new(content.clone(), &first.character, &start.map, seed);
    w.set_mods(first.mods.clone());
    for (k, h) in start.heroes.iter().enumerate().skip(1) {
        w.add_hero(k as u8, &h.character, h.mods.clone(), seed);
    }
    w.coop_scale();
    w.view_half = Vec2::from(start.view);
    w.local = me;
    w.front(me);
    w
}

// ---------------------------------------------------------------------------------------------
// Partie en cours

pub struct Run {
    pub start: Start,
    /// Heros local.
    pub me: u8,
    pub step: Lockstep,
    /// Choix en attente de leur pas (les siens compris) : (heros, pas, choix).
    cmds: VecDeque<(u8, u64, Cmd)>,
    /// Commandes locales pas encore envoyees : (premier pas, commandes).
    out: (u64, Vec<u32>),
    sums: Vec<(u64, u64)>,
    remote_sums: Vec<(u64, u64)>,
    pub desync: bool,
    /// Secondes passees a attendre les commandes des autres.
    pub stall: f32,
}

impl Run {
    pub fn new(start: Start, me: u8) -> Run {
        let n = start.heroes.len();
        let delay = start.delay as u64;
        Run { me, step: Lockstep::new(n, delay, 1), start, cmds: VecDeque::new(), out: (0, Vec::new()), sums: Vec::new(), remote_sums: Vec::new(), desync: false, stall: 0.0 }
    }

    /// Heros d'une place du serveur.
    pub fn hero_of(&self, slot: u8) -> Option<u8> {
        self.start.heroes.iter().position(|h| h.slot == slot).map(|k| k as u8)
    }

    /// Commande locale du prochain pas a envoyer.
    pub fn local_input(&mut self, input: &PlayerInput) {
        let me = self.me as usize;
        let k = self.step.tail(me);
        let v = pack(input);
        self.step.push(me, k, &[v]);
        if self.out.1.is_empty() {
            self.out.0 = k;
        }
        self.out.1.push(v);
    }

    /// Message des commandes regroupees (`force` : sans attendre le lot complet).
    pub fn flush(&mut self, force: bool) -> Option<Msg> {
        if self.out.1.is_empty() || (!force && self.out.1.len() < BATCH) {
            return None;
        }
        let (k, i) = std::mem::take(&mut self.out);
        Some(Msg::In { k, i })
    }

    /// Empreinte apres le pas `ticks` (une fois tous les `SUM_EVERY` pas) : message a envoyer.
    pub fn sum(&mut self, ticks: u64, h: u64) -> Option<Msg> {
        if !ticks.is_multiple_of(SUM_EVERY) {
            return None;
        }
        self.sums.push((ticks, h));
        if ticks.is_multiple_of(SUM_EVERY * 5) {
            log::info!("coop : pas {ticks}, empreinte {h:016x}");
        }
        self.compare();
        Some(Msg::Sum { k: ticks, h: format!("{h:016x}") })
    }

    fn compare(&mut self) {
        for &(k, h) in &self.remote_sums {
            if self.sums.iter().any(|s| s.0 == k && s.1 != h) {
                log::warn!("coop : desynchronisation au pas {k}");
                self.desync = true;
            }
        }
        let last = self.sums.last().map_or(0, |s| s.0);
        self.remote_sums.retain(|s| s.0 > last);
        self.sums.retain(|s| s.0 + SUM_EVERY * 4 > last);
    }

    /// Message d'un autre heros pendant la partie.
    pub fn receive(&mut self, hero: u8, m: Msg) {
        match m {
            Msg::In { k, i } => {
                if !self.step.push(hero as usize, k, &i) {
                    log::warn!("coop : commandes hors d'ordre du heros {hero} (pas {k})");
                }
            }
            Msg::Cmd { k, c } => self.cmds.push_back((hero, k, c)),
            Msg::Sum { k, h } => {
                if let Ok(h) = u64::from_str_radix(&h, 16) {
                    self.remote_sums.push((k, h));
                    self.compare();
                }
            }
            _ => {}
        }
    }

    /// Choix local : joue au pas de la prochaine commande locale (pas encore envoyee, donc le choix arrive
    /// chez les autres avant elle), ici comme ailleurs. Rend le message a envoyer.
    pub fn schedule(&mut self, c: Cmd) -> Msg {
        let k = self.step.tail(self.me as usize);
        log::info!("coop : choix local joue au pas {k}");
        self.cmds.push_back((self.me, k, c.clone()));
        Msg::Cmd { k, c }
    }

    /// Choix locaux pas encore joues.
    pub fn queued(&self) -> bool {
        self.cmds.iter().any(|c| c.0 == self.me)
    }

    /// Choix dont le pas est arrive (a jouer maintenant), dans le meme ordre partout : par pas, puis par
    /// heros, puis dans l'ordre d'envoi.
    pub fn due_cmds(&mut self, ticks: u64) -> Vec<(u8, Cmd)> {
        let (mut due, rest): (Vec<_>, Vec<_>) = self.cmds.drain(..).partition(|c| c.1 <= ticks);
        self.cmds = rest.into();
        due.sort_by_key(|c| (c.1, c.0));
        for &(h, k, _) in &due {
            if k < ticks {
                log::warn!("coop : choix du heros {h} en retard (pas {k} < {ticks})");
                self.desync = true;
            }
        }
        due.into_iter().map(|(h, _, c)| (h, c)).collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Transport : WebSocket (fil de fond sur ordinateur et Android, WebSocket du navigateur sinon)

pub enum Event {
    Open,
    Text(String),
    Closed(String),
}

/// Adresse de la salle : `room` = code, ou `new` pour en creer une.
pub fn room_url(room: &str, version: &str) -> String {
    let base = crate::online::base();
    let ws = if let Some(r) = base.strip_prefix("https://") {
        format!("wss://{r}")
    } else if let Some(r) = base.strip_prefix("http://") {
        format!("ws://{r}")
    } else {
        base
    };
    format!("{ws}/api/coop/{room}?v={version}")
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Link {
    tx: std::sync::mpsc::Sender<String>,
    rx: std::sync::mpsc::Receiver<Event>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Link {
    pub fn open(url: String) -> Link {
        use std::sync::mpsc::{channel, TryRecvError};
        let (tx, out_rx) = channel::<String>();
        let (in_tx, rx) = channel::<Event>();
        std::thread::spawn(move || {
            let mut ws = match tungstenite::connect(url.as_str()) {
                Ok((ws, _)) => ws,
                Err(e) => {
                    let _ = in_tx.send(Event::Closed(e.to_string()));
                    return;
                }
            };
            {
                use tungstenite::stream::MaybeTlsStream;
                let tcp = match ws.get_mut() {
                    MaybeTlsStream::Plain(s) => s,
                    MaybeTlsStream::Rustls(s) => s.get_mut(),
                    _ => unreachable!(),
                };
                let _ = tcp.set_nodelay(true);
                // non bloquant (un delai de lecture expire rend la socket inutilisable sous Windows)
                let _ = tcp.set_nonblocking(true);
            }
            let _ = in_tx.send(Event::Open);
            loop {
                let mut busy = false;
                loop {
                    match out_rx.try_recv() {
                        Ok(s) => {
                            busy = true;
                            // socket pleine : le message reste en file, envoye au prochain `flush`
                            match ws.write(tungstenite::Message::text(s)) {
                                Ok(()) => {}
                                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                                Err(e) => {
                                    let _ = in_tx.send(Event::Closed(e.to_string()));
                                    return;
                                }
                            }
                        }
                        Err(TryRecvError::Empty) => break,
                        // le jeu a ferme la liaison
                        Err(TryRecvError::Disconnected) => {
                            let _ = ws.close(None);
                            let _ = ws.flush();
                            return;
                        }
                    }
                }
                match ws.flush() {
                    Ok(()) => {}
                    Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => {
                        let _ = in_tx.send(Event::Closed(e.to_string()));
                        return;
                    }
                }
                match ws.read() {
                    Ok(tungstenite::Message::Text(t)) => {
                        busy = true;
                        if in_tx.send(Event::Text(t.to_string())).is_err() {
                            return;
                        }
                    }
                    Ok(tungstenite::Message::Close(_)) => {
                        let _ = in_tx.send(Event::Closed("closed".into()));
                        return;
                    }
                    Ok(_) => {}
                    Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                    Err(e) => {
                        let _ = in_tx.send(Event::Closed(e.to_string()));
                        return;
                    }
                }
                if !busy {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        });
        Link { tx, rx }
    }

    pub fn send(&self, s: String) {
        let _ = self.tx.send(s);
    }

    pub fn poll(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Link {
    ws: Option<web_sys::WebSocket>,
    inbox: std::rc::Rc<std::cell::RefCell<Vec<Event>>>,
    _cb: Vec<wasm_bindgen::closure::Closure<dyn FnMut(wasm_bindgen::JsValue)>>,
}

#[cfg(target_arch = "wasm32")]
impl Link {
    pub fn open(url: String) -> Link {
        use wasm_bindgen::{closure::Closure, JsCast, JsValue};
        let inbox = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        // meme domaine que la page
        let url = if url.starts_with('/') {
            let loc = web_sys::window().map(|w| w.location());
            let host = loc.as_ref().and_then(|l| l.host().ok()).unwrap_or_default();
            let secure = loc.and_then(|l| l.protocol().ok()).is_some_and(|p| p == "https:");
            format!("{}://{host}{url}", if secure { "wss" } else { "ws" })
        } else {
            url
        };
        let Ok(ws) = web_sys::WebSocket::new(&url) else {
            inbox.borrow_mut().push(Event::Closed("websocket".into()));
            return Link { ws: None, inbox, _cb: Vec::new() };
        };
        let mk = |f: Box<dyn FnMut(JsValue)>| Closure::wrap(f);
        let i1 = inbox.clone();
        let on_open = mk(Box::new(move |_| i1.borrow_mut().push(Event::Open)));
        let i2 = inbox.clone();
        let on_msg = mk(Box::new(move |e: JsValue| {
            if let Some(t) = e.dyn_into::<web_sys::MessageEvent>().ok().and_then(|e| e.data().as_string()) {
                i2.borrow_mut().push(Event::Text(t));
            }
        }));
        let i3 = inbox.clone();
        let on_close = mk(Box::new(move |_| i3.borrow_mut().push(Event::Closed("closed".into()))));
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        Link { ws: Some(ws), inbox, _cb: vec![on_open, on_msg, on_close] }
    }

    pub fn send(&self, s: String) {
        if let Some(ws) = &self.ws {
            let _ = ws.send_with_str(&s);
        }
    }

    pub fn poll(&self) -> Vec<Event> {
        std::mem::take(&mut *self.inbox.borrow_mut())
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for Link {
    fn drop(&mut self) {
        if let Some(ws) = &self.ws {
            ws.set_onopen(None);
            ws.set_onmessage(None);
            ws.set_onclose(None);
            let _ = ws.close();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Salle

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Connecting,
    Lobby,
    Closed(String),
}

pub struct Session {
    /// `None` : salle quittee (liaison fermee).
    link: Option<Link>,
    pub status: Status,
    pub room: String,
    pub slot: u8,
    pub peers: [bool; MAX],
    pub infos: [Option<Info>; MAX],
    pub run: Option<Run>,
    /// Lancement recu (a construire par le jeu).
    pub started: Option<Start>,
    /// Pire aller-retour mesure (ms) et mesure en cours.
    pub rtt: f32,
    ping: Option<f64>,
    ping_cd: f32,
}

impl Session {
    pub fn open(room: &str) -> Session {
        Session::with(Some(Link::open(room_url(room, crate::title::VERSION))))
    }

    /// Salle factice, sans liaison (controle des ecrans) : `players` = (nom, personnage, pret).
    pub fn demo(players: &[(&str, &str, bool)]) -> Session {
        let mut s = Session::with(None);
        s.status = Status::Lobby;
        s.room = "KXQZ".into();
        s.rtt = 48.0;
        for (k, &(name, character, ready)) in players.iter().enumerate().take(MAX) {
            s.peers[k] = true;
            let mods = ready.then(RunMods::default);
            s.infos[k] = Some(Info { name: name.into(), character: character.into(), skin: String::new(), mods, view: [240.0, 135.0], version: String::new() });
        }
        s
    }

    fn with(link: Option<Link>) -> Session {
        Session {
            link,
            status: Status::Connecting,
            room: String::new(),
            slot: 0,
            peers: [false; MAX],
            infos: Default::default(),
            run: None,
            started: None,
            rtt: 0.0,
            ping: None,
            ping_cd: 0.0,
        }
    }

    pub fn send(&self, m: &Msg) {
        if let (Some(link), Ok(s)) = (&self.link, serde_json::to_string(m)) {
            link.send(s);
        }
    }

    /// Ferme la liaison ; `why` : texte a montrer dans le salon.
    pub fn close(&mut self, why: &str) {
        self.link = None;
        self.status = Status::Closed(why.to_string());
    }

    /// Sa fiche a change (personnage choisi, retour au salon) : renvoyee aux autres.
    pub fn set_info(&mut self, i: Info) {
        if self.status == Status::Lobby {
            self.send(&Msg::Info(i.clone()));
            self.infos[self.slot as usize] = Some(i);
        }
    }

    /// Hote : la plus petite place occupee.
    pub fn host(&self) -> u8 {
        (0..MAX).find(|&s| self.peers[s]).unwrap_or(0) as u8
    }

    pub fn is_host(&self) -> bool {
        self.host() == self.slot
    }

    /// Joueurs presents (places).
    pub fn players(&self) -> Vec<u8> {
        (0..MAX as u8).filter(|&s| self.peers[s as usize]).collect()
    }

    /// Tous les joueurs ont choisi leur personnage.
    pub fn all_ready(&self) -> bool {
        self.players().iter().all(|&s| self.infos[s as usize].as_ref().is_some_and(|i| i.mods.is_some()))
    }

    /// Lit les messages recus ; `now` : temps reel (s), `mine` : sa fiche, renvoyee aux nouveaux venus.
    pub fn update(&mut self, now: f64, dt: f32, mine: &Info) {
        let events = self.link.as_ref().map(|l| l.poll()).unwrap_or_default();
        for ev in events {
            match ev {
                Event::Open => {}
                Event::Closed(e) => {
                    if !matches!(self.status, Status::Closed(_)) {
                        log::warn!("coop : liaison fermee ({e})");
                        self.status = Status::Closed("ui.coop.lost".into());
                    }
                }
                Event::Text(t) if t == "pong" => {
                    if let Some(t0) = self.ping.take() {
                        let ms = ((now - t0) * 1000.0) as f32;
                        self.rtt = if self.rtt <= 0.0 { ms } else { self.rtt.max(ms) * 0.9 + ms * 0.1 };
                    }
                }
                Event::Text(t) => match decode(&t) {
                    Some(Incoming::Server(Server::Hello { slot, room, peers })) => {
                        self.slot = slot;
                        self.room = room;
                        self.peers = [false; MAX];
                        for p in peers {
                            if (p as usize) < MAX {
                                self.peers[p as usize] = true;
                            }
                        }
                        self.peers[slot as usize] = true;
                        self.infos[slot as usize] = Some(mine.clone());
                        self.status = Status::Lobby;
                        log::info!("coop : salle {}, place {slot}", self.room);
                        self.send(&Msg::Info(mine.clone()));
                    }
                    Some(Incoming::Server(Server::Join { s })) => {
                        if (s as usize) < MAX {
                            self.peers[s as usize] = true;
                            self.send(&Msg::Info(mine.clone()));
                        }
                    }
                    Some(Incoming::Server(Server::Leave { s })) => {
                        if (s as usize) < MAX {
                            self.peers[s as usize] = false;
                            self.infos[s as usize] = None;
                            if let Some(run) = &mut self.run {
                                if let Some(h) = run.hero_of(s) {
                                    run.step.leave(h as usize);
                                }
                            }
                        }
                    }
                    Some(Incoming::Server(Server::Error { e })) => {
                        self.status = Status::Closed(format!("ui.coop.err_{e}"));
                    }
                    Some(Incoming::Peer(s, m)) => self.peer(s, m),
                    None => log::warn!("coop : message illisible"),
                },
            }
        }
        // aller-retour mesure dans le salon (repondu par le serveur sans le reveiller)
        self.ping_cd -= dt;
        if self.status == Status::Lobby && self.run.is_none() && self.ping.is_none() && self.ping_cd <= 0.0 {
            self.ping_cd = 1.0;
            self.ping = Some(now);
            if let Some(l) = &self.link {
                l.send("ping".into());
            }
        }
    }

    fn peer(&mut self, s: u8, m: Msg) {
        match m {
            Msg::Info(i) => self.infos[s as usize] = Some(i),
            Msg::Start(st) => {
                // partie creee tout de suite : les commandes qui suivent le lancement dans le meme lot de
                // messages ne doivent pas se perdre (le jeu construit son monde a l'image suivante)
                if let Some(me) = st.heroes.iter().position(|h| h.slot == self.slot) {
                    self.run = Some(Run::new(st.clone(), me as u8));
                }
                self.started = Some(st);
            }
            m => {
                if let Some(run) = &mut self.run {
                    if let Some(h) = run.hero_of(s) {
                        run.receive(h, m);
                    }
                }
            }
        }
    }

    /// Hote : lancement avec les joueurs prets.
    pub fn make_start(&self, map: &str, seed: u32, difficulty: usize, seals: Vec<String>) -> Option<Start> {
        if self.status != Status::Lobby {
            return None;
        }
        let mut heroes = Vec::new();
        let mut view = [0.0f32; 2];
        for s in self.players() {
            let i = self.infos[s as usize].as_ref()?;
            heroes.push(Hero { slot: s, name: i.name.clone(), character: i.character.clone(), skin: i.skin.clone(), mods: i.mods.clone()? });
            view = [view[0].max(i.view[0]), view[1].max(i.view[1])];
        }
        Some(Start { seed, map: map.to_string(), delay: delay_for(self.rtt), view, heroes, difficulty, seals })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> sim::defs::Content {
        crate::content::load_content(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    }

    #[test]
    fn pack_round_trip() {
        let i = PlayerInput { move_dir: Vec2::new(0.6, -0.8), aim: Some(Vec2::new(-3.0, 0.0)), attack: true, dash: false, toggle_auto: true };
        let o = unpack(pack(&i));
        assert!((o.move_dir - i.move_dir).length() < 0.01);
        assert!((o.aim.unwrap() - Vec2::new(-1.0, 0.0)).length() < 0.02);
        assert!(o.attack && !o.dash && o.toggle_auto);
        // la valeur decodee se recode a l'identique (tous les jeux utilisent la meme)
        assert_eq!(pack(&o), pack(&i));
        assert!(unpack(pack(&PlayerInput::default())).aim.is_none());
    }

    #[test]
    fn lockstep_waits_and_retires() {
        let mut l = Lockstep::new(2, 3, 1);
        // les 3 premiers pas sont neutres
        for _ in 0..3 {
            assert!(l.ready());
            l.pop();
        }
        assert!(!l.ready());
        assert!(l.push(0, 4, &[1, 2]));
        assert!(!l.push(0, 4, &[1]), "pas deja recu");
        assert!(!l.ready());
        assert!(l.push(1, 4, &[3]));
        assert!(l.ready());
        l.pop();
        // le heros 1 part : ses commandes s'arretent au pas 4, retire apres ce pas
        l.leave(1);
        assert!(l.retire_due(3).is_empty());
        assert_eq!(l.retire_due(4), vec![1]);
        assert!(l.ready(), "un heros parti ne bloque plus");
        assert_eq!(l.pop().len(), 2);
    }

    /// Choix du heros local (premiere carte, coffre), comme le ferait son menu : rien n'est joue avant
    /// son pas (`Run::schedule`).
    fn choose(w: &mut World) -> Option<Cmd> {
        if w.pending_levels > 0 {
            return Some(Cmd::Level { offer: Some(w.roll_offers()[0]), rerolls: 0 });
        }
        let big = *w.pending_chests.first()?;
        Some(Cmd::Chest { reward: w.roll_chest(big, None) })
    }

    fn two_start(c: &sim::defs::Content) -> Start {
        let _ = c;
        let mods = RunMods::default();
        Start {
            seed: 7,
            map: "hyrule_field".into(),
            delay: 4,
            view: [240.0, 135.0],
            heroes: vec![
                Hero { slot: 0, name: "a".into(), character: "link".into(), skin: String::new(), mods: mods.clone() },
                Hero { slot: 2, name: "b".into(), character: "link".into(), skin: String::new(), mods },
            ],
            difficulty: 1,
            seals: Vec::new(),
        }
    }

    /// Deux jeux, chacun du point de vue d'un joueur, se passent commandes et choix comme par le serveur :
    /// la partie continue pendant les choix et les deux restent identiques. Un joueur qui choisit n'attaque
    /// pas et ne perd pas de vie pendant sa protection ; passe ce delai, il redevient une cible.
    #[test]
    fn two_games_stay_in_sync() {
        let c = content();
        let start = two_start(&c);
        let mut w = [build_world(&c, &start, 0), build_world(&c, &start, 1)];
        let mut r = [Run::new(start.clone(), 0), Run::new(start, 1)];
        assert_eq!(w[0].player.slot, 0);
        assert_eq!(w[1].player.slot, 1);
        assert_eq!(w[0].checksum(), w[1].checksum());
        // trois niveaux et un coffre a choisir pour chacun, des le depart (memes etats des deux cotes)
        for x in &mut w {
            for slot in 0..2 {
                x.front(slot);
                x.pending_levels = 3;
                x.pending_chests.push(false);
            }
            x.front(x.local);
        }
        // le joueur 0 choisit vite, le joueur 1 laisse passer sa protection
        let think = [1.5, sim::world::CHOICE_SHIELD + 1.0];
        let (mut picks, mut exposed) = ([0; 2], false);
        let mut sums = std::collections::HashMap::new();
        while w[0].ticks < 3600 || w[1].ticks < 3600 {
            for g in 0..2 {
                let mut out = Vec::new();
                for (h, cmd) in r[g].due_cmds(w[g].ticks) {
                    apply_cmd(&mut w[g], h, &cmd);
                }
                let p = &w[g].player;
                if p.choosing() && !r[g].queued() && p.choose_t > think[g] {
                    let cmd = choose(&mut w[g]).expect("choix en attente");
                    out.push(r[g].schedule(cmd));
                    picks[g] += 1;
                }
                if r[g].step.ready() && w[g].ticks < 3600 {
                    let t = r[g].step.tail(g) as f32;
                    let input = PlayerInput { move_dir: Vec2::from_angle(t * 0.01 + g as f32), attack: true, dash: t as u32 % 200 == 0, ..Default::default() };
                    r[g].local_input(&input);
                    let inputs = r[g].step.pop();
                    let (hp, shielded) = (w[g].player.hp, w[g].player.shielded());
                    w[g].tick_all(&inputs);
                    let p = &w[g].player;
                    if shielded && p.choosing() {
                        assert!(p.hp >= hp, "joueur protege touche au pas {}", w[g].ticks);
                        assert!(p.swing.is_none(), "joueur en plein choix qui attaque");
                    }
                    exposed |= p.choosing() && !p.shielded();
                    if let Some(prev) = sums.insert(w[g].ticks, w[g].checksum()) {
                        assert_eq!(prev, w[g].checksum(), "desynchronisation au pas {}", w[g].ticks);
                    }
                    out.extend(r[g].flush(false));
                }
                // messages de ce jeu, recus dans l'ordre par l'autre
                for m in out {
                    r[1 - g].receive(g as u8, m);
                }
            }
        }
        assert!(picks[0] >= 4 && picks[1] >= 4, "des choix ont ete faits en pleine partie ({picks:?})");
        assert!(exposed, "la protection d'un joueur qui tarde a choisir s'arrete");
        assert!(!r[0].desync && !r[1].desync);
        assert!(w[0].horde.len() > 5);
        // un joueur part : retire au meme pas des deux cotes
        w[0].retire(1);
        w[1].retire(1);
        assert!(!w[1].player.alive && w[1].player.gone);
        let i = PlayerInput::default();
        for _ in 0..600 {
            w[0].tick_all(&[i, i]);
            w[1].tick_all(&[i, i]);
        }
        assert!(!w[1].player.alive, "un joueur parti ne se releve pas");
        assert_eq!(w[0].checksum(), w[1].checksum());
    }

    /// Un coffre tire sans l'ouvrir (`roll_chest`) puis ouvert (`apply_chest`) donne le meme etat qu'un
    /// coffre ouvert d'un coup, et ne change rien avant son ouverture.
    #[test]
    fn rolled_chest_matches_opened_chest() {
        let c = content();
        let start = two_start(&c);
        let mut a = build_world(&c, &start, 0);
        let mut b = build_world(&c, &start, 0);
        let before = (a.weapons.len(), a.passives.len(), a.player.max_hp);
        let r = a.roll_chest(true, None);
        assert_eq!((a.weapons.len(), a.passives.len(), a.player.max_hp), before, "rien d'ouvert avant son pas");
        assert!(!r.upgrades.is_empty());
        a.apply_chest(&r);
        let r2 = b.open_chest(true);
        assert_eq!(format!("{:?}", r.upgrades), format!("{:?}", r2.upgrades));
        let lv = |w: &World| (w.weapons.iter().map(|x| (x.id, x.level)).collect::<Vec<_>>(), w.passives.iter().map(|x| (x.id, x.level)).collect::<Vec<_>>());
        assert_eq!(lv(&a), lv(&b));
    }

    /// Bout en bout par un vrai serveur (`wrangler dev` local) : ASTP_SERVER=http://127.0.0.1:8787
    /// cargo test -p game --release coop_server -- --ignored. Un hote cree la salle, un invite la rejoint
    /// avec le code, l'hote lance ; les deux jeux jouent 20 s en se passant leurs commandes et restent identiques.
    #[test]
    #[ignore]
    fn coop_server_round_trip() {
        let c = content();
        let info = |name: &str| Info { name: name.into(), character: "link".into(), skin: String::new(), mods: Some(RunMods::default()), view: [240.0, 135.0], version: crate::title::VERSION.into() };
        let (ia, ib) = (info("hote"), info("invite"));
        let t0 = std::time::Instant::now();
        let now = || t0.elapsed().as_secs_f64();
        let wait = |s: &mut Session, i: &Info, until: &dyn Fn(&Session) -> bool| {
            for _ in 0..500 {
                s.update(now(), 0.01, i);
                if until(s) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            panic!("delai depasse ({:?})", s.status);
        };
        let mut a = Session::open("new");
        wait(&mut a, &ia, &|s| s.status == Status::Lobby);
        assert_eq!(a.room.len(), CODE_LEN);
        let mut b = Session::open(&a.room);
        wait(&mut b, &ib, &|s| s.status == Status::Lobby);
        assert_eq!(b.slot, 1);
        wait(&mut a, &ia, &|s| s.all_ready() && s.players().len() == 2);
        // aller-retour mesure
        wait(&mut a, &ia, &|s| s.rtt > 0.0);
        let start = a.make_start("hyrule_field", 11, 1, Vec::new()).expect("lancement");
        a.send(&Msg::Start(start.clone()));
        wait(&mut b, &ib, &|s| s.started.is_some());
        let sb = b.started.take().unwrap();
        assert_eq!(sb.seed, 11);
        let mut wa = build_world(&c, &start, 0);
        let mut wb = build_world(&c, &sb, 1);
        a.run = Some(Run::new(start, 0));
        b.run = Some(Run::new(sb, 1));
        let ticks = 1200u64;
        let mut stalls = 0;
        while wa.ticks < ticks || wb.ticks < ticks {
            for (s, w, i, me) in [(&mut a, &mut wa, &ia, 0u32), (&mut b, &mut wb, &ib, 1)] {
                s.update(now(), 0.004, i);
                let run = s.run.as_mut().unwrap();
                for (h, cmd) in run.due_cmds(w.ticks) {
                    apply_cmd(w, h, &cmd);
                }
                if w.ticks >= ticks {
                    continue;
                }
                // choix en pleine partie, joues au meme pas chez les deux
                let mut out: Vec<Msg> = Vec::new();
                if w.player.choosing() && !run.queued() {
                    out.push(run.schedule(choose(w).unwrap()));
                }
                if !run.step.ready() {
                    stalls += 1;
                    out.extend(run.flush(true));
                    for m in &out {
                        s.send(m);
                    }
                    continue;
                }
                let t = w.ticks as f32;
                let input = PlayerInput { move_dir: Vec2::from_angle(t * 0.02 + me as f32), attack: true, ..Default::default() };
                run.local_input(&input);
                let inputs = run.step.pop();
                w.tick_all(&inputs);
                out.extend(run.sum(w.ticks, w.checksum()));
                out.extend(run.flush(false));
                for m in &out {
                    s.send(m);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(4));
            assert!(now() < 120.0, "partie trop longue");
        }
        assert_eq!(wa.checksum(), wb.checksum(), "parties differentes apres {ticks} pas");
        assert!(!a.run.as_ref().unwrap().desync && !b.run.as_ref().unwrap().desync);
        println!("coop serveur : {ticks} pas en {:.1} s, {stalls} attentes, aller-retour {:.0} ms", now(), a.rtt);
        // l'invite part : l'hote est prevenu
        drop(b);
        wait(&mut a, &ia, &|s| !s.peers[1]);
    }
}

