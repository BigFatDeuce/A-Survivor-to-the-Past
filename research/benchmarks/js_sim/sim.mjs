// Meme algorithme que sim_core (Rust), en JavaScript avec TypedArrays.
// Represente le cout de simulation d'un moteur JS (Phaser, PixiJS, custom WebGL).
// Usage : node sim.mjs            -> benchmark JS
//         node sim.mjs <fichier.wasm> -> benchmark du module Rust compile en WASM
import { readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";

const CELL = 16, GRID_W = 256, WORLD = CELL * GRID_W, ENEMY_R = 6, PROJ_R = 4;

class Sim {
  constructor(n, m) {
    this.n = n; this.m = m;
    this.ex = new Float32Array(n); this.ey = new Float32Array(n);
    this.evx = new Float32Array(n); this.evy = new Float32Array(n); this.ehp = new Float32Array(n);
    this.px = new Float32Array(m); this.py = new Float32Array(m);
    this.pvx = new Float32Array(m); this.pvy = new Float32Array(m); this.plife = new Float32Array(m);
    this.cellStart = new Uint32Array(GRID_W * GRID_W + 1); this.cellCount = new Uint32Array(GRID_W * GRID_W);
    this.cellItems = new Uint32Array(n); this.ecell = new Uint32Array(n);
    this.plx = WORLD / 2; this.ply = WORLD / 2; this.rng = 0x12345678; this.kills = 0;
    for (let i = 0; i < n; i++) this.respawnEnemy(i);
    for (let j = 0; j < m; j++) this.respawnProj(j);
  }
  rnd() { let s = this.rng; s ^= s << 13; s >>>= 0; s ^= s >>> 17; s ^= s << 5; s >>>= 0; this.rng = s; return s / 4294967295; }
  respawnEnemy(i) {
    const a = this.rnd() * Math.PI * 2, r = 250 + this.rnd() * 900;
    this.ex[i] = Math.min(Math.max(this.plx + Math.cos(a) * r, 1), WORLD - 1);
    this.ey[i] = Math.min(Math.max(this.ply + Math.sin(a) * r, 1), WORLD - 1);
    this.ehp[i] = 3;
  }
  respawnProj(j) {
    const a = this.rnd() * Math.PI * 2;
    this.px[j] = this.plx; this.py[j] = this.ply;
    this.pvx[j] = Math.cos(a) * 300; this.pvy[j] = Math.sin(a) * 300; this.plife[j] = 0.5 + this.rnd() * 2.5;
  }
  buildGrid() {
    const { n, ex, ey, ecell, cellCount, cellStart, cellItems } = this;
    cellCount.fill(0);
    for (let i = 0; i < n; i++) {
      const cx = Math.min((ex[i] / CELL) | 0, GRID_W - 1), cy = Math.min((ey[i] / CELL) | 0, GRID_W - 1);
      const c = cy * GRID_W + cx; ecell[i] = c; cellCount[c]++;
    }
    let acc = 0;
    for (let c = 0; c < GRID_W * GRID_W; c++) { cellStart[c] = acc; acc += cellCount[c]; cellCount[c] = 0; }
    cellStart[GRID_W * GRID_W] = acc;
    for (let i = 0; i < n; i++) { const c = ecell[i]; cellItems[cellStart[c] + cellCount[c]] = i; cellCount[c]++; }
  }
  tick(dt) {
    this.plx += 60 * dt; if (this.plx > WORLD * 0.75) this.plx = WORLD * 0.25;
    this.buildGrid();
    const { n, m, ex, ey, evx, evy, ehp, cellStart, cellItems, px, py, pvx, pvy, plife } = this;
    const plx = this.plx, ply = this.ply, sep = 2 * ENEMY_R;
    for (let i = 0; i < n; i++) {
      const x = ex[i], y = ey[i], dx = plx - x, dy = ply - y;
      const d = Math.max(Math.sqrt(dx * dx + dy * dy), 0.001);
      let ax = dx / d * 40, ay = dy / d * 40;
      const cx = (x / CELL) | 0, cy = (y / CELL) | 0;
      let checked = 0;
      outer: for (let oy = -1; oy <= 1; oy++) for (let ox = -1; ox <= 1; ox++) {
        const nx = cx + ox, ny = cy + oy;
        if (nx < 0 || ny < 0 || nx >= GRID_W || ny >= GRID_W) continue;
        const c = ny * GRID_W + nx;
        for (let k = cellStart[c], e = cellStart[c + 1]; k < e; k++) {
          const j = cellItems[k]; if (j === i) continue;
          const sx = x - ex[j], sy = y - ey[j], d2 = sx * sx + sy * sy;
          if (d2 < sep * sep && d2 > 1e-4) { const dd = Math.sqrt(d2), push = (sep - dd) * 8 / dd; ax += sx * push; ay += sy * push; }
          if (++checked >= 24) break outer;
        }
      }
      evx[i] = ax; evy[i] = ay;
    }
    for (let i = 0; i < n; i++) {
      ex[i] = Math.min(Math.max(ex[i] + evx[i] * dt, 0), WORLD - 0.01);
      ey[i] = Math.min(Math.max(ey[i] + evy[i] * dt, 0), WORLD - 0.01);
    }
    const hr = (ENEMY_R + PROJ_R) ** 2;
    for (let j = 0; j < m; j++) {
      px[j] += pvx[j] * dt; py[j] += pvy[j] * dt; plife[j] -= dt;
      const x = px[j], y = py[j];
      if (plife[j] <= 0 || x < 0 || y < 0 || x >= WORLD || y >= WORLD) { this.respawnProj(j); continue; }
      const cx = (x / CELL) | 0, cy = (y / CELL) | 0;
      let hit = -1;
      o2: for (let oy = -1; oy <= 1; oy++) for (let ox = -1; ox <= 1; ox++) {
        const nx = cx + ox, ny = cy + oy;
        if (nx < 0 || ny < 0 || nx >= GRID_W || ny >= GRID_W) continue;
        const c = ny * GRID_W + nx;
        for (let k = cellStart[c], e = cellStart[c + 1]; k < e; k++) {
          const en = cellItems[k], sx = x - ex[en], sy = y - ey[en];
          if (sx * sx + sy * sy < hr && ehp[en] > 0) { hit = en; break o2; }
        }
      }
      if (hit >= 0) { ehp[hit] -= 1; if (ehp[hit] <= 0) { this.kills++; this.respawnEnemy(hit); } this.respawnProj(j); }
    }
  }
}

async function makeRunner(wasmPath) {
  if (!wasmPath) return (n, m) => { const s = new Sim(n, m); return { tick: dt => s.tick(dt), kills: () => s.kills }; };
  const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
  const e = instance.exports;
  return (n, m) => { e.sim_init(n, m); return { tick: dt => e.sim_tick(dt), kills: () => e.sim_kills() }; };
}

const make = await makeRunner(process.argv[2]);
console.log((process.argv[2] ? "WASM (Rust)" : "JavaScript") + " — ennemis / projs / ms par tick / p99");
for (const n of [1000, 2500, 5000, 10000, 20000, 50000, 100000]) {
  const s = make(n, n / 2);
  for (let i = 0; i < 120; i++) s.tick(1 / 60);
  const v = [];
  for (let i = 0; i < 600; i++) { const t = performance.now(); s.tick(1 / 60); v.push(performance.now() - t); }
  const avg = v.reduce((a, b) => a + b, 0) / v.length; v.sort((a, b) => a - b);
  console.log(String(n).padStart(8), String(n / 2).padStart(8), avg.toFixed(3).padStart(10), v[Math.floor(v.length * 0.99)].toFixed(3).padStart(10), String(s.kills()).padStart(8));
}
