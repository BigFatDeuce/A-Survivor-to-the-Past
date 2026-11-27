//! PNJ de passage (0.6.3) : menus de la diseuse de bonne aventure et de la sorciere, dessin des PNJ
//! dans le monde (bulle « ! »), fleches hors champ et animation de la potion bue.
use super::*;
use crate::fortune::{self, Phase};
use crate::witch;
use sim::npc::NpcKind;

/// Couleur de la sorciere (annonces, fleche).
pub(super) const WITCH: Color = rgb(170, 240, 120);
/// Couleur de chaque potion (rouge, verte, bleue).
const POTION_COLORS: [Color; 3] = [rgb(255, 96, 96), rgb(120, 240, 120), rgb(110, 170, 255)];
/// Duree de l'animation de la potion bue.
const DRINK_TIME: f32 = 1.4;

impl Game {
    /// Rubis que la partie mettra en banque si elle est gagnee (quitte ou double compris).
    pub(super) fn run_bank_if_won(&self) -> u64 {
        (bank_amount(self.world.player.rupees, true, self.meta.bank_mult(&self.profile)) as f32 * self.run_bank_mult).round() as u64
    }

    /// Diseuse de bonne aventure : `None` quand la partie reprend (l'etat est deja pose).
    pub(super) fn update_fortune(&mut self, ctx: &mut Ctx, nav: &NavIn, phase: Phase, mut sel: usize, t: f32, dt: f32) -> Option<State> {
        let view = ctx.view;
        let demo = self.opts.demo && t > 1.2;
        let ready = t >= menus::LEVELUP_LOCK;
        Some(match phase {
            Phase::Menu => {
                let (mut pick, moved) = menus::input_levelup(nav, view, 3, &mut sel, t);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if (nav.back && ready) || demo {
                    pick = Some(2);
                }
                match pick {
                    Some(k) if k < 2 && self.fortune_played[k] => {
                        self.sfx(ctx, "error");
                        State::Fortune { phase, sel, t }
                    }
                    Some(0) => {
                        self.sfx(ctx, "confirm");
                        State::Fortune { phase: Phase::Stake, sel: 0, t: 0.0 }
                    }
                    Some(1) => {
                        self.fortune_played[1] = true;
                        let win = self.rng.chance(fortune::DOUBLE_WIN);
                        self.run_bank_mult = if win { 2.0 } else { 0.5 };
                        self.sfx(ctx, "confirm");
                        State::Fortune { phase: Phase::Double { win }, sel: 0, t: 0.0 }
                    }
                    Some(_) => {
                        self.world.remove_npc(NpcKind::Fortune);
                        let id = self.world.map.music.clone();
                        self.play_music(ctx, &id, 1.0);
                        self.resume(ctx);
                        return None;
                    }
                    None => State::Fortune { phase, sel, t },
                }
            }
            Phase::Stake => {
                let n = fortune::STAKES.len() + 1;
                let (mut pick, moved) = menus::input_levelup(nav, view, n, &mut sel, t);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                if nav.back && ready {
                    pick = Some(n - 1);
                }
                match pick {
                    Some(k) if k < fortune::STAKES.len() => {
                        let stake = fortune::STAKES[k];
                        if self.profile.rupees < stake {
                            self.sfx(ctx, "error");
                            State::Fortune { phase, sel, t }
                        } else {
                            // la mise quitte la banque tout de suite (enregistree : pas de triche en quittant)
                            self.profile.rupees -= stake;
                            self.fortune_played[0] = true;
                            let root = ctx.root.clone();
                            self.profile.save(&root);
                            self.sfx(ctx, "confirm");
                            State::Fortune { phase: Phase::Chests { stake }, sel: 1, t: 0.0 }
                        }
                    }
                    Some(_) => {
                        self.sfx(ctx, "menu_close");
                        State::Fortune { phase: Phase::Menu, sel: 0, t: 0.0 }
                    }
                    None => State::Fortune { phase, sel, t },
                }
            }
            Phase::Chests { stake } => {
                let (pick, moved) = fortune::input_chests(nav, view, &mut sel, t);
                if moved {
                    self.sfx(ctx, "cursor");
                }
                match pick {
                    Some(k) => {
                        let mut mults = fortune::CHEST_MULTS;
                        for i in (1..mults.len()).rev() {
                            mults.swap(i, self.rng.below(i as u32 + 1) as usize);
                        }
                        let m = mults[k];
                        self.profile.rupees += stake * m as u64;
                        let root = ctx.root.clone();
                        self.profile.save(&root);
                        self.sfx(ctx, "chest_open");
                        match m {
                            0 => self.sfx(ctx, "error"),
                            1 => {}
                            _ => {
                                self.sfx(ctx, "fortune_win");
                                self.juice.flash([1.0, 0.9, 0.5, 0.35], 0.4);
                            }
                        }
                        State::Fortune { phase: Phase::Opened { stake, mults, pick: k }, sel: k, t: 0.0 }
                    }
                    None => State::Fortune { phase, sel, t },
                }
            }
            Phase::Opened { .. } | Phase::Double { .. } => {
                let mut lock = fortune::RESULT_LOCK;
                if let Phase::Double { win } = phase {
                    lock += fortune::DOUBLE_GAZE;
                    // la boule revele le resultat
                    if t >= fortune::DOUBLE_GAZE && t - dt < fortune::DOUBLE_GAZE {
                        if win {
                            self.sfx(ctx, "fortune_win");
                            self.juice.flash([1.0, 0.9, 0.5, 0.4], 0.4);
                        } else {
                            self.sfx(ctx, "error");
                            self.juice.flash([1.0, 0.3, 0.3, 0.3], 0.3);
                        }
                    }
                }
                if t >= lock && (nav.confirm || nav.click || nav.back || self.opts.demo) {
                    self.sfx(ctx, "confirm");
                    let sel = (0..2).find(|&k| !self.fortune_played[k]).unwrap_or(2);
                    State::Fortune { phase: Phase::Menu, sel, t: 0.0 }
                } else {
                    State::Fortune { phase, sel, t }
                }
            }
        })
    }

    /// Sorciere : achat d'une potion (rubis de la partie) ou rien. `None` quand la partie reprend.
    pub(super) fn update_witch(&mut self, ctx: &mut Ctx, nav: &NavIn, mut sel: usize, t: f32) -> Option<State> {
        let n = self.witch_offer.len() + 1;
        let (mut pick, moved) = menus::input_levelup(nav, ctx.view, n, &mut sel, t);
        if moved {
            self.sfx(ctx, "cursor");
        }
        if nav.back && t >= menus::LEVELUP_LOCK {
            pick = Some(n - 1);
        }
        if self.opts.demo && t > 1.2 {
            let rupees = self.world.player.rupees;
            pick = Some(self.witch_offer.iter().position(|p| p.price <= rupees).unwrap_or(n - 1));
        }
        match pick {
            Some(k) if k + 1 < n => {
                let p = self.witch_offer[k].clone();
                if self.world.player.rupees < p.price {
                    self.sfx(ctx, "error");
                    return Some(State::Witch { sel, t });
                }
                self.world.player.rupees -= p.price;
                self.world.add_run_stats(&witch::effects(&p));
                self.witch_offer.clear();
                self.world.remove_npc(NpcKind::Witch);
                self.drink = Some((p.color, ctx.real_time));
                self.sfx(ctx, "potion_drink");
                let col = POTION_COLORS[p.color];
                self.juice.flash(with_alpha(col, 0.3), 0.35);
                let at = self.world.player.pos - Vec2::new(0.0, 12.0);
                self.juice.ring(at, 4.0, 60.0, 0.6, col);
                self.juice.debris(at, Vec2::new(0.0, -1.0), 30, col, 90.0, true);
                let bonus: Vec<String> = p.bonus.iter().map(|&(s, v)| menus::stat_text(&self.lang, s, v)).collect();
                self.hud.toasts.push(crate::hud::Toast {
                    icon: self.vis.potions[p.color],
                    title: self.lang.get("ui.witch.drunk").to_string(),
                    name: bonus.join(", "),
                    sub: format!("{} {}", self.lang.get("ui.witch.malus"), menus::stat_text(&self.lang, p.malus.0, p.malus.1)),
                    t: 0.0,
                    life: 4.5,
                });
                self.resume(ctx);
                None
            }
            Some(_) => {
                self.sfx(ctx, "menu_close");
                self.resume(ctx);
                None
            }
            None => Some(State::Witch { sel, t }),
        }
    }

    pub(super) fn draw_fortune(&self, ctx: &Ctx, list: &mut DrawList, phase: &Phase, sel: usize, t: f32) {
        let lang = &self.lang;
        let vis = &self.vis;
        let head = (lang.get("ui.fortune.title"), lang.get("ui.fortune.sub"), fortune::PURPLE);
        match phase {
            Phase::Menu => {
                let infos = fortune::menu_infos(vis, lang, self.fortune_played, self.run_bank_if_won());
                menus::draw_cards(list, ctx, vis, lang, head, &infos, sel, t);
            }
            Phase::Stake => {
                let infos = fortune::stake_infos(vis, lang, self.profile.rupees);
                menus::draw_cards(list, ctx, vis, lang, (lang.get("ui.fortune.chests"), lang.get("ui.fortune.stake_sub"), fortune::PURPLE), &infos, sel, t);
            }
            Phase::Chests { stake } => fortune::draw_chests(list, ctx, vis, lang, *stake, None, sel, t),
            Phase::Opened { stake, mults, pick } => fortune::draw_chests(list, ctx, vis, lang, *stake, Some((*mults, *pick)), sel, t),
            Phase::Double { win } => fortune::draw_double(list, ctx, vis, lang, *win, t),
        }
    }

    pub(super) fn draw_witch(&self, ctx: &Ctx, list: &mut DrawList, sel: usize, t: f32) {
        let lang = &self.lang;
        let rupees = self.world.player.rupees;
        let mut infos: Vec<menus::OfferInfo> = self.witch_offer.iter().map(|p| witch::info(&self.vis, lang, p, rupees)).collect();
        infos.push(menus::OfferInfo {
            icon: self.vis.potions[3],
            name: lang.get("ui.witch.none").to_string(),
            tag: String::new(),
            is_new: false,
            desc: lang.get("ui.witch.none_desc").to_string(),
            level: 0,
            max: 0,
            evo: Vec::new(),
            evo_label: String::new(),
            keep: false,
            fusion: 0,
        });
        let sub = lang.get("ui.witch.sub").replace("{n}", &rupees.to_string());
        menus::draw_cards(list, ctx, &self.vis, lang, (lang.get("ui.witch.title"), &sub, WITCH), &infos, sel, t);
    }

    /// PNJ dans le monde (apres les objets au sol) : ombre, sprite et bulle « ! » qui sautille.
    pub(super) fn draw_npcs(&self, ctx: &Ctx, list: &mut DrawList, lo: Vec2, hi: Vec2) {
        let atlas = &ctx.atlas;
        let white = &self.vis.white;
        let rt = ctx.real_time as f32;
        let shadow = atlas.frame(self.vis.shadow, 0.0);
        for n in &self.world.npcs {
            if n.pos.x < lo.x || n.pos.y < lo.y || n.pos.x > hi.x || n.pos.y > hi.y {
                continue;
            }
            let anim = match n.kind {
                NpcKind::Fortune => self.vis.npc_fortune,
                NpcKind::Witch => self.vis.npc_witch,
            };
            let a = (n.t / 0.4).min(1.0);
            // la sorciere clignote pendant ses 5 dernieres secondes
            if n.life - n.t < 5.0 && (rt * 8.0).fract() < 0.35 {
                continue;
            }
            list.frame(shadow, n.pos, &Params::default().scale(Vec2::new(2.0, 1.0)).color([1.0, 1.0, 1.0, 0.55 * a]));
            let f = atlas.frame(anim, rt);
            list.frame(f, n.pos, &Params::default().color([1.0, 1.0, 1.0, a]));
            if n.kind == NpcKind::Fortune {
                fortune::draw_table(list, &self.vis, n.pos + Vec2::new(0.0, 1.0), 1.0, fortune::PURPLE);
            }
            let bob = (2.0 * (rt * 4.0).sin()).round();
            let top = (n.pos - Vec2::new(0.0, f.h as f32 + 9.0 + bob)).round();
            // bulle : contour sombre, fond blanc, point d'exclamation rouge, petite pointe
            list.rect(white, top - Vec2::new(6.0, 7.0), Vec2::new(12.0, 14.0), [0.1, 0.05, 0.15, a]);
            list.rect(white, top - Vec2::new(5.0, 6.0), Vec2::new(10.0, 12.0), [1.0, 1.0, 1.0, a]);
            list.rect(white, top - Vec2::new(1.0, 4.0), Vec2::new(2.0, 5.0), [0.85, 0.1, 0.15, a]);
            list.rect(white, top + Vec2::new(-1.0, 2.0), Vec2::new(2.0, 2.0), [0.85, 0.1, 0.15, a]);
            list.rect(white, top + Vec2::new(-1.0, 7.0), Vec2::new(3.0, 2.0), [0.1, 0.05, 0.15, a]);
            list.rect(white, top + Vec2::new(0.0, 9.0), Vec2::new(1.0, 1.0), [0.1, 0.05, 0.15, a]);
        }
    }

    /// Fleches vers les PNJ hors champ ; temps restant de la sorciere quand elle est visible.
    pub(super) fn draw_npc_markers(&self, ctx: &Ctx, list: &mut DrawList, cam: Vec2) {
        let view = ctx.view;
        let font = ctx.atlas.font("main");
        let blink = (ctx.real_time * 3.0).fract() < 0.65;
        for n in &self.world.npcs {
            let ui = self.to_ui(n.pos, cam, view);
            let col = if n.kind == NpcKind::Witch { WITCH } else { fortune::PURPLE };
            match Self::edge_point(view, ui) {
                Some((c, d)) => self.edge_arrow(ctx, list, c, d, col, true),
                None if n.kind == NpcKind::Witch => {
                    let s = format!("{}s", (n.life - n.t).ceil() as i32);
                    crate::hud::text(list, font, &s, ui + Vec2::new(0.0, 6.0), 1.0, with_alpha(col, if blink { 1.0 } else { 0.7 }), engine::draw::Align::Center);
                }
                None => {}
            }
        }
    }

    /// Potion bue : la fiole monte au-dessus de Link, se penche, des bulles de sa couleur montent.
    pub(super) fn draw_drink(&self, ctx: &Ctx, list: &mut DrawList, ui: Vec2) {
        let Some((c, t0)) = self.drink else { return };
        let age = (ctx.real_time - t0) as f32;
        if age >= DRINK_TIME {
            return;
        }
        let white = &self.vis.white;
        list.set(Space::Ui, Blend::Alpha);
        let rise = (age / 0.3).min(1.0);
        let tilt = ((age - 0.3) / 0.4).clamp(0.0, 1.0) * 2.0;
        let fade = 1.0 - ((age - 1.0) / 0.4).clamp(0.0, 1.0);
        let f = ctx.atlas.frame(self.vis.potions[c], 0.0);
        let at = (ui - Vec2::new(0.0, 16.0 + 10.0 * rise)).round();
        list.frame(f, at, &Params::default().rot(-tilt).scale(Vec2::splat(2.0)).color([1.0, 1.0, 1.0, fade]));
        list.set(Space::Ui, Blend::Add);
        let col = POTION_COLORS[c];
        for k in 0..10 {
            let ph = (age * 1.2 + k as f32 * 0.29).fract();
            let x = ((k as f32 * 2.3).sin() * 12.0).round();
            let q = ui + Vec2::new(x, 4.0 - ph * 34.0);
            let s = if k % 3 == 0 { 2.0 } else { 1.0 };
            list.rect(white, q.round(), Vec2::splat(s), with_alpha(col, (1.0 - ph) * fade));
        }
        list.set(Space::Ui, Blend::Alpha);
    }
}
