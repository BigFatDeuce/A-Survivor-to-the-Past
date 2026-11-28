//! Interface en jeu : coeurs, rubis, experience, chronometre, armes et objets, boss, annonces,
//! et la jauge de vie en cases sous Link.
use crate::lang::Lang;
use crate::visuals::Vis;
use assetkit::meta::Font;
use engine::app::Ctx;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::input::{Binding, Bindings, Device};
use engine::Vec2;
use sim::World;

pub const XP_GREEN: Color = rgb(64, 216, 64);
pub const BACK: Color = [0.06, 0.04, 0.10, 0.85];
pub const GOLD: Color = rgb(248, 216, 88);
pub const HP_RED: Color = rgb(224, 48, 56);
const DASH_BLUE: Color = rgb(120, 200, 255);

// ---------------------------------------------------------------------------------------------
// Mise en valeur dans les textes : un nom d'objet, d'arme ou de personnage entoure de balises
// (caracteres de controle, invisibles et sans largeur) est dessine en couleur, avec un reflet qui
// le parcourt et quelques etincelles. Retour 0.6.2 : le joueur doit reperer d'un coup d'oeil ce
// dont parle un message (« Une seule arme peut fusionner avec Carquois »).

/// Objet, arme, boss : or.
pub const HL: char = '\u{1}';
/// Fin de mise en valeur.
pub const HL_END: char = '\u{2}';
/// Fusion, evolution : rose qui chatoie.
pub const HL_FUSION: char = '\u{3}';
/// Deblocage, nouveaute : vert.
pub const HL_NEW: char = '\u{4}';

/// 0.7.1 : avantage (vert) et defaut (rouge) d'un personnage, sans reflet ni etincelles.
pub const HL_GOOD: char = '\u{5}';
pub const HL_BAD: char = '\u{6}';

pub fn hl_good(s: &str) -> String {
    format!("{HL_GOOD}{s}{HL_END}")
}

pub fn hl_bad(s: &str) -> String {
    format!("{HL_BAD}{s}{HL_END}")
}

pub fn hl(s: &str) -> String {
    format!("{HL}{s}{HL_END}")
}

pub fn hl_fusion(s: &str) -> String {
    format!("{HL_FUSION}{s}{HL_END}")
}

pub fn hl_new(s: &str) -> String {
    format!("{HL_NEW}{s}{HL_END}")
}

thread_local! {
    /// Horloge des reflets (temps reel, mis a jour a chaque image).
    static HL_TIME: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

pub fn set_text_time(t: f32) {
    HL_TIME.with(|c| c.set(t));
}

fn mix(a: Color, b: Color, k: f32) -> Color {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k, a[3] + (b[3] - a[3]) * k]
}

/// Texte avec ombre a une echelle entiere ; les balises de mise en valeur sont prises en compte.
/// Largeur des colonnes du HUD (coeurs, rubis, objets a gauche ; niveau, ennemis vaincus a droite), qui restent
/// dessinees sous les menus poses sur la partie.
pub const HUD_SIDE: f32 = 150.0;

/// Echelle du titre d'un menu pose sur la partie : `big`, ou plus petit s'il deborderait sur les colonnes du HUD
/// (ecrans etroits, titres longs).
pub fn title_scale(font: &Font, s: &str, view_x: f32, big: f32) -> f32 {
    let w = font.measure(s) as f32;
    let mut k = big;
    while k > 1.0 && w * k > view_x - 2.0 * HUD_SIDE {
        k -= 1.0;
    }
    k
}

/// Haut d'une ligne de texte centree verticalement dans un cadre (`top`, hauteur `h`) : avec `nudge`, le milieu de
/// la ligne est le milieu de l'encre des lettres (ombre comprise) ; un demi-pixel restant va vers le haut.
pub fn mid_y(font: &Font, top: f32, h: f32, scale: f32) -> f32 {
    (top + (h - font.line_height as f32 * scale) * 0.5).floor()
}

pub fn text(list: &mut DrawList, font: &Font, s: &str, pos: Vec2, scale: f32, color: Color, align: Align) -> f32 {
    if !s.chars().any(|c| c.is_control() && c != '\n') {
        return list.text_ex(font, s, pos, scale, color, align, Some([0.0, 0.0, 0.0, color[3] * 0.75]));
    }
    rich_text(list, font, s, pos, scale, color, align)
}

#[allow(clippy::too_many_arguments)]
fn rich_text(list: &mut DrawList, font: &Font, s: &str, pos: Vec2, scale: f32, color: Color, align: Align) -> f32 {
    let w = font.measure(s) as f32 * scale;
    let mut x = match align {
        Align::Left => pos.x,
        Align::Center => (pos.x - w * 0.5).round(),
        Align::Right => pos.x - w,
    };
    let x_start = x;
    // controle des ecrans : une seule boite pour toute la phrase (pas une par lettre ni par etincelle)
    let probe = list.probe.take();
    let t = HL_TIME.with(|c| c.get());
    let shadow = |c: Color| Some([0.0, 0.0, 0.0, c[3] * 0.75]);
    let sp = font.spacing as f32 * scale;
    let mut style: Option<char> = None;
    let mut first = true;
    let mut words: Vec<(f32, f32, char)> = Vec::new();
    let mut word_start = 0.0;
    let mut idx = 0usize;
    let mut buf = String::new();
    // texte ordinaire : par morceaux ; texte mis en valeur : lettre par lettre (couleur et reflet)
    let flush = |list: &mut DrawList, buf: &mut String, x: &mut f32, first: &mut bool| {
        if buf.is_empty() {
            return;
        }
        if !*first {
            *x += sp;
        }
        *first = false;
        let ww = list.text_ex(font, buf, Vec2::new(*x, pos.y), scale, color, Align::Left, shadow(color));
        *x += ww;
        buf.clear();
    };
    for ch in s.chars() {
        match ch {
            HL | HL_FUSION | HL_NEW | HL_GOOD | HL_BAD => {
                flush(list, &mut buf, &mut x, &mut first);
                style = Some(ch);
                word_start = x + if first { 0.0 } else { sp };
                idx = 0;
            }
            HL_END => {
                if let Some(st) = style.take().filter(|st| !matches!(*st, HL_GOOD | HL_BAD)) {
                    words.push((word_start, x, st));
                }
            }
            c if c.is_control() => {}
            c => match style {
                None => buf.push(c),
                Some(st) => {
                    if !first {
                        x += sp;
                    }
                    first = false;
                    let i = idx as f32;
                    idx += 1;
                    // reflet blanc qui balaie le mot
                    let calm = matches!(st, HL_GOOD | HL_BAD);
                    let glint = if calm { 0.0 } else { ((t * 3.2 - i * 0.45).sin().max(0.0)).powi(6) };
                    let (base, bob) = match st {
                        HL_FUSION => {
                            let k = 0.5 + 0.5 * (t * 2.5 - i * 0.6).sin();
                            (mix(rgb(255, 120, 240), rgb(255, 210, 120), k), ((t * 5.0 - i * 0.7).sin() * 1.0).round())
                        }
                        HL_NEW => (rgb(120, 255, 150), 0.0),
                        HL_GOOD => (rgb(90, 220, 90), 0.0),
                        HL_BAD => (rgb(250, 100, 90), 0.0),
                        _ => (GOLD, 0.0),
                    };
                    let mut col = mix(base, WHITE, glint * 0.85);
                    col[3] = color[3];
                    let cs = c.to_string();
                    let ww = list.text_ex(font, &cs, Vec2::new(x, pos.y + bob * scale), scale, col, Align::Left, shadow(col));
                    x += ww;
                }
            },
        }
    }
    flush(list, &mut buf, &mut x, &mut first);
    if let Some(st) = style.filter(|st| !matches!(*st, HL_GOOD | HL_BAD)) {
        words.push((word_start, x, st));
    }
    // etincelles autour des mots mis en valeur
    if color[3] > 0.3 {
        let space = list.space();
        list.set(space, Blend::Add);
        for (k, &(x0, x1, st)) in words.iter().enumerate() {
            let ww = (x1 - x0).max(4.0);
            let n = if st == HL_FUSION { 3 } else { 2 };
            for j in 0..n {
                let seed = k as f32 * 1.7 + j as f32 * 2.3;
                let ph = (t * 0.8 + seed * 0.37).fract();
                let cycle = (t * 0.8 + seed * 0.37).floor();
                let rx = ((cycle * 12.9898 + seed * 78.233).sin() * 43758.547).fract().abs();
                let ry = ((cycle * 39.346 + seed * 11.135).sin() * 24634.635).fract().abs();
                let a = (ph * std::f32::consts::PI).sin() * color[3];
                let c = match st {
                    HL_FUSION => [1.0, 0.75, 1.0, a],
                    HL_NEW => [0.7, 1.0, 0.8, a],
                    _ => [1.0, 0.95, 0.6, a],
                };
                let p = Vec2::new((x0 + rx * ww).round(), (pos.y + (ry * 10.0 - 3.0) * scale).round());
                list.text_ex(font, "+", p - Vec2::new(3.0, 3.0), 1.0, c, Align::Left, None);
            }
        }
        list.set(space, Blend::Alpha);
    }
    list.probe = probe;
    let ui = list.space() == Space::Ui;
    if let (Some(pr), true) = (list.probe.as_mut(), ui) {
        let clean: String = s.chars().filter(|c| !c.is_control()).collect();
        let missing: Vec<char> = clean.chars().filter(|&c| c != ' ' && font.glyph(c).is_none()).collect();
        let size = Vec2::new(w, font.line_height as f32 * scale);
        // boite de mise en page, sans `nudge` (meme regle que `text_ex`)
        let at = Vec2::new(x_start, pos.y);
        for ch in missing {
            pr.push(engine::gfx::Probe::MissingGlyph { ch, text: clean.clone() });
        }
        let ink = engine::draw::ink_band(font, scale, true);
        pr.push(engine::gfx::Probe::Text { pos: at, size, text: clean, ink });
    }
    w
}

/// Decoupe un texte en lignes d'au plus `width` pixels (coupure aux espaces). Une mise en valeur
/// coupee en fin de ligne est refermee, puis rouverte sur la ligne suivante.
pub fn wrap(font: &Font, s: &str, width: f32, scale: f32) -> Vec<String> {
    let lines = wrap_plain(font, s, width, scale);
    if !s.chars().any(|c| c.is_control() && c != '\n') {
        return lines;
    }
    let mut open: Option<char> = None;
    lines
        .into_iter()
        .map(|l| {
            let mut out = String::new();
            if let Some(o) = open {
                out.push(o);
            }
            for c in l.chars() {
                match c {
                    HL | HL_FUSION | HL_NEW | HL_GOOD | HL_BAD => open = Some(c),
                    HL_END => open = None,
                    _ => {}
                }
                out.push(c);
            }
            if open.is_some() {
                out.push(HL_END);
            }
            out
        })
        .collect()
}

fn wrap_plain(font: &Font, s: &str, width: f32, scale: f32) -> Vec<String> {
    let mut lines = Vec::new();
    // un saut de ligne force une nouvelle ligne
    for para in s.split('\n') {
        let mut cur = String::new();
        for word in para.split(' ') {
            let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
            if !cur.is_empty() && font.measure(&cand) as f32 * scale > width {
                lines.push(std::mem::take(&mut cur));
                cur = word.to_string();
            } else {
                cur = cand;
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
    }
    lines
}

/// Annonce d'un deblocage en pleine partie (arme debloquee par un mini-boss) : panneau en bas.
pub struct Toast {
    pub icon: engine::assets::AnimId,
    pub title: String,
    pub name: String,
    pub sub: String,
    pub t: f32,
    pub life: f32,
}

/// Grande annonce au centre de l'ecran.
pub struct Banner {
    pub text: String,
    pub color: Color,
    pub t: f32,
    pub life: f32,
}

pub struct Hud {
    /// Rubis affiches (comptent vers la vraie valeur).
    shown_rupees: f32,
    /// Rubis rouges deja gagnes pour la boutique si la partie s'arretait maintenant (fourni par le jeu).
    pub banked: u64,
    pub rupee_pop: f32,
    pub heart_shake: f32,
    pub mode_banner: f32,
    pub banner: Option<Banner>,
    /// Vie « perdue » affichee en blanc qui rattrape la vraie vie.
    hp_trail: f32,
    boss_trail: f32,
    /// Eclat de la jauge de dash quand elle redevient pleine.
    dash_ready: f32,
    dash_was_full: bool,
    t: f32,
    /// Reliques actives pendant la partie (equipees), affichees sous les objets.
    pub relics: Vec<engine::assets::AnimId>,
    /// Deblocages a annoncer (le premier est affiche).
    pub toasts: Vec<Toast>,
    /// Arme qui vient d'evoluer : sa case brille en rose (identifiant, temps restant).
    pub evo_flash: Option<(u16, f32)>,
    /// Menu pose sur la partie (niveau, coffre...) : le HUD reste, mais le chrono (fige), le nom et la barre du boss
    /// et les annonces (au milieu de l'ecran) sont caches : ils passeraient sous le titre du menu. En pause, le chrono reste.
    pub busy: bool,
    pub paused: bool,
}

impl Hud {
    pub fn new() -> Hud {
        Hud {
            shown_rupees: 0.0,
            banked: 0,
            rupee_pop: 0.0,
            heart_shake: 0.0,
            mode_banner: 0.0,
            toasts: Vec::new(),
            evo_flash: None,
            busy: false,
            paused: false,
            banner: None,
            hp_trail: 0.0,
            boss_trail: 1.0,
            dash_ready: 0.0,
            dash_was_full: true,
            relics: Vec::new(),
            t: 0.0,
        }
    }

    pub fn banner(&mut self, text: &str, color: Color, life: f32) {
        self.banner = Some(Banner { text: text.to_string(), color, t: 0.0, life });
    }

    pub fn update(&mut self, dt: f32, world: &World) {
        self.t += dt;
        let p = &world.player;
        let tr = p.rupees as f32;
        if self.shown_rupees < tr {
            self.shown_rupees = (self.shown_rupees + (tr - self.shown_rupees).max(4.0) * dt * 8.0).min(tr);
        } else {
            self.shown_rupees = tr;
        }
        self.rupee_pop = (self.rupee_pop - dt * 6.0).max(0.0);
        self.heart_shake = (self.heart_shake - dt * 2.5).max(0.0);
        self.mode_banner = (self.mode_banner - dt).max(0.0);
        if let Some(t) = self.toasts.first_mut() {
            t.t += dt;
            if t.t >= t.life {
                self.toasts.remove(0);
            }
        }
        if let Some((_, r)) = &mut self.evo_flash {
            *r -= dt;
            if *r <= 0.0 {
                self.evo_flash = None;
            }
        }
        if let Some(b) = &mut self.banner {
            b.t += dt;
            if b.t > b.life {
                self.banner = None;
            }
        }
        // la trainee blanche attend un instant puis rattrape la vie
        if self.hp_trail < p.hp {
            self.hp_trail = p.hp;
        } else if p.hurt_t > 0.35 {
            self.hp_trail = (self.hp_trail - dt * 40.0).max(p.hp);
        }
        if let Some(i) = world.boss_index() {
            let f = world.horde.hp[i] / world.horde.max_hp[i];
            self.boss_trail = if self.boss_trail < f { f } else { (self.boss_trail - dt * 0.25).max(f) };
        } else {
            self.boss_trail = 1.0;
        }
        let full = p.dash_cd <= 0.0;
        if full && !self.dash_was_full {
            self.dash_ready = 1.0;
        }
        self.dash_was_full = full;
        self.dash_ready = (self.dash_ready - dt * 4.0).max(0.0);
    }

    /// Jauges sous Link (dans le monde) : vie en cases (1 case = 1 coeur) et recharge du dash.
    pub fn draw_under_player(&self, list: &mut DrawList, world: &World, vis: &Vis, feet: Vec2) {
        let p = &world.player;
        if !p.alive {
            return;
        }
        let white = &vis.white;
        let n = (p.max_hp / 10.0).ceil().max(1.0) as i32;
        let bw = if n > 10 { 3.0 } else { 4.0 };
        let total = n as f32 * bw + (n - 1) as f32;
        let x0 = (feet.x - total * 0.5).round();
        let y0 = (feet.y + 4.0).round();
        list.set(Space::World, Blend::Alpha);
        list.rect(white, Vec2::new(x0 - 1.0, y0 - 1.0), Vec2::new(total + 2.0, 5.0), [0.0, 0.0, 0.0, 0.7]);
        let low = p.hp <= p.max_hp * 0.25;
        let blink = low && (self.t * 4.0).fract() < 0.5;
        for k in 0..n {
            let x = x0 + k as f32 * (bw + 1.0);
            list.rect(white, Vec2::new(x, y0), Vec2::new(bw, 3.0), [0.25, 0.08, 0.1, 1.0]);
            let lo = k as f32 * 10.0;
            let trail = ((self.hp_trail - lo) / 10.0).clamp(0.0, 1.0);
            let fill = ((p.hp - lo) / 10.0).clamp(0.0, 1.0);
            if trail > fill {
                list.rect(white, Vec2::new(x, y0), Vec2::new((bw * trail).round(), 3.0), [1.0, 1.0, 1.0, 0.9]);
            }
            if fill > 0.0 {
                let c = if blink { rgb(255, 140, 140) } else { HP_RED };
                list.rect(white, Vec2::new(x, y0), Vec2::new((bw * fill).round().max(1.0), 3.0), c);
                list.rect(white, Vec2::new(x, y0), Vec2::new((bw * fill).round().max(1.0), 1.0), [1.0, 0.6, 0.6, 1.0]);
            }
        }
        // recharge du dash : fine ligne visible seulement pendant la recharge, eclat quand prete
        let cd = p.dash_cooldown().max(0.01);
        let frac = 1.0 - (p.dash_cd / cd).clamp(0.0, 1.0);
        if frac < 1.0 || self.dash_ready > 0.0 {
            let y = y0 + 4.0;
            list.rect(white, Vec2::new(x0 - 1.0, y), Vec2::new(total + 2.0, 2.0), [0.0, 0.0, 0.0, 0.6]);
            let c = if frac >= 1.0 { with_alpha(WHITE, self.dash_ready) } else { with_alpha(DASH_BLUE, 0.9) };
            list.rect(white, Vec2::new(x0, y), Vec2::new((total * frac).round(), 1.0), c);
        }
    }

    pub fn draw(&self, list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang, bindings: &Bindings) {
        let view = ctx.view;
        let atlas = &ctx.atlas;
        let main = atlas.font("main");
        let hudf = atlas.font("hud");
        let white = &vis.white;
        let p = &world.player;
        list.set(Space::Ui, Blend::Alpha);

        // barre d'experience tout en haut
        let frac = (p.xp / p.xp_next).clamp(0.0, 1.0);
        list.rect(white, Vec2::ZERO, Vec2::new(view.x, 7.0), BACK);
        list.rect(white, Vec2::ZERO, Vec2::new((view.x * frac).round(), 6.0), XP_GREEN);
        list.rect(white, Vec2::ZERO, Vec2::new((view.x * frac).round(), 1.0), rgb(160, 255, 160));
        list.rect(white, Vec2::new(0.0, 6.0), Vec2::new(view.x, 1.0), [0.0, 0.0, 0.0, 0.7]);

        // coeurs x2 (1 coeur = 10 PV), rangees de 10
        let hearts = (p.max_hp / 10.0).ceil() as i32;
        let low = p.hp <= p.max_hp * 0.25 && p.alive;
        for i in 0..hearts {
            let hp_here = p.hp - i as f32 * 10.0;
            let id = if hp_here >= 10.0 {
                vis.hud_heart[0]
            } else if hp_here >= 5.0 {
                vis.hud_heart[1]
            } else {
                vis.hud_heart[2]
            };
            let (row, col) = (i / 10, i % 10);
            let mut pos = Vec2::new(8.0 + col as f32 * 16.0, 12.0 + row as f32 * 16.0);
            if self.heart_shake > 0.0 {
                let s = self.heart_shake * 3.0;
                pos += Vec2::new(((self.t * 70.0 + i as f32).sin() * s).round(), ((self.t * 53.0 + i as f32 * 2.0).cos() * s).round());
            }
            // le dernier coeur bat quand la vie est basse
            let mut sc = 2.0;
            if low && hp_here > 0.0 && hp_here <= 10.0 && (self.t * 3.0).fract() < 0.18 {
                sc = 3.0;
                pos -= Vec2::splat(4.0);
            }
            list.frame(atlas.frame(id, 0.0), pos, &Params::default().scale(Vec2::splat(sc)));
        }
        let rows = (hearts - 1) / 10 + 1;
        let mut y = 12.0 + rows as f32 * 16.0 + 4.0;

        // rubis
        let rup = atlas.frame(vis.rupees[0], 0.0);
        let pop = if self.rupee_pop > 0.5 { 2.0 } else { 1.0 };
        list.frame(rup, Vec2::new(13.0, y + 15.0) - Vec2::new(0.0, (pop - 1.0) * 7.0), &Params::default().scale(Vec2::splat(pop)));
        let txt = format!("{:03}", self.shown_rupees as u32);
        let tw = text(list, hudf, &txt, Vec2::new(24.0, y + 1.0), 2.0, WHITE, Align::Left);
        // rubis rouges gagnes pour la boutique (au moins ce montant, plus en cas de victoire)
        let rx = (24.0 + tw + 14.0).max(84.0);
        list.frame(atlas.frame(vis.icon_rupees, 0.0), Vec2::new(rx + 4.0, y + 8.0), &Params::default());
        text(list, hudf, &format!("{:03}", self.banked), Vec2::new(rx + 12.0, y + 1.0), 2.0, rgb(255, 150, 150), Align::Left);
        y += 22.0;

        // armes puis objets : cases vides visibles (la place est comptee)
        let rules = &world.content.rules;
        let slot = 20.0;
        for row in 0..2 {
            let n = if row == 0 { rules.max_weapons } else { rules.max_passives };
            for k in 0..n {
                let pos = Vec2::new(8.0 + k as f32 * (slot + 2.0), y);
                list.rect(white, pos, Vec2::splat(slot), [0.0, 0.0, 0.0, 0.45]);
                let (icon, level, max, evolved) = if row == 0 {
                    match world.weapons.get(k) {
                        Some(w) => {
                            let d = &world.content.weapons[w.id as usize];
                            (Some(vis.weapons[w.id as usize].icon), w.level, d.max_level(), d.evolved)
                        }
                        None => (None, 0, 0, false),
                    }
                } else {
                    match world.passives.get(k) {
                        Some(ps) => (Some(vis.passives[ps.id as usize]), ps.level, world.content.passives[ps.id as usize].max_level(), false),
                        None => (None, 0, 0, false),
                    }
                };
                let border = if evolved {
                    GOLD
                } else if icon.is_some() {
                    [1.0, 1.0, 1.0, 0.55]
                } else {
                    [1.0, 1.0, 1.0, 0.15]
                };
                frame_rect(list, white, pos, Vec2::splat(slot), border);
                // arme qui vient d'evoluer : halo rose qui pulse
                if let (0, Some((id, r)), Some(w)) = (row, self.evo_flash, world.weapons.get(k)) {
                    if w.id == id {
                        let a = r.min(1.0) * (0.55 + 0.45 * (self.t * 8.0).sin());
                        list.set(Space::Ui, Blend::Add);
                        for g in 1..4 {
                            let o = g as f32;
                            frame_rect(list, white, pos - Vec2::splat(o), Vec2::splat(slot + 2.0 * o), [1.0, 0.5, 0.95, a / o]);
                        }
                        list.set(Space::Ui, Blend::Alpha);
                    }
                }
                if let Some(ic) = icon {
                    list.frame(atlas.frame(ic, 0.0), pos + Vec2::splat(slot * 0.5), &Params::default());
                    if !evolved {
                        let c = if level >= max { GOLD } else { WHITE };
                        text(list, hudf, &level.to_string(), pos + Vec2::new(slot - 1.0, slot - 8.0), 1.0, c, Align::Right);
                    }
                }
            }
            y += slot + 2.0;
        }

        // reliques actives (equipees) : cases dorees, plus petites, sous les objets
        if !self.relics.is_empty() {
            let rs = 18.0;
            for (k, ic) in self.relics.iter().enumerate() {
                let pos = Vec2::new(8.0 + k as f32 * (rs + 2.0), y + 1.0);
                list.rect(white, pos, Vec2::splat(rs), [0.16, 0.11, 0.02, 0.75]);
                frame_rect(list, white, pos, Vec2::splat(rs), with_alpha(GOLD, 0.8));
                list.frame(atlas.frame(*ic, 0.0), pos + Vec2::splat(rs * 0.5), &Params::default());
            }
        }

        // chronometre
        let secs = world.time as u32;
        let tt = format!("{:02}:{:02}", secs / 60, secs % 60);
        if !self.busy || self.paused {
            text(list, main, &tt, Vec2::new(view.x * 0.5, 9.0), 2.0, WHITE, Align::Center);
        }

        // niveau et ennemis vaincus
        let lv = format!("{} {}", lang.get("ui.level"), p.level);
        text(list, main, &lv, Vec2::new(view.x - 8.0, 9.0), 2.0, GOLD, Align::Right);
        let k = format!("{}", p.kills);
        let w = text(list, hudf, &k, Vec2::new(view.x - 8.0, 40.0), 2.0, WHITE, Align::Right);
        let skull = with_alpha(WHITE, 0.85);
        let sx = view.x - 20.0 - w;
        list.rect(white, Vec2::new(sx, 41.0), Vec2::new(7.0, 6.0), skull);
        list.rect(white, Vec2::new(sx + 1.0, 47.0), Vec2::new(5.0, 3.0), skull);
        list.rect(white, Vec2::new(sx + 1.0, 43.0), Vec2::new(2.0, 2.0), [0.0, 0.0, 0.0, 0.9]);
        list.rect(white, Vec2::new(sx + 4.0, 43.0), Vec2::new(2.0, 2.0), [0.0, 0.0, 0.0, 0.9]);

        // boss : nom et barre de vie
        if let (Some(i), false) = (world.boss_index(), self.busy) {
            let kind = world.horde.kind[i] as usize;
            let key = format!("enemy.{}", world.content.enemy_ids[kind]);
            let f = (world.horde.hp[i] / world.horde.max_hp[i]).clamp(0.0, 1.0);
            let bw = (view.x * 0.45).round();
            let pos = Vec2::new(((view.x - bw) * 0.5).round(), 52.0);
            text(list, main, lang.get(&key), Vec2::new(view.x * 0.5, 38.0), 1.0, rgb(255, 200, 200), Align::Center);
            list.rect(white, pos - Vec2::splat(2.0), Vec2::new(bw + 4.0, 10.0), [0.0, 0.0, 0.0, 0.8]);
            list.rect(white, pos, Vec2::new(bw, 6.0), [0.25, 0.05, 0.08, 1.0]);
            list.rect(white, pos, Vec2::new((bw * self.boss_trail).round(), 6.0), [1.0, 1.0, 1.0, 0.85]);
            list.rect(white, pos, Vec2::new((bw * f).round(), 6.0), HP_RED);
            list.rect(white, pos, Vec2::new((bw * f).round(), 2.0), rgb(255, 130, 130));
            // reperes tous les 10 %
            for s in 1..10 {
                list.rect(white, pos + Vec2::new((bw * s as f32 / 10.0).round(), 0.0), Vec2::new(1.0, 6.0), [0.0, 0.0, 0.0, 0.5]);
            }
        }

        // mode d'attaque
        let mode = if p.auto_attack { lang.get("ui.auto") } else { lang.get("ui.manual") };
        let a = if self.mode_banner > 0.0 { 1.0 } else { 0.7 };
        let sc = if self.mode_banner > 1.2 { 2.0 } else { 1.0 };
        let my = view.y - 8.0 - 14.0 * sc;
        let mw = text(list, main, mode, Vec2::new(8.0, my), sc, with_alpha(if p.auto_attack { GOLD } else { WHITE }, a), Align::Left);
        // le mode ne concerne que l'epee : son icone a cote (grisee si le personnage n'en a pas)
        let sword = world.weapons.iter().find(|w| world.content.weapons[w.id as usize].kind == sim::defs::WeaponKind::Swing).map(|w| (w.id, true));
        let sword = sword.or_else(|| world.content.weapon_index("sword").map(|i| (i, false)));
        let mut hx = 8.0 + mw;
        if let Some((id, owned)) = sword {
            let c = if owned { [1.0, 1.0, 1.0, a] } else { [0.6, 0.6, 0.6, 0.45 * a] };
            // centre de l'icone sur le centre visuel des lettres (hauteur des capitales ~9 px), 4 px d'ecart
            let f = atlas.frame(vis.weapons[id as usize].icon, 0.0);
            let half_w = (f.w as f32 * 0.5).ceil();
            let at = Vec2::new((8.0 + mw + (4.0 + half_w) * sc).round(), (my + 4.0 * sc).round());
            list.frame(f, at, &Params::default().scale(Vec2::splat(sc)).color(c));
            list.probe_block(at - Vec2::splat(8.0 * sc), Vec2::splat(16.0 * sc), "icone du mode d'attaque");
            hx = at.x + half_w * sc;
        }
        // raccourcis de l'appareil utilise (premiere touche liee) : bascule du mode, puis dash (bottes)
        let dev = ctx.input.last_device;
        if dev != Device::Touch && !self.busy {
            let pad = dev == Device::Gamepad;
            let first = |act: &str| {
                let b = bindings.actions.get(act)?.iter().find(|b| matches!(b, Binding::Pad(_)) == pad)?;
                Some(crate::controls::binding_name(&ctx.input, lang, b))
            };
            let cy = (my + 4.0 * sc).round();
            let cap = |list: &mut DrawList, x: f32, s: &str| -> f32 {
                let w = main.measure(s) as f32;
                list.rect(white, Vec2::new(x, cy - 7.0), Vec2::new(w + 6.0, 14.0), [0.0, 0.0, 0.0, 0.7 * a]);
                text(list, main, s, Vec2::new(x + 3.0, cy - 5.0), 1.0, with_alpha(WHITE, 0.8 * a), Align::Left);
                w + 6.0
            };
            let mut x = hx + 6.0;
            if let Some(s) = first("toggle_auto") {
                x += cap(list, x, &s) + 12.0;
            }
            if let (Some(boots), Some(s)) = (atlas.find("icon_boots", "0"), first("dash")) {
                let f = atlas.frame(boots, 0.0);
                let hw = (f.w as f32 * 0.5).ceil();
                list.frame(f, Vec2::new(x + hw, cy), &Params::default().color([1.0, 1.0, 1.0, a]));
                cap(list, x + hw * 2.0 + 4.0, &s);
            }
        }

        // annonce
        if self.busy {
            return;
        }
        if let Some(tt) = self.toasts.first() {
            self.draw_toast(list, ctx, vis, tt);
        }
        if let Some(b) = &self.banner {
            // bandeau qui s'ouvre depuis le centre, texte qui glisse en fondu ; sortie vers le haut
            let ease = |x: f32| 1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3);
            let open = ease(b.t / 0.25);
            let out = ease((b.t - (b.life - 0.35)) / 0.35);
            let a = open * (1.0 - out);
            let sc = if main.measure(&b.text) as f32 * 2.0 <= view.x - 48.0 { 2.0 } else { 1.0 };
            // bandeau centre sur l'encre des majuscules (lettre A de la police : decalage `nudge` et hauteur,
            // ombre comprise) ; les jambages des p, g... descendent dans la marge du bas (mesure au pixel)
            let (ink_top, ink_h) = main.glyph('A').map_or((0.0, main.line_height as f32), |g| ((main.nudge + g.oy) as f32, g.h as f32));
            let pad = 6.0;
            let th = ink_h * sc + pad * 2.0;
            let tw = main.measure(&b.text) as f32 * sc;
            let cy = (view.y * 0.26).round() + 6.0;
            let bw = ((tw + 48.0) * open).round();
            let bp = Vec2::new(((view.x - bw) * 0.5).round(), (cy + ink_top * sc - pad - out * 6.0).round());
            list.rect(white, bp, Vec2::new(bw, th), [0.0, 0.0, 0.0, 0.45 * a]);
            list.rect(white, bp, Vec2::new(bw, 1.0), with_alpha(b.color, 0.8 * a));
            list.rect(white, bp + Vec2::new(0.0, th - 1.0), Vec2::new(bw, 1.0), with_alpha(b.color, 0.8 * a));
            let dy = ((1.0 - open) * 6.0 - out * 6.0).round();
            text(list, main, &b.text, Vec2::new(view.x * 0.5, cy + dy), sc, with_alpha(b.color, a), Align::Center);
        }
    }
}

impl Hud {
    /// Panneau de deblocage : monte depuis le bas (le haut est pris par les armes et la barre du boss),
    /// icone dans un cadre qui scintille.
    fn draw_toast(&self, list: &mut DrawList, ctx: &Ctx, vis: &Vis, tt: &Toast) {
        let view = ctx.view;
        let atlas = &ctx.atlas;
        let main = atlas.font("main");
        let white = &vis.white;
        let slide = (tt.t / 0.3).min(1.0).min(((tt.life - tt.t) / 0.4).max(0.0));
        let e = 1.0 - (1.0 - slide) * (1.0 - slide);
        let tw = [&tt.title, &tt.name, &tt.sub].iter().map(|s| main.measure(s) as f32).fold(0.0, f32::max);
        let size = Vec2::new((tw + 62.0).round(), 48.0);
        let pos = Vec2::new(((view.x - size.x) * 0.5).round(), (view.y - 90.0 + (1.0 - e) * 100.0).round());
        let green = rgb(120, 255, 150);
        list.rect(white, pos, size, BACK);
        let pulse = 0.6 + 0.4 * (self.t * 5.0).sin();
        frame_rect(list, white, pos, size, with_alpha(GOLD, pulse));
        list.set(Space::Ui, Blend::Add);
        frame_rect(list, white, pos - Vec2::ONE, size + Vec2::splat(2.0), with_alpha(GOLD, pulse * 0.4));
        // reflet qui balaie le panneau
        let sweep = ((self.t * 0.7).fract() * (size.x + 40.0)) - 20.0;
        for i in 0..5 {
            let x = (pos.x + sweep + i as f32 * 3.0).clamp(pos.x, pos.x + size.x - 2.0);
            list.rect(white, Vec2::new(x.round(), pos.y + 1.0), Vec2::new(2.0, size.y - 2.0), [1.0, 0.95, 0.6, 0.06]);
        }
        list.set(Space::Ui, Blend::Alpha);
        // icone x2 avec etincelles qui tournent autour
        let ib = Vec2::new(36.0, 36.0);
        let ip = pos + Vec2::new(6.0, 6.0);
        list.rect(white, ip, ib, [0.0, 0.0, 0.0, 0.5]);
        frame_rect(list, white, ip, ib, with_alpha(green, 0.7));
        list.frame(atlas.frame(tt.icon, 0.0), ip + ib * 0.5, &Params::default().scale(Vec2::splat(2.0)));
        list.set(Space::Ui, Blend::Add);
        for k in 0..6 {
            let a = self.t * 2.5 + k as f32 * std::f32::consts::TAU / 6.0;
            let p = ip + ib * 0.5 + Vec2::new(a.cos() * 21.0, a.sin() * 21.0);
            list.rect(white, (p - Vec2::ONE).round(), Vec2::splat(2.0), [0.7, 1.0, 0.8, 0.8]);
        }
        list.set(Space::Ui, Blend::Alpha);
        let tx = ip.x + ib.x + 8.0;
        text(list, main, &tt.title, Vec2::new(tx, pos.y + 3.0), 1.0, GOLD, Align::Left);
        text(list, main, &hl_new(&tt.name), Vec2::new(tx, pos.y + 17.0), 1.0, WHITE, Align::Left);
        text(list, main, &tt.sub, Vec2::new(tx, pos.y + 31.0), 1.0, rgb(200, 200, 220), Align::Left);
    }
}

/// Contour d'un rectangle (1 pixel).
pub fn frame_rect(list: &mut DrawList, white: &assetkit::meta::Frame, pos: Vec2, size: Vec2, c: Color) {
    list.probe_panel(pos, size, "cadre");
    list.rect(white, pos, Vec2::new(size.x, 1.0), c);
    list.rect(white, pos + Vec2::new(0.0, size.y - 1.0), Vec2::new(size.x, 1.0), c);
    list.rect(white, pos + Vec2::new(0.0, 1.0), Vec2::new(1.0, size.y - 2.0), c);
    list.rect(white, pos + Vec2::new(size.x - 1.0, 1.0), Vec2::new(1.0, size.y - 2.0), c);
}
