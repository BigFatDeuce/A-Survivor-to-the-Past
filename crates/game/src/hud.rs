//! Interface en jeu : coeurs, rubis, experience, chronometre, armes et objets, boss, annonces,
//! et la jauge de vie en cases sous Link.
use crate::lang::Lang;
use crate::visuals::Vis;
use assetkit::meta::Font;
use engine::app::Ctx;
use engine::draw::{rgb, with_alpha, Align, Color, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;
use sim::World;

pub const XP_GREEN: Color = rgb(64, 216, 64);
pub const BACK: Color = [0.06, 0.04, 0.10, 0.85];
pub const GOLD: Color = rgb(248, 216, 88);
pub const HP_RED: Color = rgb(224, 48, 56);
const DASH_BLUE: Color = rgb(120, 200, 255);

/// Texte avec ombre a une echelle entiere.
pub fn text(list: &mut DrawList, font: &Font, s: &str, pos: Vec2, scale: f32, color: Color, align: Align) -> f32 {
    list.text_ex(font, s, pos, scale, color, align, Some([0.0, 0.0, 0.0, color[3] * 0.75]))
}

/// Decoupe un texte en lignes d'au plus `width` pixels (coupure aux espaces).
pub fn wrap(font: &Font, s: &str, width: f32, scale: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split(' ') {
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
    lines
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
}

impl Hud {
    pub fn new() -> Hud {
        Hud {
            shown_rupees: 0.0,
            rupee_pop: 0.0,
            heart_shake: 0.0,
            mode_banner: 0.0,
            banner: None,
            hp_trail: 0.0,
            boss_trail: 1.0,
            dash_ready: 0.0,
            dash_was_full: true,
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

    pub fn draw(&self, list: &mut DrawList, ctx: &Ctx, world: &World, vis: &Vis, lang: &Lang) {
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
        text(list, hudf, &txt, Vec2::new(24.0, y + 1.0), 2.0, WHITE, Align::Left);
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

        // chronometre
        let secs = world.time as u32;
        let tt = format!("{:02}:{:02}", secs / 60, secs % 60);
        text(list, main, &tt, Vec2::new(view.x * 0.5, 9.0), 2.0, WHITE, Align::Center);

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
        if let Some(i) = world.boss_index() {
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
        text(list, main, mode, Vec2::new(8.0, view.y - 8.0 - 14.0 * sc), sc, with_alpha(if p.auto_attack { GOLD } else { WHITE }, a), Align::Left);

        // annonce
        if let Some(b) = &self.banner {
            let pop = if b.t < 0.12 { 3.0 } else { 2.0 };
            let a = if b.t > b.life - 0.4 { ((b.life - b.t) / 0.4).max(0.0) } else { 1.0 };
            text(list, main, &b.text, Vec2::new(view.x * 0.5, (view.y * 0.26).round()), pop, with_alpha(b.color, a), Align::Center);
        }
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
