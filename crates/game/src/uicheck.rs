//! Verification automatique des placements de l'interface (`--uicheck`).
//! Chaque ecran est dessine dans toutes les langues et plusieurs formats de vue ; les textes et
//! cadres releves par le moteur sont controles : hors ecran, chevauchements, debordements de cadre,
//! caracteres absents de la police.
use assetkit::meta::Font;
use engine::gfx::Probe;
use engine::Vec2;
use std::collections::{BTreeMap, BTreeSet};

/// Vues controlees (pixels logiques) : 16:9, 4:3, fenetre 1024x768, 16:10, 1366x768, 21:9.
pub const VIEWS: [(f32, f32); 6] = [(640.0, 360.0), (480.0, 360.0), (512.0, 384.0), (640.0, 400.0), (683.0, 384.0), (853.0, 360.0)];

#[derive(Default)]
pub struct Report {
    /// (ecran, langue, nature, detail) -> vues concernees
    issues: BTreeMap<(String, String, &'static str, String), BTreeSet<String>>,
    missing: BTreeMap<String, BTreeSet<char>>,
    approx: BTreeMap<String, BTreeSet<char>>,
    pub screens: usize,
}

fn overlap(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    // rectangles reduits d'un pixel : deux lignes qui se touchent ne comptent pas
    let (a0, a1) = (a.0 + Vec2::ONE, a.0 + a.1 - Vec2::ONE);
    let (b0, b1) = (b.0 + Vec2::ONE, b.0 + b.1 - Vec2::ONE);
    a0.x < b1.x && b0.x < a1.x && a0.y < b1.y && b0.y < a1.y
}

fn short(s: &str) -> String {
    let t: String = s.chars().take(28).collect();
    if s.chars().count() > 28 {
        format!("« {t}… »")
    } else {
        format!("« {t} »")
    }
}

impl Report {
    pub fn analyze(&mut self, probes: &[Probe], view: Vec2, font: &Font, screen: &str, lang: &str) {
        self.screens += 1;
        let vname = format!("{}x{}", view.x, view.y);
        let mut add = |kind: &'static str, detail: String| {
            self.issues.entry((screen.to_string(), lang.to_string(), kind, detail)).or_default().insert(vname.clone());
        };
        // (position, taille, texte, calque)
        let mut layer = 0;
        let mut texts: Vec<(Vec2, Vec2, &str, u32)> = Vec::new();
        for p in probes {
            match p {
                Probe::Layer => layer += 1,
                Probe::Text { pos, size, text } if !text.trim().is_empty() => texts.push((*pos, *size, text.as_str(), layer)),
                _ => {}
            }
        }
        let mut layer = 0;
        let mut panels: Vec<(Vec2, Vec2, &str, u32)> = Vec::new();
        for p in probes {
            match p {
                Probe::Layer => layer += 1,
                Probe::Panel { pos, size, name } => panels.push((*pos, *size, *name, layer)),
                _ => {}
            }
        }
        for &(pos, size, t, tl) in &texts {
            if pos.x < -0.5 || pos.y < -0.5 || pos.x + size.x > view.x + 0.5 || pos.y + size.y > view.y + 0.5 {
                add("hors ecran", short(t));
            }
            // le plus petit cadre qui contient le debut du texte doit le contenir en entier
            let start = pos + Vec2::new(1.0, 2.0);
            let host = panels
                .iter()
                .filter(|(p, s, _, pl)| *pl == tl && start.x >= p.x && start.y >= p.y && start.x < p.x + s.x && start.y < p.y + s.y)
                .min_by(|a, b| (a.1.x * a.1.y).total_cmp(&(b.1.x * b.1.y)));
            if let Some(&(p, s, name, _)) = host {
                let end = pos + size - Vec2::new(0.0, 3.0);
                if end.x > p.x + s.x + 0.5 || end.y > p.y + s.y + 0.5 {
                    add("deborde du cadre", format!("{} ({name})", short(t)));
                }
            }
            for c in t.chars() {
                if c != ' ' && !t.starts_with('<') && !font.glyphs.contains_key(&c) {
                    match font.glyph(c) {
                        Some(_) => self.approx.entry(lang.to_string()).or_default().insert(c),
                        None => self.missing.entry(lang.to_string()).or_default().insert(c),
                    };
                }
            }
        }
        for i in 0..texts.len() {
            for j in i + 1..texts.len() {
                let (a, b) = (texts[i], texts[j]);
                if (a.2 == b.2 && a.0 == b.0) || a.3 != b.3 {
                    continue;
                }
                if overlap((a.0, a.1), (b.0, b.1)) {
                    add("chevauchement", format!("{} / {}", short(a.2), short(b.2)));
                }
            }
        }
        for p in probes {
            if let Probe::MissingGlyph { ch, .. } = p {
                self.missing.entry(lang.to_string()).or_default().insert(*ch);
            }
        }
    }

    pub fn print(&self) -> String {
        let mut out = format!("VERIFICATION UI : {} ecrans controles, {} probleme(s)\n", self.screens, self.issues.len());
        for ((screen, lang, kind, detail), views) in &self.issues {
            let v: Vec<&str> = views.iter().map(|s| s.as_str()).collect();
            out += &format!("  [{screen} | {lang}] {kind} : {detail}  ({})\n", v.join(", "));
        }
        for (lang, cs) in &self.missing {
            out += &format!("  caracteres absents ({lang}) : {}\n", cs.iter().collect::<String>());
        }
        for (lang, cs) in &self.approx {
            out += &format!("  caracteres remplaces par leur lettre de base ({lang}) : {}\n", cs.iter().collect::<String>());
        }
        out
    }
}
