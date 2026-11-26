//! Ecran Joueurs (5 emplacements, facon ecran de selection d'A Link to the Past) et saisie d'un nom
//! (grille de lettres pour la manette et la souris, frappe directe au clavier).
use crate::hud::{frame_rect, text, BACK, GOLD};
use crate::lang::Lang;
use crate::menus::NavIn;
use crate::profile::{Profile, SLOTS};
use engine::app::Ctx;
use engine::assets::{AnimId, Atlas};
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::input::{Device, KeyCode, PadButton};
use engine::Vec2;

const SOFT: Color = rgb(210, 210, 236);
const DIMMED: Color = rgb(120, 116, 140);
const RED: Color = rgb(255, 120, 110);

/// Avatars : personnages et monstres d'A Link to the Past deja presents dans l'atlas (sprite, animation).
pub const AVATARS: [(&str, &str); 20] = [
    ("link", "idle_down"),
    ("fairy", "0"),
    ("soldier_green", "walk_down"),
    ("soldier_blue", "walk_down"),
    ("archer", "walk_down"),
    ("octorok", "walk_down"),
    ("moblin", "walk_down"),
    ("rope", "walk_down"),
    ("rat", "walk_down"),
    ("keese", "fly"),
    ("crow", "idle"),
    ("tektite", "idle"),
    ("buzz_blob", "walk"),
    ("popo", "walk"),
    ("hardhat", "walk"),
    ("stalfos", "walk_down"),
    ("lynel", "walk_down"),
    ("pikit", "idle"),
    ("hinox", "walk_down"),
    ("agahnim", "walk_down"),
];

pub fn avatar_anim(atlas: &Atlas, k: usize) -> Option<AnimId> {
    let (s, a) = AVATARS[k % AVATARS.len()];
    atlas.find(s, a)
}

/// Dessine un avatar centre dans une case carree (x2 pour les petits sprites).
pub fn draw_avatar(list: &mut DrawList, atlas: &Atlas, k: usize, center: Vec2, side: f32, t: f32, color: Color) {
    let Some(a) = avatar_anim(atlas, k) else { return };
    let f = atlas.frame(a, t);
    let big = f.w.max(f.h) as f32;
    let sc = if big * 2.0 <= side { 2.0 } else { 1.0 };
    // les sprites sont cales sur leur pivot : on recentre la boite visible
    let off = Vec2::new(f.px as f32 - f.w as f32 * 0.5, f.py as f32 - f.h as f32 * 0.5) * sc;
    list.frame(f, (center + off).round(), &Params::default().scale(Vec2::splat(sc)).color(color));
}

/// Resume d'un emplacement pour l'ecran Joueurs.
pub struct SlotView {
    pub name: String,
    pub avatar: usize,
    pub won: usize,
    pub time: f64,
    pub kills: u64,
}

impl SlotView {
    pub fn of(p: &Profile) -> SlotView {
        SlotView {
            name: p.name.clone(),
            avatar: p.avatar,
            won: p.stages.values().filter(|r| r.wins > 0).count(),
            time: p.totals.time,
            kills: p.totals.kills,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Ecran Joueurs

/// Lignes : les 5 emplacements, puis Supprimer et Retour.
pub const DELETE: usize = SLOTS;
pub const BACK_ITEM: usize = SLOTS + 1;

pub struct Layout {
    pub title_y: f32,
    pub cards: Vec<(Vec2, Vec2)>,
    pub hint_y: f32,
    pub delete: (Vec2, Vec2),
    pub back: (Vec2, Vec2),
}

pub fn layout(view: Vec2) -> Layout {
    let w = (view.x - 32.0).min(440.0).round();
    let x = ((view.x - w) * 0.5).round();
    let h = 46.0;
    let gap = 4.0;
    let total = SLOTS as f32 * (h + gap) - gap;
    let y0 = (36.0f32).max(((view.y - total - 70.0) * 0.5).round());
    let cards = (0..SLOTS).map(|k| (Vec2::new(x, y0 + k as f32 * (h + gap)), Vec2::new(w, h))).collect();
    let hint_y = y0 + total + 6.0;
    let bw = 150.0;
    let by = view.y - 28.0;
    let delete = (Vec2::new((view.x * 0.5 - bw - 6.0).round(), by), Vec2::new(bw, 20.0));
    let back = (Vec2::new((view.x * 0.5 + 6.0).round(), by), Vec2::new(bw, 20.0));
    Layout { title_y: (y0 - 30.0).max(6.0), cards, hint_y, delete, back }
}

/// Zones des fleches de changement d'avatar sur une carte.
fn avatar_arrows(card: (Vec2, Vec2)) -> [(Vec2, Vec2); 2] {
    let (p, s) = card;
    let ab = avatar_box(card);
    [(Vec2::new(p.x + 1.0, p.y + 1.0), Vec2::new(ab.0.x - p.x - 1.0, s.y - 2.0)), (Vec2::new(ab.0.x + ab.1.x, p.y + 1.0), Vec2::new(12.0, s.y - 2.0))]
}

fn avatar_box(card: (Vec2, Vec2)) -> (Vec2, Vec2) {
    let (p, s) = card;
    (Vec2::new(p.x + 14.0, p.y + 3.0), Vec2::splat(s.y - 6.0))
}

fn inside(m: Vec2, (p, s): (Vec2, Vec2)) -> bool {
    m.x >= p.x && m.y >= p.y && m.x < p.x + s.x && m.y < p.y + s.y
}

pub enum Action {
    None,
    Moved,
    /// Valider un emplacement (creer, choisir, renommer).
    Pick(usize),
    /// Changer l'avatar du joueur actif (-1 / +1).
    Avatar(i32),
    Delete,
    Back,
}

pub fn input(nav: &NavIn, view: Vec2, active: usize, sel: &mut usize) -> Action {
    let l = layout(view);
    let n = SLOTS + 2;
    if nav.back {
        return Action::Back;
    }
    let mut rects: Vec<(Vec2, Vec2)> = l.cards.clone();
    rects.push(l.delete);
    rects.push(l.back);
    if let Some(m) = nav.hover {
        for (k, &r) in rects.iter().enumerate() {
            if inside(m, r) && *sel != k {
                *sel = k;
                return Action::Moved;
            }
        }
    }
    if nav.click {
        if let Some(&card) = l.cards.get(active) {
            let [a, b] = avatar_arrows(card);
            if inside(nav.mouse, a) {
                *sel = active;
                return Action::Avatar(-1);
            }
            if inside(nav.mouse, b) {
                *sel = active;
                return Action::Avatar(1);
            }
        }
        for (k, &r) in rects.iter().enumerate() {
            if inside(nav.mouse, r) {
                *sel = k;
                return match k {
                    DELETE => Action::Delete,
                    BACK_ITEM => Action::Back,
                    _ => Action::Pick(k),
                };
            }
        }
    }
    if nav.up {
        *sel = (*sel + n - 1) % n;
        return Action::Moved;
    }
    if nav.down {
        *sel = (*sel + 1) % n;
        return Action::Moved;
    }
    if nav.left || nav.right {
        let d = if nav.right { 1 } else { -1 };
        if *sel == active {
            return Action::Avatar(d);
        }
        if *sel >= DELETE {
            *sel = if *sel == DELETE { BACK_ITEM } else { DELETE };
            return Action::Moved;
        }
    }
    if nav.confirm {
        return match *sel {
            DELETE => Action::Delete,
            BACK_ITEM => Action::Back,
            k => Action::Pick(k),
        };
    }
    Action::None
}

fn hours(s: f64) -> String {
    let m = (s / 60.0) as u64;
    format!("{}:{:02}", m / 60, m % 60)
}

#[allow(clippy::too_many_arguments)]
pub fn draw(list: &mut DrawList, ctx: &Ctx, lang: &Lang, slots: &[Option<SlotView>], active: usize, sel: usize, armed: bool, stages: usize, toast: &str, t: f32) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    let rt = ctx.real_time as f32;
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 0.6]);
    let l = layout(view);
    text(list, main, lang.get("ui.players"), Vec2::new(view.x * 0.5, l.title_y), 2.0, WHITE, Align::Center);
    for (k, &(p, s)) in l.cards.iter().enumerate() {
        let selected = k == sel;
        let is_active = k == active;
        list.rect(white, p, s, with_alpha(BACK, if selected { 0.96 } else { 0.84 }));
        let border = if selected {
            with_alpha(GOLD, 0.75 + 0.25 * (rt * 6.0).sin())
        } else if is_active {
            with_alpha(GOLD, 0.45)
        } else {
            [1.0, 1.0, 1.0, 0.22]
        };
        frame_rect(list, white, p, s, border);
        if selected {
            crate::hub::focus_frame(list, ctx, p, s);
        }
        list.probe_panel(p, s, "carte de joueur");
        let num = format!("{}", k + 1);
        text(list, main, &num, Vec2::new(p.x + 4.0, p.y + 4.0), 1.0, if is_active { GOLD } else { DIMMED }, Align::Left);
        let (ap, asz) = avatar_box((p, s));
        list.rect(white, ap, asz, [0.0, 0.0, 0.0, 0.4]);
        frame_rect(list, white, ap, asz, [1.0, 1.0, 1.0, 0.2]);
        let tx = ap.x + asz.x + 14.0;
        match slots.get(k).and_then(|o| o.as_ref()) {
            Some(v) => {
                draw_avatar(list, atlas, v.avatar, ap + asz * 0.5, asz.x, t, WHITE);
                if is_active && selected {
                    // fleches de changement d'avatar
                    let blink = (rt * 3.0).fract() < 0.7;
                    let c = with_alpha(GOLD, if blink { 1.0 } else { 0.5 });
                    text(list, main, "◀", Vec2::new(ap.x - 10.0, ap.y + asz.y * 0.5 - 6.0), 1.0, c, Align::Left);
                    text(list, main, "▶", Vec2::new(ap.x + asz.x + 2.0, ap.y + asz.y * 0.5 - 6.0), 1.0, c, Align::Left);
                }
                text(list, main, &v.name, Vec2::new(tx, p.y + 5.0), 2.0, if selected { GOLD } else { WHITE }, Align::Left);
                let line = lang
                    .get("ui.player_summary")
                    .replace("{w}", &v.won.to_string())
                    .replace("{n}", &stages.to_string())
                    .replace("{t}", &hours(v.time))
                    .replace("{k}", &v.kills.to_string());
                text(list, main, &line, Vec2::new(tx, p.y + 30.0), 1.0, SOFT, Align::Left);
                if is_active {
                    text(list, main, lang.get("ui.active"), Vec2::new(p.x + s.x - 8.0, p.y + 6.0), 1.0, GOLD, Align::Right);
                }
            }
            None => {
                text(list, main, lang.get("ui.free_slot"), Vec2::new(tx, p.y + 9.0), 1.0, DIMMED, Align::Left);
                text(list, main, lang.get("ui.create_player"), Vec2::new(tx, p.y + 25.0), 1.0, if selected { GOLD } else { SOFT }, Align::Left);
            }
        }
    }
    // aide de la ligne choisie (ou message)
    let hint = if !toast.is_empty() {
        toast.to_string()
    } else {
        match sel {
            DELETE if armed => lang.get("ui.delete_warn").to_string(),
            DELETE => lang.get("ui.delete_desc").to_string(),
            BACK_ITEM => String::new(),
            k if k == active => lang.get("ui.player_active_hint").to_string(),
            k if slots.get(k).is_some_and(|o| o.is_some()) => lang.get("ui.player_switch_hint").to_string(),
            _ => lang.get("ui.player_create_hint").to_string(),
        }
    };
    text(list, main, &hint, Vec2::new(view.x * 0.5, l.hint_y), 1.0, if armed && sel == DELETE { RED } else { SOFT }, Align::Center);
    for (k, (r, key)) in [(l.delete, if armed { "ui.confirm_again" } else { "ui.delete_player" }), (l.back, "ui.back")].into_iter().enumerate() {
        let selected = sel == DELETE + k;
        list.rect(white, r.0, r.1, with_alpha(BACK, 0.9));
        frame_rect(list, white, r.0, r.1, if selected { GOLD } else { [1.0, 1.0, 1.0, 0.3] });
        if selected {
            crate::hub::focus_frame(list, ctx, r.0, r.1);
        }
        let c = if k == 0 && armed { RED } else if selected { GOLD } else { SOFT };
        text(list, main, lang.get(key), Vec2::new(r.0.x + r.1.x * 0.5, r.0.y + 4.0), 1.0, c, Align::Center);
    }
}

// ---------------------------------------------------------------------------------------------
// Saisie d'un nom

/// Lettres proposees (aussi les seules acceptees au clavier).
/// (Seulement des caracteres de la police du jeu ; l'espace est la case marquee d'un crochet.)
pub const GRID: [&str; 7] = ["ABCDEFGHIJ", "KLMNOPQRST", "UVWXYZ-.!?", "abcdefghij", "klmnopqrst", "uvwxyz '+:", "0123456789"];
pub const MAX_LEN: usize = 12;
const COLS: usize = 10;
const ROWS: usize = 7;
/// Boutons sous la grille.
pub const ERASE: usize = COLS * ROWS;
pub const DONE: usize = COLS * ROWS + 1;

fn grid_char(k: usize) -> Option<char> {
    GRID.get(k / COLS)?.chars().nth(k % COLS)
}

pub fn allowed(c: char) -> bool {
    GRID.iter().any(|r| r.contains(c))
}

pub struct NameLayout {
    pub title_y: f32,
    pub field: (Vec2, Vec2),
    pub cells: Vec<(Vec2, Vec2)>,
    pub erase: (Vec2, Vec2),
    pub done: (Vec2, Vec2),
    pub hint_y: f32,
}

pub fn name_layout(view: Vec2) -> NameLayout {
    let cs = 20.0;
    let gw = cs * COLS as f32;
    let x0 = ((view.x - gw) * 0.5).round();
    let top = ((view.y - 300.0) * 0.5).max(4.0).round();
    let field = (Vec2::new(((view.x - 200.0) * 0.5).round(), top + 32.0), Vec2::new(200.0, 22.0));
    let gy = top + 66.0;
    let cells = (0..COLS * ROWS)
        .map(|k| (Vec2::new(x0 + (k % COLS) as f32 * cs, gy + (k / COLS) as f32 * cs), Vec2::splat(cs - 2.0)))
        .collect();
    let by = gy + ROWS as f32 * cs + 8.0;
    let bw = 96.0;
    let erase = (Vec2::new((view.x * 0.5 - bw - 4.0).round(), by), Vec2::new(bw, 20.0));
    let done = (Vec2::new((view.x * 0.5 + 4.0).round(), by), Vec2::new(bw, 20.0));
    NameLayout { title_y: top + 4.0, field, cells, erase, done, hint_y: by + 30.0 }
}

pub enum NameAction {
    None,
    Moved,
    Typed,
    Done,
    Cancel,
}

/// Saisie : clavier brut (les lettres ne sont pas des commandes de menu ici), manette, souris.
pub fn name_input(ctx: &Ctx, nav: &NavIn, text: &mut String, sel: &mut usize) -> NameAction {
    let i = &ctx.input;
    let l = name_layout(ctx.view);
    let n = DONE + 1;
    let pad = i.last_device == Device::Gamepad;
    let mut act = NameAction::None;
    // clavier
    for &c in &i.typed {
        if allowed(c) && text.chars().count() < MAX_LEN {
            text.push(c);
            act = NameAction::Typed;
        }
    }
    if i.key_pressed(KeyCode::Backspace) {
        text.pop();
        act = NameAction::Typed;
    }
    if i.key_pressed(KeyCode::Escape) {
        return NameAction::Cancel;
    }
    if i.key_pressed(KeyCode::Enter) || i.key_pressed(KeyCode::NumpadEnter) || i.pad_pressed(PadButton::Start) {
        return NameAction::Done;
    }
    // deplacement dans la grille : fleches du clavier, croix et stick de la manette
    let up = i.key_pressed(KeyCode::ArrowUp) || (pad && nav.up);
    let down = i.key_pressed(KeyCode::ArrowDown) || (pad && nav.down);
    let left = i.key_pressed(KeyCode::ArrowLeft) || (pad && nav.left);
    let right = i.key_pressed(KeyCode::ArrowRight) || (pad && nav.right);
    let s = *sel;
    if up || down {
        *sel = if s >= ERASE {
            if up {
                ERASE - COLS + if s == DONE { COLS - 1 } else { 0 }
            } else if s == ERASE {
                0
            } else {
                COLS - 1
            }
        } else {
            let r = s / COLS;
            let c = s % COLS;
            if up && r == 0 {
                if c < COLS / 2 { ERASE } else { DONE }
            } else if up {
                s - COLS
            } else if r + 1 == ROWS {
                if c < COLS / 2 { ERASE } else { DONE }
            } else {
                s + COLS
            }
        };
        return NameAction::Moved;
    }
    if left || right {
        *sel = if s >= ERASE {
            if s == ERASE { DONE } else { ERASE }
        } else {
            let r = s / COLS;
            let c = (s % COLS + if right { 1 } else { COLS - 1 }) % COLS;
            r * COLS + c
        };
        return NameAction::Moved;
    }
    // souris
    let mut rects = l.cells.clone();
    rects.push(l.erase);
    rects.push(l.done);
    if let Some(m) = nav.hover {
        for (k, &r) in rects.iter().enumerate() {
            if inside(m, r) && *sel != k {
                *sel = k;
                return NameAction::Moved;
            }
        }
    }
    let mut pick = None;
    if nav.click {
        pick = rects.iter().position(|&r| inside(nav.mouse, r));
    }
    // manette : A valide la case, B efface (ou annule si le nom est vide)
    if pad && i.pad_pressed(PadButton::South) {
        pick = Some(*sel);
    }
    if pad && i.pad_pressed(PadButton::East) {
        if text.is_empty() {
            return NameAction::Cancel;
        }
        text.pop();
        return NameAction::Typed;
    }
    if let Some(k) = pick.filter(|&k| k < n) {
        *sel = k;
        match k {
            ERASE => {
                text.pop();
                return NameAction::Typed;
            }
            DONE => return NameAction::Done,
            _ => {
                if let Some(c) = grid_char(k) {
                    if text.chars().count() < MAX_LEN {
                        text.push(c);
                    }
                }
                return NameAction::Typed;
            }
        }
    }
    act
}

#[allow(clippy::too_many_arguments)]
pub fn draw_name(list: &mut DrawList, ctx: &Ctx, lang: &Lang, title: &str, name: &str, sel: usize, avatar: Option<usize>, t: f32) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let white = &atlas.white;
    let main = atlas.font("main");
    let rt = ctx.real_time as f32;
    list.set(Space::Ui, Blend::Alpha);
    list.probe_layer();
    list.rect(white, Vec2::ZERO, view, [0.01, 0.01, 0.05, 0.82]);
    let l = name_layout(view);
    text(list, main, title, Vec2::new(view.x * 0.5, l.title_y), 2.0, WHITE, Align::Center);
    let (fp, fs) = l.field;
    list.rect(white, fp, fs, with_alpha(BACK, 0.95));
    frame_rect(list, white, fp, fs, with_alpha(GOLD, 0.8));
    if let Some(a) = avatar {
        draw_avatar(list, atlas, a, Vec2::new(fp.x - 22.0, fp.y + fs.y * 0.5), 20.0, t, WHITE);
    }
    let w = text(list, main, name, Vec2::new(fp.x + 8.0, fp.y + 5.0), 1.0, WHITE, Align::Left);
    if name.chars().count() < MAX_LEN && (rt * 2.0).fract() < 0.6 {
        list.rect(white, Vec2::new(fp.x + 9.0 + w, fp.y + 16.0), Vec2::new(6.0, 1.0), GOLD);
    }
    let count = format!("{}/{MAX_LEN}", name.chars().count());
    text(list, main, &count, Vec2::new(fp.x + fs.x - 6.0, fp.y + 5.0), 1.0, DIMMED, Align::Right);
    for (k, &(p, s)) in l.cells.iter().enumerate() {
        let selected = k == sel;
        list.rect(white, p, s, with_alpha(BACK, if selected { 0.95 } else { 0.7 }));
        if selected {
            crate::hub::focus_frame(list, ctx, p, s);
        }
        match grid_char(k) {
            // espace : petit crochet
            Some(' ') => {
                let c = if selected { GOLD } else { DIMMED };
                let (x0, y0) = ((p.x + s.x * 0.5 - 4.0).round(), (p.y + s.y - 6.0).round());
                list.rect(white, Vec2::new(x0, y0), Vec2::new(9.0, 1.0), c);
                list.rect(white, Vec2::new(x0, y0 - 2.0), Vec2::new(1.0, 2.0), c);
                list.rect(white, Vec2::new(x0 + 8.0, y0 - 2.0), Vec2::new(1.0, 2.0), c);
            }
            Some(c) => {
                let col = if selected { GOLD } else { SOFT };
                text(list, main, &c.to_string(), Vec2::new(p.x + s.x * 0.5, p.y + 3.0), 1.0, col, Align::Center);
            }
            None => {}
        }
    }
    for (k, r, key) in [(ERASE, l.erase, "ui.erase"), (DONE, l.done, "ui.done")] {
        let selected = sel == k;
        list.rect(white, r.0, r.1, with_alpha(BACK, 0.9));
        frame_rect(list, white, r.0, r.1, if selected { GOLD } else { [1.0, 1.0, 1.0, 0.3] });
        if selected {
            crate::hub::focus_frame(list, ctx, r.0, r.1);
        }
        text(list, main, lang.get(key), Vec2::new(r.0.x + r.1.x * 0.5, r.0.y + 4.0), 1.0, if selected { GOLD } else { SOFT }, Align::Center);
    }
    text(list, main, lang.get("ui.name_hint"), Vec2::new(view.x * 0.5, l.hint_y), 1.0, SOFT, Align::Center);
}
