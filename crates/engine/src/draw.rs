//! Aides de dessin : sprites, rectangles, texte bitmap.
use crate::gfx::{DrawList, SpriteInstance};
use assetkit::meta::{Font, Frame};
use glam::Vec2;

pub type Color = [f32; 4];
pub const WHITE: Color = [1.0, 1.0, 1.0, 1.0];
pub const BLACK: Color = [0.0, 0.0, 0.0, 1.0];

/// Couleur depuis des composantes 0..255.
pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    [c[0], c[1], c[2], c[3] * a]
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub scale: Vec2,
    pub rot: f32,
    pub color: Color,
    pub flash: Color,
}

impl Default for Params {
    fn default() -> Self {
        Params { scale: Vec2::ONE, rot: 0.0, color: WHITE, flash: [0.0; 4] }
    }
}

impl Params {
    pub fn flip(mut self, flip_x: bool) -> Self {
        if flip_x {
            self.scale.x = -self.scale.x;
        }
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn flash(mut self, c: [f32; 3], amount: f32) -> Self {
        self.flash = [c[0], c[1], c[2], amount];
        self
    }
    pub fn scale(mut self, s: Vec2) -> Self {
        self.scale = s;
        self
    }
    pub fn rot(mut self, r: f32) -> Self {
        self.rot = r;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[inline]
pub fn instance(f: &Frame, pos: Vec2, p: &Params) -> SpriteInstance {
    SpriteInstance {
        pos: pos.to_array(),
        uv: [f.x as f32, f.y as f32, f.w as f32, f.h as f32],
        pivot: [f.px as f32, f.py as f32],
        scale: p.scale.to_array(),
        rot: p.rot,
        color: p.color,
        flash: p.flash,
    }
}

impl DrawList {
    #[inline]
    pub fn frame(&mut self, f: &Frame, pos: Vec2, p: &Params) {
        self.push(instance(f, pos, p));
    }

    /// Rectangle plein (le pixel blanc de l'atlas etire).
    pub fn rect(&mut self, white: &Frame, pos: Vec2, size: Vec2, color: Color) {
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let p = Params { scale: size, color, ..Default::default() };
        self.push(instance(white, pos, &p));
    }

    /// Texte sur une ligne ; renvoie la largeur dessinee.
    pub fn text(&mut self, font: &Font, s: &str, pos: Vec2, color: Color, align: Align) -> f32 {
        self.text_ex(font, s, pos, 1.0, color, align, None)
    }

    pub fn text_shadow(&mut self, font: &Font, s: &str, pos: Vec2, color: Color, align: Align) -> f32 {
        self.text_ex(font, s, pos, 1.0, color, align, Some([0.0, 0.0, 0.0, color[3] * 0.75]))
    }

    /// Texte agrandi d'un facteur (entier de preference), avec ombre facultative.
    /// Renvoie la largeur dessinee.
    #[allow(clippy::too_many_arguments)]
    pub fn text_ex(
        &mut self,
        font: &Font,
        s: &str,
        pos: Vec2,
        scale: f32,
        color: Color,
        align: Align,
        shadow: Option<Color>,
    ) -> f32 {
        let w = font.measure(s) as f32 * scale;
        let x0 = match align {
            Align::Left => pos.x,
            Align::Center => (pos.x - w * 0.5).round(),
            Align::Right => pos.x - w,
        };
        if self.probe.is_some() && self.space() == crate::gfx::Space::Ui {
            let size = Vec2::new(w, font.line_height as f32 * scale);
            let missing: Vec<char> = s.chars().filter(|&c| c != ' ' && font.glyph(c).is_none()).collect();
            let probe = self.probe.as_mut().unwrap();
            probe.push(crate::gfx::Probe::Text { pos: Vec2::new(x0, pos.y), size, text: s.to_string() });
            for ch in missing {
                probe.push(crate::gfx::Probe::MissingGlyph { ch, text: s.to_string() });
            }
        }
        let passes: &[(Option<Color>, f32)] = &[(shadow, scale.max(1.0)), (Some(color), 0.0)];
        for &(col, off) in passes {
            let Some(col) = col else { continue };
            let p = Params { color: col, scale: Vec2::splat(scale), ..Default::default() };
            let mut x = x0 + off;
            let mut first = true;
            for c in s.chars() {
                if !first {
                    x += font.spacing as f32 * scale;
                }
                first = false;
                match font.glyph(c) {
                    Some(g) => {
                        let f = Frame { x: g.x, y: g.y, w: g.w, h: g.h, px: 0, py: 0 };
                        self.push(instance(&f, Vec2::new(x, pos.y + off + g.oy as f32 * scale), &p));
                        x += g.w as f32 * scale;
                    }
                    None => x += font.space as f32 * scale,
                }
            }
        }
        w
    }
}

