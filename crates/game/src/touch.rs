//! Commandes tactiles en partie (telephone, tablette, navigateur mobile) : un joystick apparait sous le
//! pouce sur la moitie gauche de l'ecran et reste a cet endroit tant que le doigt est pose ; sur la moitie droite, un second joystick vise (mode manuel :
//! l'epee frappe dans sa direction tant que le doigt est pose) ; boutons Esquive, Pause, Zoom et mode
//! d'attaque. Les menus se pilotent au doigt par la souris emulee du moteur (appui court = clic,
//! glisser = defilement).
use crate::hud::{text, GOLD};
use crate::lang::Lang;
use engine::app::Ctx;
use engine::draw::{with_alpha, Align, Color, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::input::{Device, Input};
use engine::Vec2;

/// Rayon des joysticks (pixels d'interface) : au-dela, la direction reste au maximum (le socle ne bouge pas).
const STICK_R: f32 = 28.0;

/// Boutons : centre et rayon (pixels d'interface).
pub struct Layout {
    pub dash: (Vec2, f32),
    pub mode: (Vec2, f32),
    pub pause: (Vec2, f32),
    pub zoom: (Vec2, f32),
}

/// Boutons a droite, sous le pouce droit ; Pause puis Zoom sous le compteur d'ennemis (coin haut droit).
pub fn layout(view: Vec2) -> Layout {
    Layout {
        dash: (Vec2::new(view.x - 50.0, view.y - 54.0), 24.0),
        mode: (Vec2::new(view.x - 104.0, view.y - 26.0), 15.0),
        pause: (Vec2::new(view.x - 22.0, 78.0), 14.0),
        zoom: (Vec2::new(view.x - 22.0, 112.0), 14.0),
    }
}

/// Bouton touche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Pause,
    Zoom,
    Dash,
    Mode,
}

impl Layout {
    /// Bouton sous le doigt (marge de 6 pixels : on vise vite avec le pouce). Les marges de Pause et de
    /// Zoom se recouvrent : le bouton le plus proche du doigt l'emporte.
    pub fn button_at(&self, p: Vec2) -> Option<Button> {
        [(self.pause, Button::Pause), (self.zoom, Button::Zoom), (self.dash, Button::Dash), (self.mode, Button::Mode)]
            .into_iter()
            .filter(|((c, r), _)| p.distance(*c) <= r + 6.0)
            .min_by(|a, b| (p.distance(a.0 .0) - a.0 .1).total_cmp(&(p.distance(b.0 .0) - b.0 .1)))
            .map(|(_, b)| b)
    }
}

/// Joystick : doigt qui le tient et centre (pixels d'interface).
type Stick = Option<(u64, Vec2)>;

#[derive(Default)]
pub struct Touch {
    mv: Stick,
    aim: Stick,
    /// Appuis en attente du prochain pas de simulation.
    dash: bool,
    toggle: bool,
    /// Zoom touche : lu (et remis a faux) par le jeu.
    pub zoom: bool,
    /// Dernieres commandes venues de l'ecran tactile : on dessine les boutons.
    pub active: bool,
}

/// Ce que l'ecran tactile donne a un pas de simulation.
pub struct Sticks {
    pub mv: Vec2,
    pub aim: Option<Vec2>,
    pub dash: bool,
    pub toggle: bool,
}

impl Touch {
    /// Une fois par image : doigts poses -> joysticks et boutons. Renvoie vrai si Pause vient d'etre touche.
    pub fn update(&mut self, input: &Input, view: Vec2, scale: f32, playing: bool) -> bool {
        self.active = input.last_device == Device::Touch;
        if !playing {
            *self = Touch { active: self.active, ..Touch::default() };
            return false;
        }
        let l = layout(view);
        let mut pause = false;
        for &(id, start) in input.touch_started() {
            let p = start / scale;
            match l.button_at(p) {
                Some(Button::Pause) => pause = true,
                Some(Button::Zoom) => self.zoom = true,
                Some(Button::Dash) => self.dash = true,
                Some(Button::Mode) => self.toggle = true,
                // doigt deja leve : un joystick n'a rien a suivre
                None if !input.touches.iter().any(|t| t.id == id) => {}
                None if p.x < view.x * 0.5 => self.mv = self.mv.or(Some((id, p))),
                None => self.aim = self.aim.or(Some((id, p))),
            }
        }
        // doigt leve : plus de joystick (sinon le socle reste ou le doigt s'est pose)
        for s in [&mut self.mv, &mut self.aim] {
            if s.is_some_and(|(id, _)| !input.touches.iter().any(|t| t.id == id)) {
                *s = None;
            }
        }
        pause
    }

    fn dir(&self, s: Stick, input: &Input, scale: f32) -> Vec2 {
        let Some((id, o)) = s else { return Vec2::ZERO };
        let Some(t) = input.touches.iter().find(|t| t.id == id) else { return Vec2::ZERO };
        let d = (t.pos / scale - o) / STICK_R;
        if d.length() < 0.15 {
            Vec2::ZERO
        } else {
            d.clamp_length_max(1.0)
        }
    }

    /// Un pas de simulation : direction, visee, appuis (consommes).
    pub fn read(&mut self, input: &Input, scale: f32) -> Sticks {
        let aim = self.dir(self.aim, input, scale);
        let out = Sticks {
            mv: self.dir(self.mv, input, scale),
            aim: (aim != Vec2::ZERO).then_some(aim),
            dash: self.dash,
            toggle: self.toggle,
        };
        self.dash = false;
        self.toggle = false;
        out
    }

    /// Boutons et joysticks, discrets (le jeu reste lisible dessous).
    pub fn draw(&self, list: &mut DrawList, ctx: &Ctx, lang: &Lang, auto: bool) {
        if !self.active {
            return;
        }
        let white = &ctx.atlas.white;
        let main = ctx.atlas.font("main");
        let input = &ctx.input;
        let scale = ctx.scale as f32;
        list.set(Space::Ui, Blend::Alpha);
        let l = layout(ctx.view);
        let held = |c: Vec2, r: f32| input.touches.iter().any(|t| (t.pos / scale).distance(c) <= r + 6.0);
        // Esquive
        let (c, r) = l.dash;
        let on = held(c, r);
        disc(list, white, c, r, [0.0, 0.0, 0.0, if on { 0.5 } else { 0.3 }]);
        ring(list, white, c, r, with_alpha(WHITE, if on { 0.9 } else { 0.55 }));
        let label = lang.get("act.dash");
        if main.measure(label) as f32 <= r * 2.0 - 6.0 {
            text(list, main, label, Vec2::new(c.x, crate::hud::mid_y(main, c.y - r, r * 2.0, 1.0)), 1.0, with_alpha(WHITE, 0.85), Align::Center);
        } else {
            arrow(list, white, c, with_alpha(WHITE, 0.85));
        }
        // mode d'attaque
        let (c, r) = l.mode;
        disc(list, white, c, r, [0.0, 0.0, 0.0, 0.3]);
        ring(list, white, c, r, with_alpha(if auto { GOLD } else { WHITE }, 0.6));
        let m = if auto { "A" } else { "M" };
        text(list, main, m, Vec2::new(c.x, crate::hud::mid_y(main, c.y - r, r * 2.0, 1.0)), 1.0, with_alpha(if auto { GOLD } else { WHITE }, 0.85), Align::Center);
        // Pause : deux barres
        let (c, r) = l.pause;
        disc(list, white, c, r, [0.0, 0.0, 0.0, 0.3]);
        ring(list, white, c, r, with_alpha(WHITE, 0.55));
        for dx in [-4.0, 2.0] {
            list.rect(white, (c + Vec2::new(dx, -5.0)).round(), Vec2::new(2.0, 10.0), with_alpha(WHITE, 0.85));
        }
        // Zoom : loupe
        let (c, r) = l.zoom;
        disc(list, white, c, r, [0.0, 0.0, 0.0, 0.3]);
        ring(list, white, c, r, with_alpha(WHITE, 0.55));
        ring(list, white, c - Vec2::splat(2.0), 6.0, with_alpha(WHITE, 0.85));
        for k in 0..3 {
            list.rect(white, (c + Vec2::splat(2.0 + k as f32)).round(), Vec2::splat(2.0), with_alpha(WHITE, 0.85));
        }
        // joysticks poses : socle et bouton sous le pouce
        for s in [self.mv, self.aim] {
            let Some((id, o)) = s else { continue };
            let Some(t) = input.touches.iter().find(|t| t.id == id) else { continue };
            ring(list, white, o, STICK_R, with_alpha(WHITE, 0.35));
            let k = o + (t.pos / scale - o).clamp_length_max(STICK_R);
            disc(list, white, k, 11.0, with_alpha(WHITE, 0.35));
        }
    }
}

/// Disque plein, ligne par ligne.
fn disc(list: &mut DrawList, white: &assetkit::meta::Frame, c: Vec2, r: f32, color: Color) {
    let c = c.round();
    let n = r as i32;
    for dy in -n..n {
        let y = dy as f32 + 0.5;
        let half = (r * r - y * y).max(0.0).sqrt().round();
        if half > 0.0 {
            list.rect(white, Vec2::new(c.x - half, c.y + dy as f32), Vec2::new(half * 2.0, 1.0), color);
        }
    }
}

/// Cercle de 2 pixels d'epaisseur.
fn ring(list: &mut DrawList, white: &assetkit::meta::Frame, c: Vec2, r: f32, color: Color) {
    let c = c.round();
    let n = r as i32;
    for dy in -n..n {
        let y = dy as f32 + 0.5;
        let outer = (r * r - y * y).max(0.0).sqrt().round();
        let inner = ((r - 2.0) * (r - 2.0) - y * y).max(0.0).sqrt().round();
        let w = (outer - inner).max(1.0);
        for x in [c.x - outer, c.x + inner] {
            list.rect(white, Vec2::new(x, c.y + dy as f32), Vec2::new(w, 1.0), color);
        }
    }
}

/// Double chevron (esquive) quand le mot ne tient pas dans le bouton.
fn arrow(list: &mut DrawList, white: &assetkit::meta::Frame, c: Vec2, color: Color) {
    for x in [-6.0f32, 1.0] {
        for i in 0..5 {
            let h = 9.0 - i as f32 * 2.0;
            list.rect(white, (c + Vec2::new(x + i as f32, -h * 0.5)).round(), Vec2::new(1.0, h), color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Haut de la loupe (dans la marge de Pause aussi) : c'est le Zoom ; centres et bords bien a eux.
    #[test]
    fn nearest_button_wins() {
        let view = Vec2::new(640.0, 360.0);
        let l = layout(view);
        let x = view.x - 22.0;
        assert_eq!(l.button_at(Vec2::new(x, 78.0)), Some(Button::Pause));
        assert_eq!(l.button_at(Vec2::new(x, 112.0)), Some(Button::Zoom));
        assert_eq!(l.button_at(Vec2::new(x, 98.0)), Some(Button::Zoom));
        assert_eq!(l.button_at(Vec2::new(x, 93.0)), Some(Button::Pause));
        assert_eq!(l.button_at(Vec2::new(100.0, 200.0)), None);
    }

    /// Le socle du joystick reste ou le doigt s'est pose, meme quand le doigt va plus loin que son rayon ;
    /// doigt leve : plus de joystick.
    #[test]
    fn stick_stays_put() {
        let view = Vec2::new(640.0, 360.0);
        let o = Vec2::new(80.0, 250.0);
        let mut input = Input::new();
        input.touches.push(engine::input::TouchPt { id: 1, start: o, pos: o + Vec2::new(STICK_R * 3.0, 0.0) });
        let mut t = Touch { mv: Some((1, o)), ..Touch::default() };
        t.update(&input, view, 1.0, true);
        assert_eq!(t.mv, Some((1, o)));
        assert_eq!(t.read(&input, 1.0).mv, Vec2::X);
        input.touches.clear();
        t.update(&input, view, 1.0, true);
        assert_eq!(t.mv, None);
    }
}
