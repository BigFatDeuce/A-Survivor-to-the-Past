//! Operations d'image communes : transparence, recadrage, rognage.
use image::{Rgba, RgbaImage};
use std::collections::HashMap;

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Couleur opaque la plus frequente (fond d'une planche).
pub fn dominant_color(img: &RgbaImage) -> [u8; 3] {
    let mut counts: HashMap<[u8; 3], u32> = HashMap::new();
    for p in img.pixels() {
        if p[3] > 0 {
            *counts.entry([p[0], p[1], p[2]]).or_default() += 1;
        }
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(k, _)| k).unwrap_or([0, 0, 0])
}

/// Rend transparentes les couleurs listees ("auto" = couleur dominante).
pub fn apply_transparency(img: &mut RgbaImage, specs: &[String]) {
    let mut colors: Vec<[u8; 3]> = Vec::new();
    for s in specs {
        if s.eq_ignore_ascii_case("auto") {
            colors.push(dominant_color(img));
        } else if let Some(c) = parse_hex(s) {
            colors.push(c);
        }
    }
    if colors.is_empty() {
        return;
    }
    for p in img.pixels_mut() {
        if colors.iter().any(|c| c[0] == p[0] && c[1] == p[1] && c[2] == p[2]) {
            *p = Rgba([0, 0, 0, 0]);
        }
    }
}

pub fn crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for yy in 0..h {
        for xx in 0..w {
            let (sx, sy) = (x + xx, y + yy);
            if sx < img.width() && sy < img.height() {
                out.put_pixel(xx, yy, *img.get_pixel(sx, sy));
            }
        }
    }
    out
}

/// Colonnes extremes contenant au moins un pixel opaque (None si vide).
pub fn opaque_columns(img: &RgbaImage) -> Option<(u32, u32)> {
    let mut lo = u32::MAX;
    let mut hi = 0;
    for x in 0..img.width() {
        if (0..img.height()).any(|y| img.get_pixel(x, y)[3] > 0) {
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    (lo != u32::MAX).then_some((lo, hi))
}

/// Boite englobante des pixels opaques (x, y, l, h).
pub fn opaque_bounds(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 0 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x0 != u32::MAX).then_some((x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}
