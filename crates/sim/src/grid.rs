//! Grille spatiale uniforme (tri par comptage) centree sur le joueur : reconstruite a chaque pas,
//! elle repond aux requetes de voisinage en temps quasi constant, meme avec des milliers d'ennemis.
use glam::Vec2;

pub const CELL: f32 = 16.0;

pub struct Grid {
    /// Coin haut-gauche de la cellule (0, 0) dans le monde.
    pub origin: Vec2,
    pub w: i32,
    pub h: i32,
    start: Vec<u32>,
    count: Vec<u32>,
    items: Vec<u32>,
    cell_of: Vec<u32>,
}

impl Grid {
    pub fn new(w: i32, h: i32) -> Grid {
        let n = (w * h) as usize;
        Grid {
            origin: Vec2::ZERO,
            w,
            h,
            start: vec![0; n + 1],
            count: vec![0; n],
            items: Vec::new(),
            cell_of: Vec::new(),
        }
    }

    #[inline]
    pub fn coords(&self, p: Vec2) -> (i32, i32) {
        let q = (p - self.origin) / CELL;
        ((q.x as i32).clamp(0, self.w - 1), (q.y as i32).clamp(0, self.h - 1))
    }

    pub fn rebuild(&mut self, center: Vec2, pos: &[Vec2]) {
        let half = Vec2::new(self.w as f32, self.h as f32) * CELL * 0.5;
        self.origin = ((center - half) / CELL).floor() * CELL;
        let n = pos.len();
        self.items.resize(n, 0);
        self.cell_of.resize(n, 0);
        self.count.iter_mut().for_each(|c| *c = 0);
        for (i, p) in pos.iter().enumerate() {
            let (cx, cy) = self.coords(*p);
            let c = (cy * self.w + cx) as u32;
            self.cell_of[i] = c;
            self.count[c as usize] += 1;
        }
        let mut acc = 0u32;
        for c in 0..self.count.len() {
            self.start[c] = acc;
            acc += self.count[c];
            self.count[c] = 0;
        }
        self.start[self.count.len()] = acc;
        for i in 0..n {
            let c = self.cell_of[i] as usize;
            self.items[(self.start[c] + self.count[c]) as usize] = i as u32;
            self.count[c] += 1;
        }
    }

    /// Elements d'une cellule.
    #[inline]
    pub fn cell(&self, cx: i32, cy: i32) -> &[u32] {
        let c = (cy * self.w + cx) as usize;
        &self.items[self.start[c] as usize..self.start[c + 1] as usize]
    }

    /// Appelle `f` pour chaque element des cellules couvrant le disque (p, r).
    #[inline]
    pub fn query(&self, p: Vec2, r: f32, mut f: impl FnMut(usize)) {
        let (x0, y0) = self.coords(p - Vec2::splat(r));
        let (x1, y1) = self.coords(p + Vec2::splat(r));
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                for &i in self.cell(cx, cy) {
                    f(i as usize);
                }
            }
        }
    }
}
