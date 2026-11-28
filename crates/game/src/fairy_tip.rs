//! Conseils de la fee (0.7.1) : apres les deux premieres defaites, une fee vient expliquer sur
//! l'ecran titre que les rubis servent a la Boutique. Boite de dialogue a la maniere d'A Link to
//! the Past : texte qui s'ecrit lettre par lettre, fleche quand on peut continuer, question
//! « vraiment passer ? » avant de la faire taire.
use crate::hud::{frame_rect, text, wrap, GOLD, HL, HL_END};
use crate::lang::Lang;
use crate::menus::NavIn;
use engine::app::Ctx;
use engine::draw::{rgb, with_alpha, Align, Params, WHITE};
use engine::gfx::{Blend, DrawList, Space};
use engine::Vec2;

/// Discours de la fee : Boutique (apres deux defaites), bases du jeu (premiere partie, en partie),
/// felicitations (Ganon vaincu une premiere fois).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Talk {
    Shop,
    Intro,
    Victory,
}

impl Talk {
    /// Pages : textes « fairy.<prefixe>.1 » a « .N » (`*mot*` : mis en valeur ; `{x}` : `Fill`).
    pub fn pages(self) -> usize {
        match self {
            Talk::Shop | Talk::Intro => 5,
            Talk::Victory => 4,
        }
    }
    fn prefix(self) -> &'static str {
        match self {
            Talk::Shop => "tip",
            Talk::Intro => "intro",
            Talk::Victory => "win",
        }
    }
}

/// Remplacements `{nom}` des textes (touches de l'appareil utilise) ; `touch` : variante tactile des pages.
#[derive(Default)]
pub struct Fill {
    pub touch: bool,
    pub keys: Vec<(&'static str, String)>,
}
/// Arrivee de la fee, puis ouverture de la boite : le texte commence ensuite.
const ARRIVE: f32 = 0.9;
/// Lettres par seconde.
const CPS: f32 = 42.0;
/// Attente apres la derniere lettre avant qu'un appui passe a la page suivante.
pub const SETTLE: f32 = 0.3;
const LINE_H: f32 = 16.0;
const MIN_LINES: usize = 3;

pub fn due(p: &crate::profile::Profile) -> bool {
    !p.fairy_tip && p.totals.deaths >= 2
}

/// Texte d'une page, `*mot*` remplace par la mise en valeur du jeu.
fn page_text(lang: &Lang, talk: Talk, fill: &Fill, page: usize) -> String {
    let key = format!("fairy.{}.{}", talk.prefix(), page + 1);
    let touch = format!("{key}_touch");
    let mut s = lang.get(if fill.touch && lang.has(&touch) { &touch } else { &key }).to_string();
    for (k, v) in &fill.keys {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s.split('*').enumerate().map(|(k, s)| if k % 2 == 1 { format!("{HL}{s}{HL_END}") } else { s.to_string() }).collect()
}

fn start(page: usize) -> f32 {
    if page == 0 {
        ARRIVE
    } else {
        0.0
    }
}

/// Lettres visibles a l'instant `t` de la page (les codes de couleur ne comptent pas).
fn shown(t: f32, page: usize) -> usize {
    ((t - start(page)).max(0.0) * CPS) as usize
}

fn letters(s: &str) -> usize {
    s.chars().filter(|c| !c.is_control()).count()
}

/// Debut des lignes deja ecrites : `n` lettres en tout, les codes de couleur suivent.
fn reveal(lines: &[String], mut n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for l in lines {
        let mut s = String::new();
        for c in l.chars() {
            if !c.is_control() {
                if n == 0 {
                    break;
                }
                n -= 1;
            }
            s.push(c);
        }
        out.push(s);
    }
    out
}

/// Boite en bas de l'ecran, lignes de la page (coupees une fois pour toutes : les mots ne sautent
/// pas d'une ligne a l'autre pendant qu'ils s'ecrivent).
struct Layout {
    pos: Vec2,
    size: Vec2,
    lines: Vec<String>,
}

fn layout(ctx: &Ctx, lang: &Lang, talk: Talk, fill: &Fill, page: usize) -> Layout {
    let view = ctx.view;
    let w = (view.x - 32.0).min(440.0).round();
    let lines = wrap(ctx.atlas.font("main"), &page_text(lang, talk, fill, page), w - 28.0, 1.0);
    let h = 18.0 + LINE_H * lines.len().max(MIN_LINES) as f32;
    Layout { pos: Vec2::new(((view.x - w) * 0.5).round(), (view.y - h - 30.0).round()), size: Vec2::new(w, h), lines }
}

/// Zones des deux reponses de la question (souris).
fn answer_rects(l: &Layout) -> [(Vec2, Vec2); 2] {
    let r = |k: f32| (Vec2::new(l.pos.x + 14.0, l.pos.y + 7.0 + LINE_H * (1.0 + k)), Vec2::new(l.size.x - 28.0, LINE_H));
    [r(0.0), r(1.0)]
}

/// Ce que la fee demande au jeu apres une image.
pub enum Step {
    Stay { page: usize, t: f32, ask: Option<usize> },
    /// Fin du discours (lu ou passe) : retour au titre, curseur sur la Boutique.
    Done,
    /// Son a jouer, puis rester.
    Sfx(&'static str, usize, f32, Option<usize>),
}

#[allow(clippy::too_many_arguments)]
pub fn update(ctx: &Ctx, lang: &Lang, talk: Talk, fill: &Fill, nav: &NavIn, dt: f32, page: usize, t: f32, ask: Option<usize>) -> Step {
    let l = layout(ctx, lang, talk, fill, page);
    let full_at = start(page) + letters(&l.lines.concat()) as f32 / CPS;
    match ask {
        Some(mut k) => {
            let mut sfx = None;
            if let Some(m) = nav.hover {
                for (i, (p, s)) in answer_rects(&l).iter().enumerate() {
                    if m.x >= p.x && m.y >= p.y && m.x < p.x + s.x && m.y < p.y + s.y && k != i {
                        k = i;
                        sfx = Some("cursor");
                    }
                }
            }
            if nav.up || nav.down {
                k = 1 - k;
                sfx = Some("cursor");
            }
            if nav.back {
                return Step::Sfx("dialog_close", page, t, None);
            }
            if nav.confirm || nav.click {
                return if k == 1 { Step::Done } else { Step::Sfx("dialog_close", page, t, None) };
            }
            // texte fige pendant la question
            match sfx {
                Some(s) => Step::Sfx(s, page, t, Some(k)),
                None => Step::Stay { page, t, ask: Some(k) },
            }
        }
        None => {
            let t2 = t + dt;
            let ready = t2 >= start(page);
            if nav.back && ready {
                return Step::Sfx("dialog_open", page, t, Some(0));
            }
            if (nav.confirm || nav.click) && ready {
                if t2 < full_at {
                    // appui pendant l'ecriture : toute la page d'un coup
                    return Step::Stay { page, t: full_at, ask: None };
                }
                if t2 >= full_at + SETTLE {
                    return if page + 1 < talk.pages() { Step::Sfx("confirm", page + 1, 0.0, None) } else { Step::Done };
                }
            }
            // une lettre sur deux fait le petit bruit du texte d'origine
            let (a, b) = (shown(t, page), shown(t2, page));
            if b != a && t2 < full_at && b / 2 != a / 2 {
                return Step::Sfx("text", page, t2, None);
            }
            Step::Stay { page, t: t2, ask: None }
        }
    }
}

/// Fee posee au-dessus de la boite (arrivee en vol depuis le coin), boite, texte, fleche.
#[allow(clippy::too_many_arguments)]
pub fn draw(list: &mut DrawList, ctx: &Ctx, lang: &Lang, talk: Talk, fill: &Fill, white: &assetkit::meta::Frame, page: usize, t: f32, ask: Option<usize>) {
    let view = ctx.view;
    let atlas = &ctx.atlas;
    let main = atlas.font("main");
    let rt = ctx.real_time as f32;
    let l = layout(ctx, lang, talk, fill, page);
    list.set(Space::Ui, Blend::Alpha);
    let fade = if page == 0 { (t / 0.3).min(1.0) } else { 1.0 };
    list.rect(white, Vec2::ZERO, view, [0.0, 0.0, 0.05, 0.5 * fade]);
    // fee : vol en courbe depuis le coin haut droit, puis elle flotte
    let rest = Vec2::new(l.pos.x + 30.0, l.pos.y - 26.0);
    let k = if page == 0 { (t / 0.8).min(1.0) } else { 1.0 };
    let e = 1.0 - (1.0 - k).powi(3);
    let from = Vec2::new(view.x + 20.0, -20.0);
    let arc = Vec2::new(0.0, -60.0 * (e * std::f32::consts::PI).sin());
    let bob = Vec2::new((rt * 1.3).sin() * 3.0, (rt * 2.6).sin() * 3.0);
    let at = (from.lerp(rest, e) + arc + bob * k).round();
    list.set(Space::Ui, Blend::Add);
    for s in 0..6 {
        let ph = (rt * 0.9 + s as f32 / 6.0).fract();
        let p = at + Vec2::new(((s as f32 * 2.3 + rt).sin() * 10.0).round(), (ph * 18.0).round());
        let a = (1.0 - ph) * 0.9;
        list.rect(white, p, Vec2::splat(2.0), with_alpha(rgb(255, 210, 250), a));
    }
    list.set(Space::Ui, Blend::Alpha);
    if let Some(anim) = atlas.find("fairy", "0") {
        let f = atlas.frame(anim, rt);
        list.frame(f, at + Vec2::new(0.0, 20.0), &Params::default().scale(Vec2::new(1.5, 0.5)).color([0.0, 0.0, 0.0, 0.3 * k]));
        list.frame(f, at, &Params::default().scale(Vec2::splat(2.0)));
    }
    // boite : s'ouvre verticalement, bordure claire doublee de bleu comme dans le jeu d'origine
    let open = if page == 0 { ((t - 0.55) / 0.3).clamp(0.0, 1.0) } else { 1.0 };
    if open <= 0.0 {
        return;
    }
    let h = (l.size.y * open).round().max(4.0);
    let pos = Vec2::new(l.pos.x, (l.pos.y + (l.size.y - h) * 0.5).round());
    let size = Vec2::new(l.size.x, h);
    list.rect(white, pos, size, [0.02, 0.02, 0.1, 0.94]);
    frame_rect(list, white, pos, size, WHITE);
    frame_rect(list, white, pos + Vec2::splat(2.0), size - Vec2::splat(4.0), rgb(110, 130, 220));
    if open < 1.0 {
        return;
    }
    let tx = l.pos.x + 14.0;
    let ty = l.pos.y + 9.0;
    if let Some(sel) = ask {
        text(list, main, lang.get("fairy.skip_ask"), Vec2::new(tx, ty), 1.0, WHITE, Align::Left);
        for (i, (p, _)) in answer_rects(&l).iter().enumerate() {
            let key = if i == 0 { "fairy.skip_no" } else { "fairy.skip_yes" };
            let c = if i == sel { GOLD } else { [0.7, 0.7, 0.8, 1.0] };
            if i == sel {
                arrow_right(list, white, Vec2::new(p.x + 2.0, p.y + 4.0), c);
            }
            text(list, main, lang.get(key), Vec2::new(p.x + 14.0, crate::hud::mid_y(main, p.y, LINE_H, 1.0)), 1.0, c, Align::Left);
        }
    } else {
        let n = shown(t, page);
        for (i, s) in reveal(&l.lines, n).iter().enumerate() {
            text(list, main, s, Vec2::new(tx, ty + LINE_H * i as f32), 1.0, WHITE, Align::Left);
        }
        let full_at = start(page) + letters(&l.lines.concat()) as f32 / CPS;
        // fleche qui clignote : on peut continuer
        if t >= full_at + SETTLE && (rt * 2.5).fract() < 0.65 {
            let p = l.pos + l.size - Vec2::new(16.0, 9.0);
            for (r, w) in [5.0f32, 3.0, 1.0].iter().enumerate() {
                list.rect(white, Vec2::new(p.x + (5.0 - w) * 0.5, p.y + r as f32), Vec2::new(*w, 1.0), GOLD);
            }
        }
    }
    if ask.is_some() {
        return;
    }
    let hint = format!("{}  {}/{}", lang.get("fairy.hint"), page + 1, talk.pages());
    text(list, main, &hint, Vec2::new((view.x * 0.5).round(), l.pos.y + l.size.y + 8.0), 1.0, [0.6, 0.6, 0.72, 1.0], Align::Center);
}

/// Question a deux reponses dans la boite de dialogue d'A Link to the Past (bordure claire doublee de
/// bleu, fleche devant la reponse choisie) : achats confirmes ailleurs que dans la boutique.
pub struct Question {
    pub pos: Vec2,
    pub size: Vec2,
    pub lines: Vec<String>,
}

pub fn question(ctx: &Ctx, text_: &str) -> Question {
    let view = ctx.view;
    let w = (view.x - 32.0).min(440.0).round();
    let lines = wrap(ctx.atlas.font("main"), text_, w - 28.0, 1.0);
    let h = 18.0 + LINE_H * (lines.len() + 2) as f32;
    Question { pos: Vec2::new(((view.x - w) * 0.5).round(), (view.y - h - 30.0).round()), size: Vec2::new(w, h), lines }
}

impl Question {
    fn answer(&self, k: usize) -> (Vec2, Vec2) {
        (Vec2::new(self.pos.x + 14.0, self.pos.y + 7.0 + LINE_H * (self.lines.len() + k) as f32), Vec2::new(self.size.x - 28.0, LINE_H))
    }

    /// Reponse choisie (`sel`, 0 ou 1) deplacee ; Some(k) quand elle est validee, Some(0) pour Retour.
    pub fn input(&self, nav: &NavIn, sel: &mut usize) -> (Option<usize>, bool) {
        let mut moved = false;
        if let Some(m) = nav.hover {
            for k in 0..2 {
                let (p, s) = self.answer(k);
                if m.x >= p.x && m.y >= p.y && m.x < p.x + s.x && m.y < p.y + s.y && *sel != k {
                    *sel = k;
                    moved = true;
                }
            }
        }
        if nav.up || nav.down || nav.left || nav.right {
            *sel = 1 - *sel;
            moved = true;
        }
        if nav.back {
            return (Some(0), moved);
        }
        ((nav.confirm || nav.click).then_some(*sel), moved)
    }

    pub fn draw(&self, list: &mut DrawList, ctx: &Ctx, white: &assetkit::meta::Frame, answers: [&str; 2], sel: usize) {
        let main = ctx.atlas.font("main");
        list.set(Space::Ui, Blend::Alpha);
        list.probe_layer();
        list.rect(white, Vec2::ZERO, ctx.view, [0.0, 0.0, 0.05, 0.5]);
        list.rect(white, self.pos, self.size, [0.02, 0.02, 0.1, 0.94]);
        frame_rect(list, white, self.pos, self.size, WHITE);
        frame_rect(list, white, self.pos + Vec2::splat(2.0), self.size - Vec2::splat(4.0), rgb(110, 130, 220));
        for (i, l) in self.lines.iter().enumerate() {
            text(list, main, l, Vec2::new(self.pos.x + 14.0, self.pos.y + 9.0 + LINE_H * i as f32), 1.0, WHITE, Align::Left);
        }
        for (k, a) in answers.iter().enumerate() {
            let (p, _) = self.answer(k);
            let c = if k == sel { GOLD } else { [0.7, 0.7, 0.8, 1.0] };
            if k == sel {
                arrow_right(list, white, Vec2::new(p.x + 2.0, p.y + 4.0), c);
            }
            text(list, main, a, Vec2::new(p.x + 14.0, crate::hud::mid_y(main, p.y, LINE_H, 1.0)), 1.0, c, Align::Left);
        }
    }
}

fn arrow_right(list: &mut DrawList, white: &assetkit::meta::Frame, p: Vec2, c: engine::draw::Color) {
    for (k, hh) in [7.0f32, 5.0, 3.0, 1.0].iter().enumerate() {
        list.rect(white, Vec2::new(p.x + k as f32, p.y + (7.0 - hh) * 0.5), Vec2::new(1.0, *hh), c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_counts_letters_not_color_codes() {
        let lines = vec![format!("ab{HL}cd{HL_END}"), "ef".to_string()];
        assert_eq!(letters(&lines.concat()), 6);
        assert_eq!(reveal(&lines, 3), vec![format!("ab{HL}c"), String::new()]);
        assert_eq!(reveal(&lines, 5), vec![format!("ab{HL}cd{HL_END}"), "e".to_string()]);
        assert_eq!(shown(start(1) + 1.0, 1), CPS as usize);
    }
}
