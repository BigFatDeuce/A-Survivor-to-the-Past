//! Rangement d'images dans un atlas (algorithme "en etageres", trie par hauteur).
use image::RgbaImage;

pub struct Packer {
    pub width: u32,
    pub height: u32,
    pad: u32,
    x: u32,
    y: u32,
    shelf_h: u32,
}

impl Packer {
    pub fn new(width: u32, height: u32, pad: u32) -> Self {
        Packer { width, height, pad, x: pad, y: pad, shelf_h: 0 }
    }

    /// Reserve un emplacement w x h ; None si l'atlas est plein.
    pub fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.x + w + self.pad > self.width {
            self.x = self.pad;
            self.y += self.shelf_h + self.pad;
            self.shelf_h = 0;
        }
        if self.y + h + self.pad > self.height || w + 2 * self.pad > self.width {
            return None;
        }
        let pos = (self.x, self.y);
        self.x += w + self.pad;
        self.shelf_h = self.shelf_h.max(h);
        Some(pos)
    }

    /// Hauteur reellement utilisee.
    pub fn used_height(&self) -> u32 {
        self.y + self.shelf_h + self.pad
    }
}

pub fn blit(dst: &mut RgbaImage, src: &RgbaImage, x: u32, y: u32) {
    for (sx, sy, p) in src.enumerate_pixels() {
        dst.put_pixel(x + sx, y + sy, *p);
    }
}
