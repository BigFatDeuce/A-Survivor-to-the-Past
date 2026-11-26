//! Ecran Classements : meilleurs scores par stage. Mes parties (a publier en ligne), joueurs locaux
//! (les 5 profils de cet ordinateur) et monde (top 100 en ligne, par version du jeu ; la plus recente
//! par defaut).
use sim::defs::Content;
use crate::hub::{Row, DIMMED, LOST, PUBLISHED, WON};
use crate::hud::GOLD;
use crate::lang::Lang;
use crate::online::{Board, Entry, Status};
use crate::players;
use crate::profile::{Profile, ScoreRecord};
use crate::stages::{fmt_time, stage_name};
use engine::assets::{AnimId, Atlas};
use engine::draw::{rgb, Color};

pub const TABS: [&str; 3] = ["ui.scores_mine", "ui.scores_players", "ui.scores_world"];
pub const MINE: usize = 0;
pub const PLAYERS: usize = 1;
pub const WORLD: usize = 2;
/// Premiere ligne : choix du stage ; onglet Monde, deuxieme ligne : choix de la version.
pub const STAGE_ROW: usize = 0;
pub const VERSION_ROW: usize = 1;

const SILVER: Color = rgb(200, 214, 230);
const BRONZE: Color = rgb(222, 150, 96);

/// Lignes de reglage en tete de liste.
pub fn head(tab: usize) -> usize {
    if tab == WORLD {
        2
    } else {
        1
    }
}

/// Score lisible : chiffres groupes par trois (« 12 345 »).
pub fn fmt_score(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn rank_color(rank: usize) -> Option<Color> {
    match rank {
        1 => Some(GOLD),
        2 => Some(SILVER),
        3 => Some(BRONZE),
        _ => None,
    }
}

pub struct View<'a> {
    pub lang: &'a Lang,
    pub content: &'a Content,
    pub atlas: &'a Atlas,
    pub stage: &'a str,
    /// Joueur actif (a jour) et les 5 emplacements lus sur le disque.
    pub profile: &'a Profile,
    pub slots: &'a [Option<Profile>],
    /// Noms des difficultes (dans l'ordre de `meta.ron`).
    pub difficulties: &'a [String],
    /// Titres offerts a la Grande Fee (dans l'ordre de `meta.ron`).
    pub titles: &'a [String],
    pub board: &'a ((String, String), Status<Board>),
    /// Version demandee (vide : la plus recente du serveur).
    pub version: &'a str,
}

impl View<'_> {
    fn char_icon(&self, id: &str) -> Option<AnimId> {
        let def = self.content.characters.get(id)?;
        self.atlas.find(&def.sprite, "idle_down")
    }

    fn char_name(&self, id: &str) -> String {
        if id.is_empty() {
            "?".into()
        } else {
            crate::chars::name(self.lang, id)
        }
    }

    fn difficulty(&self, k: usize) -> String {
        self.difficulties.get(k).cloned().unwrap_or_default()
    }

    #[allow(clippy::too_many_arguments)]
    fn detail(&self, character: &str, time: f32, kills: u32, level: Option<u32>, difficulty: usize, won: bool, version: &str) -> String {
        let mut s = self
            .lang
            .get("ui.score_detail")
            .replace("{c}", &self.char_name(character))
            .replace("{t}", &fmt_time(time))
            .replace("{k}", &kills.to_string())
            .replace("{d}", &self.difficulty(difficulty));
        if let Some(l) = level {
            s.push_str(&format!(" - {}", self.lang.get("ui.score_level").replace("{n}", &l.to_string())));
        }
        if won {
            s.push_str(&format!(" - {}", self.lang.get("ui.score_won")));
        }
        if !version.is_empty() {
            s.push_str(&format!(" - v{version}"));
        }
        s
    }

    fn record_detail(&self, r: &ScoreRecord) -> String {
        self.detail(&r.character, r.time, r.kills, Some(r.level), r.difficulty, r.won, &r.version)
    }

    fn entry_detail(&self, e: &Entry, version: &str) -> String {
        let d = self.detail(&e.character, e.time, e.kills, None, e.difficulty, e.won, version);
        match self.titles.get((e.title as usize).wrapping_sub(1)) {
            Some(t) => format!("{} - {d}", crate::shop::titled_name(e.title, t)),
            None => d,
        }
    }

    /// Classement en ligne du stage choisi, s'il correspond a la demande.
    pub fn board(&self) -> Option<&Status<Board>> {
        let (key, status) = self.board;
        (key.0 == self.stage && key.1 == self.version).then_some(status)
    }

    /// Version affichee dans l'onglet Monde.
    pub fn shown_version(&self) -> String {
        if !self.version.is_empty() {
            return self.version.to_string();
        }
        match self.board() {
            Some(Status::Done(b)) => b.version.clone(),
            _ => "...".into(),
        }
    }
}

/// Lignes et details de l'onglet.
pub fn rows(v: &View, tab: usize) -> (Vec<Row>, Vec<String>) {
    let l = v.lang;
    let mut rows = vec![Row {
        label: l.get("ui.stage").to_string(),
        value: format!("◀ {} ▶", stage_name(l, v.stage)),
        value_color: Some(GOLD),
        ..Default::default()
    }];
    let mut details = vec![format!("{}\n{}", l.get("ui.scores_stage_hint"), l.get("ui.scores_legend"))];
    let empty = |rows: &mut Vec<Row>, details: &mut Vec<String>, key: &str| {
        rows.push(Row { label: l.get(key).to_string(), dim: true, ..Default::default() });
        details.push(String::new());
    };
    match tab {
        MINE => {
            let list = v.profile.scores.get(v.stage).map(|s| s.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                empty(&mut rows, &mut details, "ui.no_scores");
            }
            for (k, r) in list.iter().enumerate() {
                rows.push(Row {
                    icon: v.char_icon(&r.character),
                    label: format!("{}. {}", k + 1, fmt_score(r.score)),
                    value: if r.published { l.get("ui.published").to_string() } else { fmt_time(r.time) },
                    value_color: Some(if r.published { PUBLISHED } else { rgb(228, 228, 244) }),
                    label_color: Some(if r.won { WON } else { LOST }),
                    mark: r.published,
                    ..Default::default()
                });
                let hint = if r.published { l.get("ui.published_hint") } else { l.get("ui.publish_hint") };
                details.push(format!("{}\n{hint}", v.record_detail(r)));
            }
        }
        PLAYERS => {
            // meilleur score de chaque joueur local (le joueur actif tel qu'en memoire)
            let mut best: Vec<(Option<&ScoreRecord>, &Profile)> = Vec::new();
            for (k, slot) in v.slots.iter().enumerate() {
                let p = if k == v.profile.slot { Some(v.profile) } else { slot.as_ref() };
                if let Some(p) = p {
                    best.push((p.best_score(v.stage), p));
                }
            }
            best.sort_by(|a, b| b.0.map(|r| r.score).cmp(&a.0.map(|r| r.score)));
            for (k, (rec, p)) in best.iter().enumerate() {
                rows.push(Row {
                    icon: players::avatar_anim(v.atlas, p.avatar),
                    label: format!("{}. {}", k + 1, p.name),
                    value: rec.map(|r| fmt_score(r.score)).unwrap_or_else(|| "-".into()),
                    value_color: if rec.is_some() { rank_color(k + 1) } else { Some(DIMMED) },
                    label_color: rec.map(|r| if r.won { WON } else { LOST }),
                    mark: p.slot == v.profile.slot,
                    ..Default::default()
                });
                details.push(match rec {
                    Some(r) => v.record_detail(r),
                    None => l.get("ui.no_scores").to_string(),
                });
            }
        }
        _ => {
            rows.push(Row {
                label: l.get("ui.version").to_string(),
                value: format!("◀ {} ▶", v.shown_version()),
                value_color: Some(GOLD),
                ..Default::default()
            });
            details.push(l.get("ui.version_hint").to_string());
            match v.board() {
                Some(Status::Done(b)) if b.scores.is_empty() => empty(&mut rows, &mut details, "ui.no_scores_world"),
                Some(Status::Done(b)) => {
                    for e in &b.scores {
                        rows.push(Row {
                            icon: v.char_icon(&e.character),
                            label: format!("{}. {}", e.rank, crate::shop::titled_name(e.title, &e.name)),
                            value: fmt_score(e.score),
                            value_color: rank_color(e.rank as usize),
                            label_color: Some(if e.won { WON } else { LOST }),
                            ..Default::default()
                        });
                        details.push(v.entry_detail(e, &b.version));
                    }
                }
                Some(Status::Failed(_)) => empty(&mut rows, &mut details, "ui.online_failed"),
                _ => empty(&mut rows, &mut details, "ui.loading"),
            }
        }
    }
    (rows, details)
}

#[cfg(test)]
mod tests {
    #[test]
    fn score_grouping() {
        assert_eq!(super::fmt_score(0), "0");
        assert_eq!(super::fmt_score(999), "999");
        assert_eq!(super::fmt_score(1000), "1 000");
        assert_eq!(super::fmt_score(1234567), "1 234 567");
    }
}
