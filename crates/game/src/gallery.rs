//! Galerie (onglet de la Collection, ou ecran titre) : une salle ou Link se promene entre ses trophees,
//! ses armes, ses fusions, ses objets, les figurines du bestiaire et les statues des personnages
//! debloques, chacun sur un socle. Devant un socle, son nom s'affiche ; A ouvre la fiche deja utilisee
//! par la Collection (achat des figurines compris). Une aile par onglet de la Collection puis par univers
//! de personnages, l'entree en bas, la Grande Fee au fond (pourcentage de la collection) ; une aile
//! complete allume ses torches. La camera suit Link.
use crate::chars;
use crate::hud::{frame_rect, text, GOLD};
use crate::lang::Lang;
use crate::meta::Meta;
use crate::profile::Profile;
use crate::records;
use crate::trophies;
use crate::visuals::Vis;
use engine::app::Ctx;
use engine::assets::{AnimId, Atlas};
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::defs::Content;
use sim::horde::FACE_DOWN;

const SOFT: Color = rgb(210, 210, 236);
const DIMMED: Color = rgb(120, 116, 140);
// couleurs des rectangles en valeurs lineaires (l'ecran les convertit en sRGB)
const FLOOR_A: Color = [0.042, 0.032, 0.069, 1.0];
const FLOOR_B: Color = [0.032, 0.024, 0.051, 1.0];
const CARPET: Color = [0.188, 0.013, 0.021, 1.0];
const WALL: Color = [0.013, 0.009, 0.025, 1.0];
const STONE: Color = [0.093, 0.080, 0.138, 1.0];
const STONE_TOP: Color = [0.305, 0.279, 0.402, 1.0];
const CLOTH: Color = [0.020, 0.012, 0.045, 1.0];
const WATER: Color = [0.020, 0.150, 0.420, 1.0];
const COLS: usize = 10;
const STEP: Vec2 = Vec2::new(36.0, 44.0);
const MARGIN: f32 = 20.0;
const HEADER: f32 = 30.0;
const ENTRANCE: f32 = 56.0;
/// Fond de la salle : fontaine de la Grande Fee.
const FOUNTAIN: f32 = 96.0;
const POOL: Vec2 = Vec2::new(84.0, 26.0);
const SPEED: f32 = 90.0;
/// Plus grand cote d'un objet expose (les grands boss sont reduits a cette taille).
const SHOW: f32 = 30.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Exhibit {
    Trophy(usize),
    Weapon(usize),
    Fusion(usize),
    Item(usize),
    Figurine(usize),
    /// Statue d'un personnage debloque (indice dans `Room::chars`).
    Statue(usize),
    /// Grande Fee, au fond de la salle.
    Fairy,
}

/// Contenu d'une aile : un onglet de la Collection ou un univers de personnages (`chars::GROUPS`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WingKind {
    Tab(usize),
    Group(usize),
}

pub struct Wing {
    pub kind: WingKind,
    pub top: f32,
    pub items: Vec<Exhibit>,
    /// Elements de l'aile en tout (personnages : debloques ou non).
    pub total: usize,
}

pub struct Room {
    pub wings: Vec<Wing>,
    /// Personnages exposes (statues).
    pub chars: Vec<String>,
    pub size: Vec2,
    pub link: Vec2,
    /// 0 bas, 1 haut, 2 gauche, 3 droite.
    pub face: u8,
    pub walk: f32,
    pub moving: bool,
    /// Socle devant Link a l'image precedente (cri d'une figurine quand Link arrive).
    pub last_near: Option<Exhibit>,
}

/// Ailes de l'entree vers le fond : la Collection, puis un univers de personnages par aile (ceux qui en ont).
pub fn kinds(content: &Content) -> Vec<WingKind> {
    let mut v: Vec<WingKind> = [records::TAB_TROPHIES, records::TAB_WEAPONS, records::TAB_FUSIONS, records::TAB_ITEMS, records::TAB_BESTIARY].into_iter().map(WingKind::Tab).collect();
    v.extend((0..chars::GROUPS.len()).filter(|&g| !chars::page_roster(content, g).is_empty()).map(WingKind::Group));
    v
}

impl Room {
    /// Salle entiere ; Link entre par le bas, ou juste devant l'aile `wing` (indice de `kinds`).
    pub fn new(content: &Content, meta: &Meta, p: &Profile, wing: usize) -> Room {
        let mut wings = Vec::new();
        let mut shown = Vec::new();
        let mut y = 24.0 + FOUNTAIN;
        for kind in kinds(content).into_iter().rev() {
            let (items, total): (Vec<Exhibit>, usize) = match kind {
                WingKind::Tab(records::TAB_TROPHIES) => {
                    let n = trophies::families(content, meta, p).len();
                    ((0..n).map(Exhibit::Trophy).collect(), n)
                }
                WingKind::Tab(records::TAB_WEAPONS) => count(records::armory_weapons(content, false).into_iter().map(Exhibit::Weapon).collect()),
                WingKind::Tab(records::TAB_FUSIONS) => count(records::armory_weapons(content, true).into_iter().map(Exhibit::Fusion).collect()),
                WingKind::Tab(records::TAB_ITEMS) => count((0..content.passives.len()).map(Exhibit::Item).collect()),
                WingKind::Tab(_) => count(records::bestiary_order(content, p).into_iter().map(Exhibit::Figurine).collect()),
                WingKind::Group(g) => {
                    // seulement les personnages debloques : les univers en comptent des centaines
                    let ids = chars::page_roster(content, g);
                    let total = ids.len();
                    let open: Vec<Exhibit> = ids
                        .into_iter()
                        .filter(|id| chars::unlocked(content, &content.characters[id], p))
                        .map(|id| {
                            shown.push(id);
                            Exhibit::Statue(shown.len() - 1)
                        })
                        .collect();
                    (open, total)
                }
            };
            let h = body(items.len());
            wings.push(Wing { kind, top: y, items, total });
            y += HEADER + h + 8.0;
        }
        let size = Vec2::new(MARGIN * 2.0 + STEP.x * COLS as f32, y + ENTRANCE);
        let mut room = Room { wings, chars: shown, size, link: Vec2::new(size.x * 0.5, size.y - 16.0), face: 1, walk: 0.0, moving: false, last_near: None };
        if wing > 0 {
            let kind = kinds(content).get(wing).copied();
            if let Some(w) = room.wings.iter().find(|w| Some(w.kind) == kind) {
                room.link.y = w.top + HEADER + body(w.items.len()) + 2.0;
            }
        }
        room
    }

    /// Centre du bassin de la Grande Fee (face avant).
    fn pool(&self) -> Vec2 {
        Vec2::new(self.size.x * 0.5, 24.0 + FOUNTAIN - 18.0)
    }

    /// Socles : (objet, centre de la face avant du socle). La Grande Fee d'abord (au fond).
    pub fn plinths(&self) -> impl Iterator<Item = (Exhibit, Vec2)> + '_ {
        std::iter::once((Exhibit::Fairy, self.pool())).chain(self.wings.iter().flat_map(|w| {
            w.items.iter().enumerate().map(move |(k, &e)| {
                let (c, r) = (k % COLS, k / COLS);
                (e, Vec2::new(MARGIN + STEP.x * (c as f32 + 0.5), w.top + HEADER + STEP.y * (r as f32 + 1.0) - 10.0))
            })
        }))
    }

    fn blocked(&self, at: Vec2) -> bool {
        if at.x < MARGIN - 8.0 || at.x > self.size.x - MARGIN + 8.0 || at.y < 30.0 || at.y > self.size.y - 6.0 {
            return true;
        }
        let pool = self.pool();
        if (at.x - pool.x).abs() < POOL.x * 0.5 + 3.0 && at.y < pool.y + 4.0 {
            return true;
        }
        self.plinths().skip(1).any(|(_, p)| (at.x - p.x).abs() < 13.0 && at.y > p.y - 10.0 && at.y < p.y + 4.0)
    }

    /// Deplacement (axe par axe : Link glisse le long des socles).
    pub fn step(&mut self, mv: Vec2, dt: f32) {
        self.moving = mv.length_squared() > 0.01;
        if !self.moving {
            return;
        }
        self.walk += dt;
        self.face = if mv.y.abs() >= mv.x.abs() { if mv.y > 0.0 { 0 } else { 1 } } else if mv.x < 0.0 { 2 } else { 3 };
        let d = mv.clamp_length_max(1.0) * SPEED * dt;
        let x = self.link + Vec2::new(d.x, 0.0);
        if !self.blocked(x) {
            self.link = x;
        }
        let y = self.link + Vec2::new(0.0, d.y);
        if !self.blocked(y) {
            self.link = y;
        }
    }

    /// Socle devant lequel Link se tient (il le regarde de pres, par en dessous).
    pub fn near(&self) -> Option<Exhibit> {
        let reach = |e: Exhibit| if e == Exhibit::Fairy { 30.0 } else { 16.0 };
        self.plinths()
            .filter(|&(e, p)| self.link.y >= p.y && (self.link - (p + Vec2::new(0.0, 8.0))).length() < reach(e))
            .min_by(|a, b| (self.link - a.1).length().total_cmp(&(self.link - b.1).length()))
            .map(|(e, _)| e)
    }

    pub fn camera(&self, view: Vec2) -> Vec2 {
        let axis = |l: f32, room: f32, v: f32| if room <= v { (room - v) * 0.5 } else { (l - v * 0.5).clamp(0.0, room - v) };
        Vec2::new(axis(self.link.x, self.size.x, view.x), axis(self.link.y - 12.0, self.size.y, view.y)).round()
    }
}

/// Hauteur des rangees d'une aile (aile de personnages sans statue : une bande, Link y entre sous le titre).
fn body(n: usize) -> f32 {
    if n == 0 {
        22.0
    } else {
        n.div_ceil(COLS) as f32 * STEP.y
    }
}

fn count(v: Vec<Exhibit>) -> (Vec<Exhibit>, usize) {
    let n = v.len();
    (v, n)
}

/// Etat d'un objet expose : montre, ombre (vu mais pas obtenu), cache.
#[derive(Clone, Copy, PartialEq)]
enum Shown {
    Full,
    Shadow,
    Hidden,
}

fn seen(content: &Content, p: &Profile, e: Exhibit) -> Shown {
    let armory = |id: &str| p.armory.get(id).copied().unwrap_or(0) > 0;
    match e {
        Exhibit::Weapon(k) | Exhibit::Fusion(k) => {
            let id = &content.weapon_ids[k];
            if armory(id) || p.evolutions.contains(id) {
                Shown::Full
            } else {
                Shown::Hidden
            }
        }
        Exhibit::Item(k) => {
            if armory(&content.passive_ids[k]) {
                Shown::Full
            } else {
                Shown::Hidden
            }
        }
        Exhibit::Figurine(k) => {
            let id = &content.enemy_ids[k];
            if p.figurines.contains(id) {
                Shown::Full
            } else if p.bestiary.get(id).copied().unwrap_or(0) > 0 {
                Shown::Shadow
            } else {
                Shown::Hidden
            }
        }
        Exhibit::Trophy(_) | Exhibit::Statue(_) | Exhibit::Fairy => Shown::Full,
    }
}

/// Paliers atteints d'une famille de trophees (0 a 3).
fn tier(f: &trophies::Family) -> usize {
    f.goals.iter().filter(|&&g| g > 0 && f.value >= g).count()
}

/// Paliers d'une famille (ceux dont l'objectif existe).
fn tiers(f: &trophies::Family) -> usize {
    f.goals.iter().filter(|&&g| g > 0).count()
}

/// Obtenu : trophee d'au moins un palier, figurine achetee, arme ou objet deja trouve, personnage debloque.
fn owned(content: &Content, p: &Profile, fams: &[trophies::Family], e: Exhibit) -> bool {
    match e {
        Exhibit::Trophy(k) => fams.get(k).is_some_and(|f| tier(f) > 0),
        _ => seen(content, p, e) == Shown::Full,
    }
}

/// Aile complete : tout obtenu (trophees : tous les paliers ; personnages : tous debloques).
fn complete(content: &Content, p: &Profile, fams: &[trophies::Family], w: &Wing) -> bool {
    w.items.len() == w.total
        && w.items.iter().all(|&e| match e {
            Exhibit::Trophy(k) => fams.get(k).is_some_and(|f| tier(f) == tiers(f)),
            _ => owned(content, p, fams, e),
        })
}

/// Avancement de toute la collection, en pour cent (paliers des trophees, figurines achetees, armes,
/// objets, personnages debloques).
pub fn percent(content: &Content, meta: &Meta, p: &Profile, room: &Room) -> u32 {
    let fams = trophies::families(content, meta, p);
    let (mut got, mut all) = (0usize, 0usize);
    for w in &room.wings {
        match w.kind {
            WingKind::Tab(records::TAB_TROPHIES) => {
                got += fams.iter().map(tier).sum::<usize>();
                all += fams.iter().map(tiers).sum::<usize>();
            }
            _ => {
                got += w.items.iter().filter(|&&e| owned(content, p, &fams, e)).count();
                all += w.total;
            }
        }
    }
    // 100 seulement si tout est la (pas d'arrondi vers le haut)
    (got * 100).checked_div(all).unwrap_or(100) as u32
}

fn anim(content: &Content, room: &Room, vis: &Vis, atlas: &Atlas, fams: &[trophies::Family], e: Exhibit) -> Option<AnimId> {
    match e {
        Exhibit::Weapon(k) | Exhibit::Fusion(k) => Some(vis.weapons[k].icon),
        Exhibit::Item(k) => Some(vis.passives[k]),
        Exhibit::Figurine(k) => Some(vis.enemies[k].pick(FACE_DOWN, 0.0, false).0),
        Exhibit::Trophy(k) => fams.get(k).and_then(|f| atlas.find(f.icon, "0")).or_else(|| atlas.find("icon_crystal", "0")),
        Exhibit::Statue(k) => {
            let sprite = &content.characters.get(room.chars.get(k)?)?.sprite;
            atlas.find(sprite, "walk_down").or_else(|| atlas.find(sprite, "idle_down"))
        }
        Exhibit::Fairy => atlas.find("fairy", "0"),
    }
}

fn wing_title(lang: &Lang, kind: WingKind) -> String {
    match kind {
        WingKind::Tab(t) => lang.get(records::TABS[t]).to_string(),
        WingKind::Group(g) => format!("{} - {}", lang.get("ui.characters"), lang.get(&format!("group.{}", chars::GROUPS[g]))),
    }
}

/// Nom et seconde ligne (etat, prix de la figurine, palier) du socle devant Link.
pub fn label(content: &Content, meta: &Meta, p: &Profile, lang: &Lang, room: &Room, e: Exhibit) -> (String, String) {
    if seen(content, p, e) == Shown::Hidden {
        return ("???".into(), lang.get("ui.gallery_unknown").to_string());
    }
    match e {
        Exhibit::Weapon(k) | Exhibit::Fusion(k) => (lang.get(&format!("weapon.{}.name", content.weapon_ids[k])).to_string(), lang.get("ui.gallery_card").to_string()),
        Exhibit::Item(k) => (lang.get(&format!("passive.{}.name", content.passive_ids[k])).to_string(), lang.get("ui.gallery_card").to_string()),
        Exhibit::Figurine(k) => {
            let id = &content.enemy_ids[k];
            let line = if p.figurines.contains(id) {
                lang.get("ui.figurine_owned").to_string()
            } else {
                lang.get("ui.figurine_buy").replace("{n}", &meta.figurine_cost(&content.enemies[k], id).to_string())
            };
            (lang.get(&format!("enemy.{id}")).to_string(), line)
        }
        Exhibit::Trophy(k) => {
            let fams = trophies::families(content, meta, p);
            let Some(f) = fams.get(k) else { return (String::new(), String::new()) };
            let name = lang.get(&format!("trophy.{}", f.id)).to_string();
            let n = tier(f);
            let next = f.goals.iter().copied().filter(|&g| g > 0).nth(n);
            let line = match next {
                Some(g) => format!("{}  {} / {}", if n > 0 { lang.get(trophies::TIER_KEYS[n - 1]) } else { "" }, trophies::amount(f, f.value.min(g)), trophies::amount(f, g)),
                None => lang.get(trophies::TIER_KEYS[2]).to_string(),
            };
            (name, line.trim().to_string())
        }
        Exhibit::Statue(k) => {
            let id = room.chars.get(k).cloned().unwrap_or_default();
            let group = content.characters.get(&id).map(chars::group_of).unwrap_or(0);
            (chars::name(lang, &id), lang.get(&format!("group.{}", chars::GROUPS[group])).to_string())
        }
        Exhibit::Fairy => {
            let n = percent(content, meta, p, room);
            let line = if n >= 100 { lang.get("ui.gallery_complete").to_string() } else { lang.get("ui.gallery_percent").replace("{n}", &n.to_string()) };
            (lang.get("ui.tab.fairy").to_string(), line)
        }
    }
}

/// Socle devant Link : la fiche qu'il ouvre (onglet de la Collection et indice), s'il y en a une.
pub fn card_of(content: &Content, p: &Profile, room: &Room, e: Exhibit) -> Option<(usize, usize)> {
    match (e, seen(content, p, e)) {
        (_, Shown::Hidden) | (Exhibit::Fairy, _) => None,
        (Exhibit::Statue(k), _) => {
            let id = room.chars.get(k)?;
            chars::roster(content).iter().position(|r| r == id).map(|i| (records::CARD_CHARS, i))
        }
        (Exhibit::Trophy(k), _) => Some((records::TAB_TROPHIES, k)),
        (Exhibit::Weapon(k), _) => Some((records::TAB_WEAPONS, k)),
        (Exhibit::Fusion(k), _) => Some((records::TAB_FUSIONS, k)),
        (Exhibit::Item(k), _) => Some((records::TAB_ITEMS, k)),
        (Exhibit::Figurine(k), _) => Some((records::TAB_BESTIARY, k)),
    }
}

/// Link vient d'arriver devant une figurine achetee : elle pousse son cri (son du jeu, boss : rugissement).
pub fn cry(content: &Content, p: &Profile, room: &mut Room) -> Option<&'static str> {
    let near = room.near();
    let arrived = near != room.last_near;
    room.last_near = near;
    match near {
        Some(Exhibit::Figurine(k)) if arrived && p.figurines.contains(&content.enemy_ids[k]) => {
            Some(if records::boss_rank(content, &content.enemy_ids[k]) > 0 { "boss_spawn" } else { "enemy_hit" })
        }
        Some(Exhibit::Fairy) if arrived => Some("fairy"),
        _ => None,
    }
}

/// Lignes de l'onglet Galerie : une aile par ligne (A y fait entrer Link), avec ce qui est expose.
pub fn rows(content: &Content, meta: &Meta, vis: &Vis, atlas: &Atlas, p: &Profile, lang: &Lang) -> (Vec<crate::hub::Row>, Vec<String>) {
    let room = Room::new(content, meta, p, 0);
    let fams = trophies::families(content, meta, p);
    let mut rows = Vec::new();
    let mut details = Vec::new();
    for kind in kinds(content) {
        let Some(w) = room.wings.iter().find(|w| w.kind == kind) else { continue };
        let got = w.items.iter().filter(|&&e| owned(content, p, &fams, e)).count();
        rows.push(crate::hub::Row {
            icon: w.items.iter().find_map(|&e| anim(content, &room, vis, atlas, &fams, e)),
            label: wing_title(lang, kind),
            value: format!("{got} / {}", w.total),
            value_color: Some(DIMMED),
            mark: complete(content, p, &fams, w),
            ..Default::default()
        });
        details.push(lang.get("ui.gallery_detail").to_string());
    }
    (rows, details)
}

/// Torche au mur : allumee (flamme qui vacille) quand l'aile est complete.
fn torch(list: &mut DrawList, white: &assetkit::meta::Frame, at: Vec2, lit: bool, t: f32) {
    list.rect(white, at + Vec2::new(-2.0, 0.0), Vec2::new(4.0, 8.0), STONE_TOP);
    list.rect(white, at + Vec2::new(-3.0, -2.0), Vec2::new(6.0, 2.0), STONE);
    if lit {
        let f = ((t * 9.0 + at.x * 0.13).sin() * 1.5).round();
        list.rect(white, at + Vec2::new(-3.0, -9.0 - f.max(0.0)), Vec2::new(6.0, 7.0 + f.max(0.0)), [1.0, 0.35, 0.05, 1.0]);
        list.rect(white, at + Vec2::new(-1.0, -6.0), Vec2::new(2.0, 4.0), [1.0, 0.85, 0.30, 1.0]);
        list.rect(white, at + Vec2::new(-12.0, -14.0), Vec2::new(24.0, 22.0), [1.0, 0.45, 0.10, 0.10]);
    }
}

/// Image d'un sprite centre a l'echelle `sc`, posee sur `base` (bas de l'image).
fn on_base(list: &mut DrawList, atlas: &Atlas, a: AnimId, base: Vec2, t: f32, prm: Params, max: f32) {
    let size = (0..8).map(|i| atlas.frame(a, i as f32 * 0.1)).fold((0u16, 0u16), |m, g| (m.0.max(g.w), m.1.max(g.h)));
    let sc = 1.0f32.min(max / size.0.max(size.1).max(1) as f32);
    let f = atlas.frame(a, t);
    let center = base + Vec2::new(0.0, -(size.1 as f32) * sc * 0.5);
    let off = Vec2::new(f.px as f32 - f.w as f32 * 0.5, f.py as f32 - f.h as f32 * 0.5) * sc;
    list.frame(f, (center + off).round(), &prm.scale(Vec2::splat(sc)));
}

/// Salle, socles, Link (dessines du fond vers l'avant), puis le nom du socle devant lui.
#[allow(clippy::too_many_arguments)]
pub fn draw(list: &mut DrawList, ctx: &Ctx, room: &Room, content: &Content, meta: &Meta, vis: &Vis, p: &Profile, lang: &Lang, link_side_left: bool, t: f32) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    list.set(Space::Ui, Blend::Alpha);
    let cam = room.camera(view);
    let o = -cam;
    list.rect(white, Vec2::ZERO, view, [0.005, 0.004, 0.009, 1.0]);
    // sol en dalles (seulement les dalles visibles), tapis central, murs
    let tile = 16.0;
    let (y0, y1) = (((cam.y / tile).floor() as i32).max(0), ((cam.y + view.y) / tile).ceil() as i32);
    let (x0, x1) = (((cam.x / tile).floor() as i32).max(0), ((cam.x + view.x) / tile).ceil() as i32);
    for ty in y0..y1.min((room.size.y / tile).ceil() as i32) {
        for tx in x0..x1.min((room.size.x / tile).ceil() as i32) {
            let c = if (tx + ty) % 2 == 0 { FLOOR_A } else { FLOOR_B };
            list.rect(white, o + Vec2::new(tx as f32, ty as f32) * tile, Vec2::splat(tile), c);
        }
    }
    let cx = room.size.x * 0.5;
    let pool = room.pool();
    let carpet_top = pool.y + 6.0;
    list.rect(white, o + Vec2::new(cx - 13.0, carpet_top), Vec2::new(26.0, room.size.y - carpet_top), CARPET);
    list.rect(white, o + Vec2::new(cx - 13.0, carpet_top), Vec2::new(2.0, room.size.y - carpet_top), with_alpha(GOLD, 0.6));
    list.rect(white, o + Vec2::new(cx + 11.0, carpet_top), Vec2::new(2.0, room.size.y - carpet_top), with_alpha(GOLD, 0.6));
    list.rect(white, o, Vec2::new(room.size.x, 24.0), WALL);
    list.rect(white, o + Vec2::new(0.0, 22.0), Vec2::new(room.size.x, 2.0), with_alpha(GOLD, 0.5));
    for x in [0.0, room.size.x - 6.0] {
        list.rect(white, o + Vec2::new(x, 0.0), Vec2::new(6.0, room.size.y), WALL);
    }
    // porte d'entree
    list.rect(white, o + Vec2::new(cx - 18.0, room.size.y - 6.0), Vec2::new(36.0, 6.0), [0.002, 0.002, 0.004, 1.0]);
    // fontaine de la Grande Fee : bassin, reflets, la fee qui flotte au-dessus
    let pp = o + pool - Vec2::new(POOL.x * 0.5, POOL.y);
    list.rect(white, pp - Vec2::splat(3.0), POOL + Vec2::new(6.0, 6.0), STONE_TOP);
    list.rect(white, pp, POOL, WATER);
    for k in 0..4 {
        let x = ((t * 0.6 + k as f32 * 0.27).fract() * (POOL.x - 12.0)).round();
        list.rect(white, pp + Vec2::new(4.0 + x, 5.0 + k as f32 * 5.0), Vec2::new(8.0, 1.0), [0.6, 0.8, 1.0, 0.35]);
    }
    let fams = trophies::families(content, meta, p);
    for w in room.wings.iter().filter(|w| (o.y + w.top).clamp(-4.0, view.y - 44.0) == o.y + w.top) {
        let got = w.items.iter().filter(|&&e| owned(content, p, &fams, e)).count();
        let title = format!("{}  {got}/{}", wing_title(lang, w.kind), w.total);
        text(list, main, &title, o + Vec2::new(cx, w.top + 8.0), 1.0, GOLD, Align::Center);
        let lit = complete(content, p, &fams, w);
        for x in [MARGIN + 6.0, room.size.x - MARGIN - 6.0] {
            torch(list, white, o + Vec2::new(x, w.top + 10.0), lit, t);
        }
    }
    let near = room.near();
    let mut drawn_link = false;
    for (k, (e, pos)) in room.plinths().enumerate() {
        if !drawn_link && pos.y > room.link.y {
            draw_link(list, atlas, room, o, link_side_left);
            drawn_link = true;
        }
        let at = o + pos;
        if e == Exhibit::Fairy {
            // la fee (16 px) en double, qui flotte au-dessus du bassin
            if let Some(a) = anim(content, room, vis, atlas, &fams, e) {
                let f = atlas.frame(a, t);
                let center = at + Vec2::new(0.0, -POOL.y - 18.0 + (t * 2.0).sin() * 3.0);
                let off = Vec2::new(f.px as f32 - f.w as f32 * 0.5, f.py as f32 - f.h as f32 * 0.5) * 2.0;
                list.frame(f, (center + off).round(), &Params::default().scale(Vec2::splat(2.0)));
            }
            continue;
        }
        if at.y < -4.0 || at.y - 40.0 > view.y - 30.0 {
            continue;
        }
        let shown = seen(content, p, e);
        let trophy_tier = match e {
            Exhibit::Trophy(k) => fams.get(k).map(tier).unwrap_or(0),
            _ => 3,
        };
        // socle : pierre, liseré bronze, argent ou or pour les trophees selon le palier
        let top = match (e, trophy_tier) {
            (Exhibit::Trophy(_), 1..=3) => trophies::TIER_COLORS[trophy_tier - 1],
            _ => STONE_TOP,
        };
        list.rect(white, at + Vec2::new(-10.0, -7.0), Vec2::new(20.0, 9.0), STONE);
        list.rect(white, at + Vec2::new(-10.0, -7.0), Vec2::new(20.0, 2.0), top);
        list.rect(white, at + Vec2::new(-10.0, 2.0), Vec2::new(20.0, 1.0), [0.0, 0.0, 0.0, 0.4]);
        if near == Some(e) {
            frame_rect(list, white, at + Vec2::new(-12.0, -9.0), Vec2::new(24.0, 13.0), with_alpha(GOLD, 0.9));
        }
        let Some(a) = (shown != Shown::Hidden).then(|| anim(content, room, vis, atlas, &fams, e)).flatten() else {
            // pas encore decouvert : un drap sombre sur le socle
            list.rect(white, at + Vec2::new(-6.0, -19.0), Vec2::new(12.0, 12.0), CLOTH);
            list.rect(white, at + Vec2::new(-7.0, -9.0), Vec2::new(14.0, 2.0), CLOTH);
            list.rect(white, at + Vec2::new(-1.0, -16.0), Vec2::new(2.0, 2.0), with_alpha(GOLD, 0.5));
            continue;
        };
        // tout en couleurs : les figurines achetees bougent sur place, chacune a son rythme ; le reste
        // (statues, objets, figurines a acheter) s'anime quand Link est devant
        let ft = match e {
            Exhibit::Figurine(_) if shown == Shown::Full => t + k as f32 * 0.37,
            _ if near == Some(e) => t,
            _ => 0.0,
        };
        on_base(list, atlas, a, at + Vec2::new(0.0, -8.0), ft, Params::default(), SHOW);
    }
    if !drawn_link {
        draw_link(list, atlas, room, o, link_side_left);
    }
    // bandeau du bas : nom du socle et ce que fait A, sinon comment sortir
    let (a, b) = match near {
        Some(e) => label(content, meta, p, lang, room, e),
        None => (lang.get("ui.gallery").to_string(), lang.get("ui.gallery_hint").to_string()),
    };
    let h = 30.0;
    // le bandeau couvre la salle (verification des ecrans : ce qui passe dessous est cache)
    list.probe_layer();
    list.rect(white, Vec2::new(0.0, view.y - h), Vec2::new(view.x, h), [0.02, 0.01, 0.05, 0.85]);
    list.rect(white, Vec2::new(0.0, view.y - h), Vec2::new(view.x, 1.0), with_alpha(GOLD, 0.6));
    text(list, main, &a, Vec2::new(view.x * 0.5, view.y - h + 4.0), 1.0, if near.is_some() { GOLD } else { WHITE }, Align::Center);
    text(list, main, &b, Vec2::new(view.x * 0.5, view.y - h + 16.0), 1.0, SOFT, Align::Center);
}

fn draw_link(list: &mut DrawList, atlas: &Atlas, room: &Room, o: Vec2, side_left: bool) {
    let (name, flip) = match room.face {
        0 => ("down", false),
        1 => ("up", false),
        2 => ("side", !side_left),
        _ => ("side", side_left),
    };
    let kind = if room.moving { "walk" } else { "idle" };
    let a = atlas.find("link", &format!("{kind}_{name}")).or_else(|| atlas.find("link", &format!("walk_{name}")));
    if let Some(a) = a {
        let f = atlas.frame(a, room.walk);
        let at = (o + room.link).round();
        list.rect(&atlas.white, at + Vec2::new(-5.0, -1.0), Vec2::new(10.0, 2.0), [0.0, 0.0, 0.0, 0.3]);
        list.frame(f, at, &Params::default().flip(flip));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque socle est accessible : la case devant lui est libre, et Link s'y arrete devant lui.
    #[test]
    fn every_plinth_can_be_reached() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = crate::content::load_content(&root);
        let meta = Meta::load(&root);
        let mut p = Profile::new();
        // quelques personnages debloques : une aile de statues
        p.totals.kills = 1_000_000;
        p.totals.wins = 1000;
        let mut room = Room::new(&content, &meta, &p, 0);
        assert!(!room.blocked(room.link), "entree libre");
        assert!(!room.chars.is_empty(), "statues");
        let plinths: Vec<_> = room.plinths().collect();
        assert!(plinths.len() > 50);
        for (e, pos) in plinths {
            room.link = pos + Vec2::new(0.0, 10.0);
            assert!(!room.blocked(room.link), "{e:?} : place devant le socle");
            assert_eq!(room.near(), Some(e));
        }
        // Link remonte le tapis jusqu'a la fontaine sans traverser socles ni bassin
        room.link = room.size * Vec2::new(0.5, 1.0) - Vec2::new(0.0, 16.0);
        for _ in 0..20_000 {
            room.step(Vec2::new(0.0, -1.0), 1.0 / 60.0);
        }
        assert_eq!(room.near(), Some(Exhibit::Fairy));
        assert!(percent(&content, &meta, &p, &room) < 100);
        // chaque aile de la liste de l'onglet a sa ligne d'entree
        for (k, kind) in kinds(&content).into_iter().enumerate() {
            let r = Room::new(&content, &meta, &p, k);
            assert!(!r.blocked(r.link), "{kind:?} : entree devant l'aile");
        }
    }
}
