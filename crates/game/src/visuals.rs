//! Poignees graphiques resolues une fois au chargement (aucune recherche par nom pendant le jeu).
use assetkit::meta::Frame;
use engine::assets::{AnimId, Atlas};
use sim::defs::{CharacterDef, Content, EnemyAnims};
use sim::horde::{FACE_DOWN, FACE_LEFT, FACE_RIGHT, FACE_UP};

pub enum EnemyVis {
    Dir4 { down: AnimId, side: AnimId, up: AnimId, attack: Option<[AnimId; 3]>, side_faces_left: bool },
    Single(AnimId),
    Hop { idle: AnimId, jump: AnimId },
}

impl EnemyVis {
    /// Animation et miroir selon l'orientation, la hauteur et l'attaque en cours.
    #[inline]
    pub fn pick(&self, face: u8, z: f32, attacking: bool) -> (AnimId, bool) {
        match *self {
            EnemyVis::Dir4 { down, side, up, attack, side_faces_left } => {
                let [d, s, u] = match attack {
                    Some(a) if attacking => a,
                    _ => [down, side, up],
                };
                match face {
                    FACE_UP => (u, false),
                    FACE_DOWN => (d, false),
                    f => (s, if side_faces_left { f == FACE_RIGHT } else { f == FACE_LEFT }),
                }
            }
            EnemyVis::Single(a) => (a, false),
            EnemyVis::Hop { idle, jump } => (if z > 0.5 { jump } else { idle }, false),
        }
    }
}

pub struct CharVis {
    pub idle: [AnimId; 3],
    pub walk: [AnimId; 3],
    pub weapon: AnimId,
    /// Lame de l'epee de legende.
    pub weapon_master: AnimId,
}

impl CharVis {
    /// (animation, miroir) pour une orientation ; le cote d'origine regarde a droite.
    pub fn pick(&self, face: u8, moving: bool) -> (AnimId, bool) {
        let set = if moving { &self.walk } else { &self.idle };
        match face {
            FACE_DOWN => (set[0], false),
            FACE_UP => (set[2], false),
            f => (set[1], f == FACE_LEFT),
        }
    }
}

/// Comment orienter un projectile a l'ecran.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Orient {
    /// Image fixe (boule de feu, bombe).
    Fixed,
    /// Tourne selon l'angle du projectile ; `true` = l'image d'origine pointe vers le haut.
    Angle(bool),
}

/// Choisit l'orientation d'apres la forme de l'image : allongee = suit sa direction.
fn orient_of(atlas: &Atlas, id: AnimId, spin: bool) -> Orient {
    let f = atlas.frame(id, 0.0);
    if spin {
        Orient::Angle(false)
    } else if f.w > f.h + 2 {
        Orient::Angle(false)
    } else if f.h > f.w + 2 {
        Orient::Angle(true)
    } else {
        Orient::Fixed
    }
}

pub struct WeaponVis {
    pub icon: AnimId,
    /// Projectile, flamme, objet en orbite (selon la famille).
    pub sprite: Option<AnimId>,
    pub orient: Orient,
    pub beam: Option<(AnimId, Orient)>,
    /// Teinte de l'image (fleches d'argent : reflet argente).
    pub tint: Option<[f32; 3]>,
}

pub struct Vis {
    pub player: CharVis,
    pub enemies: Vec<EnemyVis>,
    pub shadow_size: Vec<u8>,
    /// Agrandissement d'affichage par type d'ennemi.
    pub enemy_scale: Vec<f32>,
    pub weapons: Vec<WeaponVis>,
    pub passives: Vec<AnimId>,
    /// Images des tirs ennemis (indice = `EnemyShot::sprite`) et leur orientation.
    pub shots: Vec<(AnimId, Orient)>,
    pub rupees: [AnimId; 3],
    pub heart_small: AnimId,
    pub chest: [AnimId; 2],
    pub chest_open: [AnimId; 2],
    pub shadow: AnimId,
    pub poof: AnimId,
    pub spark: AnimId,
    pub dust: AnimId,
    pub explosion: AnimId,
    pub flame: AnimId,
    pub flame_fade: AnimId,
    pub ice_shatter: AnimId,
    pub icon_heart: AnimId,
    pub icon_rupees: AnimId,
    pub hud_heart: [AnimId; 3],
    pub ground: Vec<Frame>,
    pub white: Frame,
}

impl Vis {
    pub fn new(atlas: &Atlas, content: &Content, ch: &CharacterDef, ground: &str) -> Vis {
        let s = &ch.sprite;
        let player = CharVis {
            idle: [atlas.get(s, "idle_down"), atlas.get(s, "idle_side"), atlas.get(s, "idle_up")],
            walk: [atlas.get(s, "walk_down"), atlas.get(s, "walk_side"), atlas.get(s, "walk_up")],
            weapon: atlas.get(if ch.weapon_sprite.is_empty() { "sword1" } else { &ch.weapon_sprite }, "h"),
            weapon_master: atlas.find("sword2", "h").unwrap_or_else(|| atlas.get("sword1", "h")),
        };
        let enemies = content
            .enemies
            .iter()
            .map(|e| match &e.anims {
                EnemyAnims::Dir4 { prefix, side_faces_left, attack } => EnemyVis::Dir4 {
                    down: atlas.get(&e.sprite, &format!("{prefix}_down")),
                    side: atlas.get(&e.sprite, &format!("{prefix}_side")),
                    up: atlas.get(&e.sprite, &format!("{prefix}_up")),
                    attack: attack.as_ref().map(|a| {
                        [
                            atlas.get(&e.sprite, &format!("{a}_down")),
                            atlas.get(&e.sprite, &format!("{a}_side")),
                            atlas.get(&e.sprite, &format!("{a}_up")),
                        ]
                    }),
                    side_faces_left: *side_faces_left,
                },
                EnemyAnims::Single(a) => EnemyVis::Single(atlas.get(&e.sprite, a)),
                EnemyAnims::Hop { idle, jump } => {
                    EnemyVis::Hop { idle: atlas.get(&e.sprite, idle), jump: atlas.get(&e.sprite, jump) }
                }
            })
            .collect();
        let weapons = content
            .weapon_ids
            .iter()
            .zip(&content.weapons)
            .map(|(id, w)| {
                let sprite = (!w.sprite.is_empty()).then(|| atlas.get(&w.sprite, "0"));
                let orient = sprite.map(|a| orient_of(atlas, a, w.spin != 0.0)).unwrap_or(Orient::Fixed);
                let beam = w.beam.as_ref().map(|b| {
                    let a = atlas.get(&b.sprite, "0");
                    (a, orient_of(atlas, a, false))
                });
                WeaponVis {
                    icon: atlas.get(&w.icon, "0"),
                    sprite,
                    orient,
                    beam,
                    tint: (id == "silver_bow").then_some([0.8, 0.9, 1.0]),
                }
            })
            .collect();
        let shots = content_shots(atlas, content);
        Vis {
            player,
            enemies,
            shadow_size: content.enemies.iter().map(|e| e.shadow).collect(),
            enemy_scale: content.enemies.iter().map(|e| e.scale.max(1.0).round()).collect(),
            weapons,
            passives: content.passives.iter().map(|p| atlas.get(&p.icon, "0")).collect(),
            shots,
            rupees: [atlas.get("rupee_green", "0"), atlas.get("rupee_blue", "0"), atlas.get("rupee_red", "0")],
            heart_small: atlas.get("heart_small", "0"),
            chest: [atlas.get("chest_small", "closed"), atlas.get("chest_big", "closed")],
            chest_open: [atlas.get("chest_small", "open"), atlas.get("chest_big", "open")],
            shadow: atlas.get("shadow", "0"),
            poof: atlas.get("fx_poof", "0"),
            spark: atlas.get("fx_spark", "0"),
            dust: atlas.get("fx_dust", "0"),
            explosion: atlas.get("fx_explosion", "0"),
            flame: atlas.get("fx_flame", "0"),
            flame_fade: atlas.get("fx_flame_fade", "0"),
            ice_shatter: atlas.get("fx_ice_shatter", "0"),
            icon_heart: atlas.get("icon_heart", "0"),
            icon_rupees: atlas.get("icon_rupees", "0"),
            hud_heart: [atlas.get("hud_heart", "full"), atlas.get("hud_heart", "half"), atlas.get("hud_heart", "empty")],
            ground: atlas.tiles(ground).to_vec(),
            white: atlas.white,
        }
    }
}

/// Images des tirs ennemis, dans l'ordre de `World::shot_sprites`.
fn content_shots(atlas: &Atlas, content: &Content) -> Vec<(AnimId, Orient)> {
    sim::enemies::collect_shot_sprites(content)
        .iter()
        .map(|s| {
            let a = atlas.get(s, "0");
            (a, orient_of(atlas, a, false))
        })
        .collect()
}
