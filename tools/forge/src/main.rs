//! forge : fabrique les assets du jeu a partir des planches et sons d'origine.
//!
//!   cargo run -p forge -- sprites            -> assets/gen/atlas.png + atlas.ron
//!   cargo run -p forge -- audio <musique> <sfx>  -> assets/audio/{music,sfx}/*.ogg (ffmpeg requis)
//!
//! Les planches d'origine ne sont jamais modifiees.
use assetkit::imageops::{apply_transparency, crop, opaque_bounds, parse_hex};
use assetkit::manifest::{Align, Compose, FontSpec, Manifest};
use assetkit::meta::{Anim, AtlasMeta, Font, Frame, Glyph};
use assetkit::pack::{blit, Packer};
use image::{Rgba, RgbaImage};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

const ALTTP_PREFIX: &str = "SNES - The Legend of Zelda_ A Link to the Past - ";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("audio") => {
            let music = args.get(2).expect("dossier des musiques FLAC");
            let sfx = args.get(3).expect("dossier des effets sonores WAV");
            audio(Path::new(music), Path::new(sfx));
        }
        _ => sprites(),
    }
}

enum Kind {
    Sprite { sprite: String, anim: String, index: usize, pivot: (i32, i32) },
    Glyph { font: String, ch: char, oy: i16 },
    Tile { set: String, index: usize },
}

struct Item {
    kind: Kind,
    img: RgbaImage,
}

struct Sources {
    dir: PathBuf,
    cache: HashMap<(String, Vec<String>), RgbaImage>,
}

impl Sources {
    fn get(&mut self, name: &str, transparent: &[String]) -> &RgbaImage {
        let key = (name.to_string(), transparent.to_vec());
        if !self.cache.contains_key(&key) {
            let mut path = self.dir.join(name);
            if !path.exists() {
                path = self.dir.join(format!("{ALTTP_PREFIX}{name}"));
            }
            let mut img = image::open(&path)
                .unwrap_or_else(|e| panic!("planche introuvable {}: {e}", path.display()))
                .to_rgba8();
            apply_transparency(&mut img, transparent);
            self.cache.insert(key.clone(), img);
        }
        &self.cache[&key]
    }
}

fn sprites() {
    let root = PathBuf::from(".");
    let mut src = Sources { dir: root.join("assets/source"), cache: HashMap::new() };
    let mut items: Vec<Item> = Vec::new();

    // pixel blanc (formes pleines, fondus, barres)
    let white = RgbaImage::from_pixel(4, 4, Rgba([255, 255, 255, 255]));
    items.push(Item { kind: Kind::Sprite { sprite: "white".into(), anim: "0".into(), index: 0, pivot: (0, 0) }, img: white });

    let mut manifests: Vec<PathBuf> = std::fs::read_dir(root.join("data/sprites"))
        .expect("data/sprites")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .collect();
    manifests.sort();

    let mut anim_info: BTreeMap<(String, String), (f32, assetkit::meta::AnimMode, usize)> = BTreeMap::new();
    let mut font_info: BTreeMap<String, (u16, u16, i16)> = BTreeMap::new();

    for mpath in &manifests {
        let text = std::fs::read_to_string(mpath).unwrap();
        let m: Manifest = ron::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", mpath.display()));
        println!("manifeste {}", mpath.display());
        for sheet in &m.sheets {
            let img = src.get(&sheet.source, &sheet.transparent).clone();
            for (sname, anims) in &sheet.sprites {
                for (aname, spec) in anims {
                    let top = spec.frames.iter().map(|f| f.1 as i32).min().unwrap_or(0);
                    let bottom = spec.frames.iter().map(|f| (f.1 + f.3) as i32).max().unwrap_or(0);
                    for (i, &(x, y, w, h)) in spec.frames.iter().enumerate() {
                        let fimg = crop(&img, x, y, w, h);
                        let px = if spec.center_x { w as i32 / 2 + spec.pivot.0 } else { spec.pivot.0 };
                        let py = match spec.align {
                            Align::Top => top + spec.pivot.1 - y as i32,
                            Align::Bottom => bottom + spec.pivot.1 - y as i32,
                            Align::Center => h as i32 / 2 + spec.pivot.1,
                        };
                        items.push(Item {
                            kind: Kind::Sprite { sprite: sname.clone(), anim: aname.clone(), index: i, pivot: (px, py) },
                            img: fimg,
                        });
                    }
                    anim_info.insert((sname.clone(), aname.clone()), (spec.fps, spec.mode, spec.frames.len()));
                }
            }
        }
        for f in &m.fonts {
            build_font(&mut src, f, &mut items);
            font_info.insert(f.name.clone(), (f.line_h, f.space, f.spacing));
        }
        for t in &m.tilesets {
            let img = src.get(&t.source, &[]).clone();
            for (i, &(x, y)) in t.tiles.iter().enumerate() {
                items.push(Item { kind: Kind::Tile { set: t.name.clone(), index: i }, img: crop(&img, x, y, t.size, t.size) });
            }
        }
    }

    // rangement : les plus hauts d'abord
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(items[i].img.height()), std::cmp::Reverse(items[i].img.width()), i));
    let mut size = 512;
    let placements = loop {
        let mut p = Packer::new(size, size, 1);
        let res: Option<Vec<(u32, u32)>> = order.iter().map(|&i| p.alloc(items[i].img.width(), items[i].img.height())).collect();
        if let Some(r) = res {
            break r;
        }
        size *= 2;
        assert!(size <= 8192, "atlas trop grand");
    };
    let mut atlas = RgbaImage::new(size, size);
    let mut meta = AtlasMeta { image: "atlas.png".into(), width: size, height: size, ..Default::default() };
    let mut frames: BTreeMap<(String, String), Vec<(usize, Frame)>> = BTreeMap::new();
    for (k, &i) in order.iter().enumerate() {
        let (x, y) = placements[k];
        let it = &items[i];
        blit(&mut atlas, &it.img, x, y);
        let (w, h) = (it.img.width() as u16, it.img.height() as u16);
        match &it.kind {
            Kind::Sprite { sprite, anim, index, pivot } => {
                frames.entry((sprite.clone(), anim.clone())).or_default().push((
                    *index,
                    Frame { x: x as u16, y: y as u16, w, h, px: pivot.0 as i16, py: pivot.1 as i16 },
                ));
            }
            Kind::Glyph { font, ch, oy } => {
                meta.fonts.entry(font.clone()).or_default().glyphs.insert(*ch, Glyph { x: x as u16, y: y as u16, w, h, oy: *oy });
            }
            Kind::Tile { set, index } => {
                let v = meta.tilesets.entry(set.clone()).or_default();
                if v.len() <= *index {
                    v.resize(*index + 1, Frame::default());
                }
                v[*index] = Frame { x: x as u16, y: y as u16, w, h, px: 0, py: 0 };
            }
        }
    }
    for ((s, a), mut fr) in frames {
        fr.sort_by_key(|f| f.0);
        let (fps, mode, _) = anim_info.get(&(s.clone(), a.clone())).copied().unwrap_or((8.0, Default::default(), 1));
        meta.sprites.entry(s).or_default().insert(a, Anim { frames: fr.into_iter().map(|f| f.1).collect(), fps, mode });
    }
    for (name, (lh, space, spacing)) in font_info {
        let f = meta.fonts.entry(name).or_insert_with(Font::default);
        f.line_height = lh;
        f.space = space;
        f.spacing = spacing;
    }
    let out = root.join("assets/gen");
    std::fs::create_dir_all(&out).unwrap();
    atlas.save(out.join("atlas.png")).unwrap();
    let ron_text = ron::ser::to_string_pretty(&meta, ron::ser::PrettyConfig::default().depth_limit(3)).unwrap();
    std::fs::write(out.join("atlas.ron"), ron_text).unwrap();
    let nsprites: usize = meta.sprites.values().map(|a| a.len()).sum();
    println!(
        "atlas {size}x{size} : {} images, {} animations, {} polices, {} jeux de tuiles",
        items.len(),
        nsprites,
        meta.fonts.len(),
        meta.tilesets.len()
    );
}

/// Construit une police : cases des blocs, puis glyphes composes, puis rognage.
fn build_font(src: &mut Sources, f: &FontSpec, items: &mut Vec<Item>) {
    // case entiere par caractere (hauteur de case conservee pour composer) ;
    // le troisieme champ compte les rangees ajoutees au-dessus de la case
    let mut cells: BTreeMap<char, (RgbaImage, bool, u32)> = BTreeMap::new();
    for b in &f.blocks {
        let img = src.get(&b.source, &b.transparent).clone();
        for (r, row) in b.rows.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                if ch == '¤' {
                    continue;
                }
                let (x, y) = (b.origin.0 + c as u32 * b.cell.0, b.origin.1 + r as u32 * b.cell.1);
                cells.insert(ch, (crop(&img, x, y, b.cell.0, b.cell.1), b.mono, 0));
            }
        }
    }
    let ink = |s: &str| {
        let c = parse_hex(s).unwrap_or([0, 0, 0]);
        Rgba([c[0], c[1], c[2], 255])
    };
    let (outline, fill) = (ink(&f.ink.0), ink(&f.ink.1));
    let ar = f.accent_rows;
    for op in &f.compose {
        let made = match op {
            Compose::Accent(res, base, donor) => match (cells.get(base), cells.get(donor)) {
                (Some(b), Some(d)) => Some((*res, accent_from(&b.0, &d.0, ar, outline), 0)),
                _ => None,
            },
            Compose::AccentCap(res, base, donor) => match (cells.get(base), cells.get(donor)) {
                (Some(b), Some(d)) => accent_cap(&b.0, &d.0, ar, outline).map(|(img, lift)| (*res, img, lift)),
                _ => None,
            },
            Compose::AccentArt(res, base, mask) => cells.get(base).map(|b| {
                let mut out = b.0.clone();
                clear_rows(&mut out, 0, ar);
                let (b0, b1) = body_columns(&out, ar);
                let mw = mask.iter().map(|l| l.chars().count()).max().unwrap_or(0) as i32;
                let x0 = (b0 + b1 + 1 - mw) / 2;
                let y0 = ar as i32 - mask.len() as i32;
                stamp(&mut out, mask, x0, y0, outline, fill);
                (*res, out, 0)
            }),
            Compose::Rotate(res, from) => cells.get(from).map(|b| (*res, image::imageops::rotate180(&b.0), b.2)),
            Compose::Art(res, row, mask) => {
                let h = f.blocks.first().map(|b| b.cell.1).unwrap_or(16);
                let mw = mask.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u32;
                let mut out = RgbaImage::new(mw + 2, h);
                stamp(&mut out, mask, 1, *row as i32, outline, fill);
                Some((*res, out, 0))
            }
        };
        match made {
            Some((ch, img, lift)) => {
                cells.insert(ch, (img, false, lift));
            }
            None => println!("  police {} : glyphe compose impossible {op:?}", f.name),
        }
    }
    for (ch, (cell, mono, lift)) in cells {
        let Some((bx, by, bw, bh)) = opaque_bounds(&cell) else {
            continue;
        };
        let (x0, w) = if mono { (0, cell.width()) } else { (bx, bw) };
        let img = crop(&cell, x0, by, w, bh);
        let oy = by as i16 - f.top as i16 - lift as i16;
        items.push(Item { kind: Kind::Glyph { font: f.name.clone(), ch, oy }, img });
    }
}

fn clear_rows(img: &mut RgbaImage, y0: u32, y1: u32) {
    for y in y0..y1.min(img.height()) {
        for x in 0..img.width() {
            img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        }
    }
}

/// Colonnes extremes du corps de la lettre (rangees >= `from`).
fn body_columns(img: &RgbaImage, from: u32) -> (i32, i32) {
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    for y in from..img.height() {
        for x in 0..img.width() {
            if img.get_pixel(x, y)[3] > 0 {
                lo = lo.min(x as i32);
                hi = hi.max(x as i32);
            }
        }
    }
    if lo > hi {
        (0, img.width() as i32 - 1)
    } else {
        (lo, hi)
    }
}

/// Corps de `base` + accent du donneur (rangees < `ar`), centre sur le corps ;
/// le contour sous l'accent est ferme si le corps est plus etroit.
fn accent_from(base: &RgbaImage, donor: &RgbaImage, ar: u32, outline: Rgba<u8>) -> RgbaImage {
    let mut out = base.clone();
    clear_rows(&mut out, 0, ar);
    let (b0, b1) = body_columns(base, ar);
    let (d0, d1) = body_columns(donor, ar);
    let shift = ((b0 + b1) - (d0 + d1)).div_euclid(2);
    for y in 0..ar.min(donor.height()) {
        for x in 0..donor.width() as i32 {
            let p = *donor.get_pixel(x as u32, y);
            let tx = x + shift;
            if p[3] > 0 && tx >= 0 && (tx as u32) < out.width() {
                out.put_pixel(tx as u32, y, p);
            }
        }
    }
    close_under(&mut out, ar, outline);
    out
}

/// Majuscule accentuee : l'accent du donneur (rangees opaques au-dessus de `ar`) est pose juste
/// au-dessus du contour haut de la majuscule, qui le ferme. Renvoie la case (agrandie vers le
/// haut si l'accent depasse) et le nombre de rangees ajoutees.
fn accent_cap(base: &RgbaImage, donor: &RgbaImage, ar: u32, outline: Rgba<u8>) -> Option<(RgbaImage, u32)> {
    let opaque_row = |img: &RgbaImage, y: u32| (0..img.width()).any(|x| img.get_pixel(x, y)[3] > 0);
    let top = (0..base.height()).find(|&y| opaque_row(base, y))?;
    let rows: Vec<u32> = (0..ar.min(donor.height())).filter(|&y| opaque_row(donor, y)).collect();
    let (a0, a1) = (*rows.first()?, *rows.last()?);
    let h = a1 - a0 + 1;
    let lift = h.saturating_sub(top);
    let mut out = RgbaImage::new(base.width(), base.height() + lift);
    image::imageops::replace(&mut out, base, 0, lift as i64);
    let (b0, b1) = body_columns(base, top);
    let (d0, d1) = body_columns(donor, ar);
    let shift = ((b0 + b1) - (d0 + d1)).div_euclid(2);
    let y_to = top + lift - h;
    for y in a0..=a1 {
        for x in 0..donor.width() as i32 {
            let p = *donor.get_pixel(x as u32, y);
            let tx = x + shift;
            if p[3] > 0 && tx >= 0 && (tx as u32) < out.width() {
                out.put_pixel(tx as u32, y_to + (y - a0), p);
            }
        }
    }
    close_under(&mut out, top + lift, outline);
    Some((out, lift))
}

/// Ferme par un contour la rangee `ar` sous les pixels de la rangee `ar - 1`.
fn close_under(img: &mut RgbaImage, ar: u32, outline: Rgba<u8>) {
    if ar == 0 || ar >= img.height() {
        return;
    }
    for x in 0..img.width() {
        if img.get_pixel(x, ar - 1)[3] > 0 && img.get_pixel(x, ar)[3] == 0 {
            img.put_pixel(x, ar, outline);
        }
    }
}

/// Dessine un masque ('o' = remplissage) entoure d'un contour (voisinage 8).
fn stamp(img: &mut RgbaImage, mask: &[String], x0: i32, y0: i32, outline: Rgba<u8>, fill: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let mut fills = Vec::new();
    for (j, line) in mask.iter().enumerate() {
        for (i, c) in line.chars().enumerate() {
            if c == 'o' {
                fills.push((x0 + i as i32, y0 + j as i32));
            }
        }
    }
    for &(x, y) in &fills {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h && img.get_pixel(nx as u32, ny as u32)[3] == 0 {
                    img.put_pixel(nx as u32, ny as u32, outline);
                }
            }
        }
    }
    for &(x, y) in &fills {
        if x >= 0 && y >= 0 && x < w && y < h {
            img.put_pixel(x as u32, y as u32, fill);
        }
    }
}

/// Conversion audio : musiques FLAC -> OGG (stereo, 32 kHz d'origine),
/// effets WAV -> OGG mono, silences de debut/fin retires.
fn audio(music_dir: &Path, sfx_dir: &Path) {
    let out_m = PathBuf::from("assets/audio/music");
    let out_s = PathBuf::from("assets/audio/sfx");
    std::fs::create_dir_all(&out_m).unwrap();
    std::fs::create_dir_all(&out_s).unwrap();
    for (dir, out, is_music) in [(music_dir, &out_m, true), (sfx_dir, &out_s, false)] {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "flac" || x == "wav"))
            .collect();
        files.sort();
        for f in files {
            let stem = slug(&f.file_stem().unwrap().to_string_lossy());
            let dst = out.join(format!("{stem}.ogg"));
            let mut cmd = Command::new("ffmpeg");
            cmd.args(["-v", "error", "-y", "-i"]).arg(&f);
            if is_music {
                cmd.args(["-c:a", "libvorbis", "-q:a", "6"]);
            } else {
                cmd.args([
                    "-af",
                    "silenceremove=start_periods=1:start_threshold=-55dB,areverse,silenceremove=start_periods=1:start_threshold=-55dB,areverse",
                    "-ac",
                    "1",
                    "-c:a",
                    "libvorbis",
                    "-q:a",
                    "5",
                ]);
            }
            let st = cmd.arg(&dst).status().expect("ffmpeg introuvable");
            println!("{} {}", if st.success() { "ok " } else { "ERR" }, dst.display());
        }
    }
}

/// "10. Hyrule Field Main Theme" -> "10_hyrule_field_main_theme"
fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}
