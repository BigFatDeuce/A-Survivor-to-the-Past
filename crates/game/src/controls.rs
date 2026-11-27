//! Ecran Commandes (options) : reattribution des touches du clavier / de la souris et des boutons
//! de la manette pour les actions de jeu. Les menus gardent leurs touches d'origine.
use crate::hub::{Row, DIMMED, RED};
use crate::lang::Lang;
use engine::input::{Binding, Bindings, Input, KeyCode, MouseButton, PadButton};

pub const TABS: [&str; 2] = ["ui.tab.keyboard", "ui.tab.gamepad"];

/// Actions reattribuables (dans l'ordre de l'ecran) ; la ligne suivante remet tout a zero.
pub const ACTIONS: [&str; 9] = ["up", "down", "left", "right", "attack", "dash", "toggle_auto", "zoom", "pause"];
pub const RESET: usize = ACTIONS.len();

/// Attente d'une touche : abandon apres ce delai (s).
pub const WAIT: f32 = 6.0;

fn is_pad(b: &Binding) -> bool {
    matches!(b, Binding::Pad(_))
}

fn on_tab(b: &Binding, tab: usize) -> bool {
    is_pad(b) == (tab == 1)
}

pub fn key_name(input: &Input, lang: &Lang, k: KeyCode) -> String {
    let key = match k {
        KeyCode::Space => "key.space",
        KeyCode::Escape => "key.escape",
        KeyCode::Enter | KeyCode::NumpadEnter => "key.enter",
        KeyCode::Tab => "key.tab",
        KeyCode::Backspace => "key.backspace",
        KeyCode::ArrowUp => "key.up",
        KeyCode::ArrowDown => "key.down",
        KeyCode::ArrowLeft => "key.left",
        KeyCode::ArrowRight => "key.right",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "key.shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "key.ctrl",
        KeyCode::AltLeft | KeyCode::AltRight => "key.alt",
        _ => "",
    };
    if !key.is_empty() {
        return lang.get(key).to_string();
    }
    if let Some(l) = input.key_label(k) {
        return l.to_string();
    }
    let s = format!("{k:?}");
    s.strip_prefix("Key").or_else(|| s.strip_prefix("Digit")).unwrap_or(&s).to_string()
}

pub fn binding_name(input: &Input, lang: &Lang, b: &Binding) -> String {
    match *b {
        Binding::Key(k) => key_name(input, lang, k),
        Binding::Mouse(m) => lang
            .get(match m {
                MouseButton::Left => "key.mouse_left",
                MouseButton::Right => "key.mouse_right",
                MouseButton::Middle => "key.mouse_middle",
            })
            .to_string(),
        Binding::Pad(p) => match p {
            PadButton::South => "A".into(),
            PadButton::East => "B".into(),
            PadButton::West => "X".into(),
            PadButton::North => "Y".into(),
            PadButton::LeftTrigger => "LB".into(),
            PadButton::RightTrigger => "RB".into(),
            PadButton::LeftTrigger2 => "LT".into(),
            PadButton::RightTrigger2 => "RT".into(),
            PadButton::Start => "Start".into(),
            PadButton::Select => "Select".into(),
            PadButton::LeftThumb => "L3".into(),
            PadButton::RightThumb => "R3".into(),
            PadButton::DPadUp => lang.get("key.dpad_up").to_string(),
            PadButton::DPadDown => lang.get("key.dpad_down").to_string(),
            PadButton::DPadLeft => lang.get("key.dpad_left").to_string(),
            PadButton::DPadRight => lang.get("key.dpad_right").to_string(),
            other => format!("{other:?}"),
        },
    }
}

pub fn rows(input: &Input, lang: &Lang, b: &Bindings, tab: usize, waiting: Option<usize>) -> (Vec<Row>, Vec<String>) {
    let mut rows = Vec::new();
    let mut details = Vec::new();
    for (k, a) in ACTIONS.iter().enumerate() {
        let names: Vec<String> = b.actions.get(*a).into_iter().flatten().filter(|x| on_tab(x, tab)).map(|x| binding_name(input, lang, x)).collect();
        let wait = waiting == Some(k);
        rows.push(Row {
            label: lang.get(&format!("act.{a}")).to_string(),
            value: if wait {
                "...".into()
            } else if names.is_empty() {
                "-".into()
            } else {
                names.join(", ")
            },
            value_color: Some(if wait { RED } else if names.is_empty() { DIMMED } else { crate::hud::GOLD }),
            ..Default::default()
        });
        details.push(lang.get(if wait {
            if tab == 1 {
                "ui.press_button"
            } else {
                "ui.press_key"
            }
        } else {
            "ui.rebind_hint"
        }).to_string());
    }
    rows.push(Row { label: lang.get("ui.reset_keys").to_string(), ..Default::default() });
    details.push(lang.get("ui.reset_keys_desc").to_string());
    (rows, details)
}

/// Touche lue pendant l'attente : `Some(None)` = abandon (Echap), `Some(Some(b))` = nouvelle touche.
pub fn read_new(input: &Input, tab: usize) -> Option<Option<Binding>> {
    if input.any_key_pressed() == Some(KeyCode::Escape) {
        return Some(None);
    }
    if tab == 1 {
        return input.any_pad_pressed().map(|p| Some(Binding::Pad(p)));
    }
    if let Some(k) = input.any_key_pressed() {
        return Some(Some(Binding::Key(k)));
    }
    input.any_mouse_pressed().map(|m| Some(Binding::Mouse(m)))
}

/// Remplace la premiere touche de l'onglet pour l'action ; la touche est retiree des autres actions
/// reattribuables (pas de doublon). Renvoie les listes modifiees (action -> touches).
pub fn rebind(b: &Bindings, action: &str, tab: usize, new: Binding) -> Vec<(String, Vec<Binding>)> {
    let mut out = Vec::new();
    for a in ACTIONS {
        let mut list = b.actions.get(a).cloned().unwrap_or_default();
        if a == action {
            list.retain(|x| *x != new);
            match list.iter().position(|x| on_tab(x, tab)) {
                Some(i) => list[i] = new,
                None => list.insert(0, new),
            }
            out.push((a.to_string(), list));
        } else if list.contains(&new) {
            list.retain(|x| *x != new);
            out.push((a.to_string(), list));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebind_replaces_and_moves() {
        let mut b = Bindings::default();
        b.actions.insert("attack".into(), vec![Binding::Key(KeyCode::KeyJ), Binding::Pad(PadButton::West)]);
        b.actions.insert("dash".into(), vec![Binding::Key(KeyCode::KeyK), Binding::Key(KeyCode::Space)]);
        let ch = rebind(&b, "attack", 0, Binding::Key(KeyCode::Space));
        let attack = &ch.iter().find(|(a, _)| a == "attack").unwrap().1;
        let dash = &ch.iter().find(|(a, _)| a == "dash").unwrap().1;
        assert_eq!(attack, &vec![Binding::Key(KeyCode::Space), Binding::Pad(PadButton::West)]);
        assert_eq!(dash, &vec![Binding::Key(KeyCode::KeyK)]);
        let ch = rebind(&b, "dash", 1, Binding::Pad(PadButton::South));
        assert_eq!(ch[0].1, vec![Binding::Pad(PadButton::South), Binding::Key(KeyCode::KeyK), Binding::Key(KeyCode::Space)]);
    }
}
