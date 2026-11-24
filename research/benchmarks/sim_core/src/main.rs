use std::time::Instant;

fn main() {
    println!(
        "{:>8} {:>8} {:>10} {:>10} {:>10}",
        "ennemis", "projs", "ms/tick", "p99 ms", "kills"
    );
    for &n in &[1_000usize, 2_500, 5_000, 10_000, 20_000, 50_000, 100_000] {
        let m = n / 2;
        let mut s = sim_core::Sim::new(n, m);
        for _ in 0..120 {
            s.tick(1.0 / 60.0); // chauffe + regroupement de la horde
        }
        let mut v = Vec::with_capacity(600);
        for _ in 0..600 {
            let t = Instant::now();
            s.tick(1.0 / 60.0);
            v.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = v.iter().sum::<f64>() / v.len() as f64;
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "{:>8} {:>8} {:>10.3} {:>10.3} {:>10}",
            n,
            m,
            avg,
            v[(v.len() * 99) / 100],
            s.kills
        );
    }
}
