//! Benchmark "horde" independant du moteur : simulation data-oriented (SoA).
//! N ennemis qui poursuivent le joueur + separation via grille spatiale
//! + M projectiles testes contre les ennemis via la meme grille.
#![allow(clippy::needless_range_loop)]

const CELL: f32 = 16.0;
const GRID_W: usize = 256; // 256*16 = 4096 px de monde
const WORLD: f32 = CELL * GRID_W as f32;
const ENEMY_R: f32 = 6.0;
const PROJ_R: f32 = 4.0;

pub struct Sim {
    n: usize,
    ex: Vec<f32>,
    ey: Vec<f32>,
    evx: Vec<f32>,
    evy: Vec<f32>,
    ehp: Vec<f32>,
    m: usize,
    px: Vec<f32>,
    py: Vec<f32>,
    pvx: Vec<f32>,
    pvy: Vec<f32>,
    plife: Vec<f32>,
    cell_start: Vec<u32>,
    cell_count: Vec<u32>,
    cell_items: Vec<u32>,
    ecell: Vec<u32>,
    player_x: f32,
    player_y: f32,
    rng: u32,
    pub kills: u64,
    pub hits: u64,
}

#[inline]
fn rnd(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s as f32) / (u32::MAX as f32)
}

impl Sim {
    pub fn new(n: usize, m: usize) -> Sim {
        let mut s = Sim {
            n,
            ex: vec![0.0; n],
            ey: vec![0.0; n],
            evx: vec![0.0; n],
            evy: vec![0.0; n],
            ehp: vec![0.0; n],
            m,
            px: vec![0.0; m],
            py: vec![0.0; m],
            pvx: vec![0.0; m],
            pvy: vec![0.0; m],
            plife: vec![0.0; m],
            cell_start: vec![0; GRID_W * GRID_W + 1],
            cell_count: vec![0; GRID_W * GRID_W],
            cell_items: vec![0; n],
            ecell: vec![0; n],
            player_x: WORLD * 0.5,
            player_y: WORLD * 0.5,
            rng: 0x1234_5678,
            kills: 0,
            hits: 0,
        };
        for i in 0..n {
            s.respawn_enemy(i);
        }
        for j in 0..m {
            s.respawn_proj(j);
        }
        s
    }

    pub fn positions(&self) -> (&[f32], &[f32]) {
        (&self.ex, &self.ey)
    }

    fn respawn_enemy(&mut self, i: usize) {
        let a = rnd(&mut self.rng) * std::f32::consts::TAU;
        let r = 250.0 + rnd(&mut self.rng) * 900.0;
        self.ex[i] = (self.player_x + a.cos() * r).clamp(1.0, WORLD - 1.0);
        self.ey[i] = (self.player_y + a.sin() * r).clamp(1.0, WORLD - 1.0);
        self.ehp[i] = 3.0;
    }

    fn respawn_proj(&mut self, j: usize) {
        let a = rnd(&mut self.rng) * std::f32::consts::TAU;
        self.px[j] = self.player_x;
        self.py[j] = self.player_y;
        self.pvx[j] = a.cos() * 300.0;
        self.pvy[j] = a.sin() * 300.0;
        self.plife[j] = 0.5 + rnd(&mut self.rng) * 2.5;
    }

    #[inline]
    fn cell_of(x: f32, y: f32) -> usize {
        let cx = ((x / CELL) as usize).min(GRID_W - 1);
        let cy = ((y / CELL) as usize).min(GRID_W - 1);
        cy * GRID_W + cx
    }

    fn build_grid(&mut self) {
        self.cell_count.iter_mut().for_each(|c| *c = 0);
        for i in 0..self.n {
            let c = Self::cell_of(self.ex[i], self.ey[i]);
            self.ecell[i] = c as u32;
            self.cell_count[c] += 1;
        }
        let mut acc = 0u32;
        for c in 0..GRID_W * GRID_W {
            self.cell_start[c] = acc;
            acc += self.cell_count[c];
            self.cell_count[c] = 0;
        }
        self.cell_start[GRID_W * GRID_W] = acc;
        for i in 0..self.n {
            let c = self.ecell[i] as usize;
            let k = self.cell_start[c] + self.cell_count[c];
            self.cell_items[k as usize] = i as u32;
            self.cell_count[c] += 1;
        }
    }

    pub fn tick(&mut self, dt: f32) {
        // le joueur se deplace en continu
        self.player_x += 60.0 * dt;
        if self.player_x > WORLD * 0.75 {
            self.player_x = WORLD * 0.25;
        }
        self.build_grid();

        // 1) poursuite + separation (max 24 voisins testes)
        let (plx, ply) = (self.player_x, self.player_y);
        let sep = 2.0 * ENEMY_R;
        for i in 0..self.n {
            let (x, y) = (self.ex[i], self.ey[i]);
            let (dx, dy) = (plx - x, ply - y);
            let d = (dx * dx + dy * dy).sqrt().max(0.001);
            let (mut ax, mut ay) = (dx / d * 40.0, dy / d * 40.0);
            let cx = (x / CELL) as i32;
            let cy = (y / CELL) as i32;
            let mut checked = 0;
            'outer: for oy in -1..=1 {
                for ox in -1..=1 {
                    let (nx, ny) = (cx + ox, cy + oy);
                    if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_W as i32 {
                        continue;
                    }
                    let c = ny as usize * GRID_W + nx as usize;
                    for k in self.cell_start[c]..self.cell_start[c + 1] {
                        let j = self.cell_items[k as usize] as usize;
                        if j == i {
                            continue;
                        }
                        let (sx, sy) = (x - self.ex[j], y - self.ey[j]);
                        let d2 = sx * sx + sy * sy;
                        if d2 < sep * sep && d2 > 1e-4 {
                            let dd = d2.sqrt();
                            let push = (sep - dd) * 8.0 / dd;
                            ax += sx * push;
                            ay += sy * push;
                        }
                        checked += 1;
                        if checked >= 24 {
                            break 'outer;
                        }
                    }
                }
            }
            self.evx[i] = ax;
            self.evy[i] = ay;
        }
        for i in 0..self.n {
            self.ex[i] = (self.ex[i] + self.evx[i] * dt).clamp(0.0, WORLD - 0.01);
            self.ey[i] = (self.ey[i] + self.evy[i] * dt).clamp(0.0, WORLD - 0.01);
        }

        // 2) projectiles vs ennemis
        let hr = (ENEMY_R + PROJ_R) * (ENEMY_R + PROJ_R);
        for j in 0..self.m {
            self.px[j] += self.pvx[j] * dt;
            self.py[j] += self.pvy[j] * dt;
            self.plife[j] -= dt;
            let (x, y) = (self.px[j], self.py[j]);
            if self.plife[j] <= 0.0 || x < 0.0 || y < 0.0 || x >= WORLD || y >= WORLD {
                self.respawn_proj(j);
                continue;
            }
            let cx = (x / CELL) as i32;
            let cy = (y / CELL) as i32;
            let mut hit = usize::MAX;
            'o2: for oy in -1..=1 {
                for ox in -1..=1 {
                    let (nx, ny) = (cx + ox, cy + oy);
                    if nx < 0 || ny < 0 || nx >= GRID_W as i32 || ny >= GRID_W as i32 {
                        continue;
                    }
                    let c = ny as usize * GRID_W + nx as usize;
                    for k in self.cell_start[c]..self.cell_start[c + 1] {
                        let e = self.cell_items[k as usize] as usize;
                        let (sx, sy) = (x - self.ex[e], y - self.ey[e]);
                        if sx * sx + sy * sy < hr && self.ehp[e] > 0.0 {
                            hit = e;
                            break 'o2;
                        }
                    }
                }
            }
            if hit != usize::MAX {
                self.hits += 1;
                self.ehp[hit] -= 1.0;
                if self.ehp[hit] <= 0.0 {
                    self.kills += 1;
                    self.respawn_enemy(hit);
                }
                self.respawn_proj(j);
            }
        }
    }
}

// ---- Exports WASM minimalistes (sans wasm-bindgen) ----
static mut SIM: Option<Sim> = None;

#[no_mangle]
pub extern "C" fn sim_init(n: u32, m: u32) {
    unsafe {
        SIM = Some(Sim::new(n as usize, m as usize));
    }
}

#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn sim_tick(dt: f32) {
    unsafe {
        if let Some(s) = SIM.as_mut() {
            s.tick(dt);
        }
    }
}

#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn sim_kills() -> f64 {
    unsafe { SIM.as_ref().map(|s| s.kills as f64).unwrap_or(0.0) }
}
