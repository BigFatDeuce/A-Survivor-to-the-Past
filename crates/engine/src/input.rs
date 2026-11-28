//! Etat brut des entrees : clavier (touches physiques, independantes de la disposition AZERTY/QWERTY),
//! souris et manette (gilrs). Les « appuis » sont memorises jusqu'au prochain pas de simulation,
//! pour qu'aucun appui bref ne soit perdu a 240 Hz comme a 60 Hz.
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use glam::Vec2;
use std::collections::{HashMap, HashSet};
pub use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Keyboard,
    Mouse,
    Gamepad,
    /// Ecran tactile (telephone, tablette).
    Touch,
}

/// Doigt pose sur l'ecran (pixels de l'image de jeu).
#[derive(Clone, Copy, Debug)]
pub struct TouchPt {
    pub id: u64,
    pub start: Vec2,
    pub pos: Vec2,
}

/// Etape d'un doigt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// Doigt qui pilote la souris des menus : appui court = clic au relacher, glisser = molette.
struct Tap {
    id: u64,
    last: Vec2,
    drag: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub use gilrs::Button as PadButton;

pub struct Input {
    keys: HashSet<KeyCode>,
    keys_pressed: [HashSet<KeyCode>; 2],
    mouse: HashSet<MouseButton>,
    mouse_pressed: [HashSet<MouseButton>; 2],
    /// Position de la souris en pixels ecran.
    pub mouse_px: Vec2,
    /// Souris au-dessus de la fenetre (curseur dessine par le jeu).
    pub mouse_inside: bool,
    pad_buttons: HashSet<Button>,
    pad_pressed: [HashSet<Button>; 2],
    /// Codes de boutons / axes inconnus deja notes dans le journal (une fois chacun).
    pad_logged: HashSet<u32>,
    /// Gachettes analogiques vues (valeurs intermediaires) : seules leurs valeurs comptent.
    analog_triggers: HashSet<Button>,
    /// 0 = lecture par image (menus), 1 = lecture par pas de simulation.
    phase: usize,
    gilrs: Option<Gilrs>,
    pad: Option<GamepadId>,
    pub last_device: Device,
    pub focused: bool,
    /// Caractere produit par chaque touche physique deja pressee (disposition du clavier).
    key_labels: HashMap<KeyCode, String>,
    /// Caracteres tapes depuis la derniere image (saisie d'un nom).
    pub typed: Vec<char>,
    /// Molette depuis la derniere lecture (en crans ; positif = vers le haut), par phase.
    wheel: [f32; 2],
    /// Doigts poses, et ceux poses depuis la derniere lecture (par phase).
    pub touches: Vec<TouchPt>,
    /// (doigt, point de depart) : garde meme si le doigt est deja leve (appui tres bref entre deux images).
    touch_started: [Vec<(u64, Vec2)>; 2],
    tap: Option<Tap>,
    /// Console portable (PortMaster, `ASTP_CONSOLE`) : gilrs lit mal ses boutons ; gptokeyb envoie
    /// F1..F10 et les fleches, traduits ici en boutons de manette.
    console: bool,
}

/// Touches envoyees par gptokeyb en mode console -> bouton de manette.
fn console_button(k: KeyCode) -> Option<Button> {
    Some(match k {
        KeyCode::F1 => Button::South,
        KeyCode::F2 => Button::East,
        KeyCode::F3 => Button::West,
        KeyCode::F4 => Button::North,
        KeyCode::F5 => Button::LeftTrigger,
        KeyCode::F6 => Button::RightTrigger,
        KeyCode::F7 => Button::LeftTrigger2,
        KeyCode::F8 => Button::RightTrigger2,
        KeyCode::F9 => Button::Start,
        KeyCode::F10 => Button::Select,
        KeyCode::ArrowUp => Button::DPadUp,
        KeyCode::ArrowDown => Button::DPadDown,
        KeyCode::ArrowLeft => Button::DPadLeft,
        KeyCode::ArrowRight => Button::DPadRight,
        _ => return None,
    })
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

/// Gachette portee par un axe que gilrs ne range pas en bouton (manette absente de la base SDL,
/// Linux) : ABS_Z / ABS_RZ (deja nommes LeftZ / RightZ) ou ABS_BRAKE / ABS_GAS (Xbox en Bluetooth).
/// `code` : code evdev empaquete par gilrs, (type << 16) | code.
fn trigger_axis(axis: Axis, code: u32) -> Option<Button> {
    const ABS_GAS: u32 = 3 << 16 | 0x09;
    const ABS_BRAKE: u32 = 3 << 16 | 0x0a;
    match (axis, code) {
        (Axis::LeftZ, _) => Some(Button::LeftTrigger2),
        (Axis::RightZ, _) => Some(Button::RightTrigger2),
        (Axis::Unknown, ABS_BRAKE) if cfg!(target_os = "linux") => Some(Button::LeftTrigger2),
        (Axis::Unknown, ABS_GAS) if cfg!(target_os = "linux") => Some(Button::RightTrigger2),
        _ => None,
    }
}

/// Nom, source de la correspondance des boutons (base SDL, pilote ou aucune) et identifiant SDL :
/// de quoi ajouter une manette mal reconnue dans SDL_GAMECONTROLLERCONFIG.
fn log_pad(p: gilrs::Gamepad<'_>) {
    let uuid: String = p.uuid().iter().map(|b| format!("{b:02x}")).collect();
    log::info!("manette : {} ({:?}, {uuid})", p.name(), p.mapping_source());
}

impl Input {
    pub fn new() -> Input {
        let console = std::env::var_os("ASTP_CONSOLE").is_some();
        let gilrs = match (!console).then(Gilrs::new) {
            None => None,
            Some(Ok(g)) => Some(g),
            Some(Err(e)) => {
                log::warn!("manettes indisponibles : {e}");
                None
            }
        };
        let pad = gilrs.as_ref().and_then(|g| g.gamepads().next().map(|(id, _)| id));
        for (_, p) in gilrs.iter().flat_map(|g| g.gamepads()) {
            log_pad(p);
        }
        Input {
            keys: HashSet::new(),
            keys_pressed: Default::default(),
            mouse: HashSet::new(),
            mouse_pressed: Default::default(),
            mouse_px: Vec2::ZERO,
            mouse_inside: false,
            pad_buttons: HashSet::new(),
            pad_pressed: Default::default(),
            pad_logged: HashSet::new(),
            analog_triggers: HashSet::new(),
            phase: 0,
            gilrs,
            pad,
            last_device: Device::Keyboard,
            focused: true,
            key_labels: HashMap::new(),
            typed: Vec::new(),
            wheel: [0.0; 2],
            touches: Vec::new(),
            touch_started: Default::default(),
            tap: None,
            console,
        }
    }

    pub(crate) fn key_event(&mut self, code: KeyCode, down: bool, repeat: bool) {
        if let Some(b) = console_button(code).filter(|_| self.console) {
            if !down {
                self.pad_buttons.remove(&b);
            } else if !repeat {
                self.pad_buttons.insert(b);
                self.pad_pressed.iter_mut().for_each(|s| {
                    s.insert(b);
                });
                self.last_device = Device::Gamepad;
            }
            return;
        }
        if down {
            if !repeat {
                self.keys_pressed.iter_mut().for_each(|s| {
                    s.insert(code);
                });
            }
            self.keys.insert(code);
            self.last_device = Device::Keyboard;
        } else {
            self.keys.remove(&code);
        }
    }

    pub(crate) fn mouse_event(&mut self, b: MouseButton, down: bool) {
        if down {
            self.mouse.insert(b);
            self.mouse_pressed.iter_mut().for_each(|s| {
                s.insert(b);
            });
            self.last_device = Device::Mouse;
        } else {
            self.mouse.remove(&b);
        }
    }

    pub(crate) fn wheel_event(&mut self, notches: f32) {
        self.wheel.iter_mut().for_each(|w| *w += notches);
        self.last_device = Device::Mouse;
    }

    /// Molette depuis la derniere image (crans ; positif = vers le haut).
    pub fn wheel(&self) -> f32 {
        self.wheel[self.phase]
    }

    pub(crate) fn mouse_moved(&mut self, p: Vec2) {
        if (p - self.mouse_px).length_squared() > 4.0 {
            self.last_device = Device::Mouse;
        }
        self.mouse_px = p;
    }

    /// Doigt pose, deplace ou leve (`p` : pixels de l'image de jeu ; `row` : hauteur d'une ligne de liste en
    /// pixels, pour le defilement au glisser). Le premier doigt sert aussi de souris aux menus.
    pub(crate) fn touch_event(&mut self, id: u64, phase: TouchPhase, p: Vec2, row: f32) {
        self.last_device = Device::Touch;
        match phase {
            TouchPhase::Start => {
                self.touches.retain(|t| t.id != id);
                self.touches.push(TouchPt { id, start: p, pos: p });
                self.touch_started.iter_mut().for_each(|s| s.push((id, p)));
                if self.tap.is_none() {
                    self.tap = Some(Tap { id, last: p, drag: false });
                    self.mouse_px = p;
                    self.mouse_inside = true;
                }
            }
            TouchPhase::Move => {
                if let Some(t) = self.touches.iter_mut().find(|t| t.id == id) {
                    t.pos = p;
                    let start = t.start;
                    if let Some(tap) = self.tap.as_mut().filter(|tap| tap.id == id) {
                        // au-dela de 6 pixels d'interface : c'est un glisser (defilement), plus un appui
                        if !tap.drag && (p - start).length() > row * 0.3 {
                            tap.drag = true;
                        }
                        if tap.drag {
                            let notches = (p.y - tap.last.y) / row.max(1.0);
                            self.wheel.iter_mut().for_each(|w| *w += notches);
                            tap.last = p;
                        }
                    }
                }
            }
            TouchPhase::End | TouchPhase::Cancel => {
                self.touches.retain(|t| t.id != id);
                if let Some(tap) = self.tap.take_if(|tap| tap.id == id) {
                    if phase == TouchPhase::End && !tap.drag {
                        self.mouse_event(MouseButton::Left, true);
                        self.mouse_event(MouseButton::Left, false);
                        self.last_device = Device::Touch;
                    }
                }
            }
        }
    }

    /// Doigts poses depuis la derniere lecture.
    pub fn touch_started(&self) -> &[(u64, Vec2)] {
        &self.touch_started[self.phase]
    }

    pub(crate) fn focus_lost(&mut self) {
        self.keys.clear();
        self.mouse.clear();
        self.touches.clear();
        self.tap = None;
        self.focused = false;
    }

    /// Lit les evenements manette (une fois par image).
    pub(crate) fn poll_gamepad(&mut self) {
        let Some(g) = self.gilrs.as_mut() else { return };
        while let Some(ev) = g.next_event() {
            // manette absente de la base SDL (Linux) : ses gachettes arrivent en axes, traites en boutons
            let event = match ev.event {
                EventType::AxisChanged(a, v, c) => match trigger_axis(a, c.into_u32()) {
                    Some(b) => EventType::ButtonChanged(b, v, c),
                    None => {
                        if a == Axis::Unknown && v.abs() > 0.5 && self.pad_logged.insert(c.into_u32()) {
                            log::info!("axe de manette sans correspondance : {c}");
                        }
                        ev.event
                    }
                },
                e => e,
            };
            match event {
                EventType::Connected => {
                    log_pad(g.gamepad(ev.id));
                    if self.pad.is_none() {
                        self.pad = Some(ev.id);
                    }
                }
                EventType::Disconnected => {
                    if self.pad == Some(ev.id) {
                        self.pad = g.gamepads().map(|(id, _)| id).find(|&id| id != ev.id);
                        self.pad_buttons.clear();
                    }
                }
                // gachette analogique : gilrs deduit aussi des appuis / relachements de ses valeurs, a d'autres
                // seuils ; les compter en plus doublerait les appuis (onglets qui defilent trop vite)
                EventType::ButtonPressed(b, _) | EventType::ButtonReleased(b, _) if self.analog_triggers.contains(&b) => {}
                EventType::ButtonPressed(b, code) => {
                    if b == Button::Unknown && self.pad_logged.insert(code.into_u32()) {
                        log::info!("bouton de manette sans correspondance : {code}");
                    }
                    self.pad = Some(ev.id);
                    self.pad_buttons.insert(b);
                    self.pad_pressed.iter_mut().for_each(|s| {
                        s.insert(b);
                    });
                    self.last_device = Device::Gamepad;
                }
                EventType::ButtonReleased(b, _) => {
                    if self.pad == Some(ev.id) {
                        self.pad_buttons.remove(&b);
                    }
                }
                // gachettes (tout ou rien) : certains pilotes n'envoient que des valeurs, pas d'appui
                EventType::ButtonChanged(b @ (Button::LeftTrigger2 | Button::RightTrigger2), v, _) => {
                    if v > 0.02 && v < 0.98 {
                        self.analog_triggers.insert(b);
                    }
                    if v > 0.5 && !self.pad_buttons.contains(&b) {
                        self.pad = Some(ev.id);
                        self.pad_buttons.insert(b);
                        self.pad_pressed.iter_mut().for_each(|s| {
                            s.insert(b);
                        });
                        self.last_device = Device::Gamepad;
                    } else if v < 0.3 && self.pad == Some(ev.id) {
                        self.pad_buttons.remove(&b);
                    }
                }
                EventType::AxisChanged(_, v, _) if v.abs() > 0.5 => {
                    self.pad = Some(ev.id);
                    self.last_device = Device::Gamepad;
                }
                _ => {}
            }
        }
    }

    pub(crate) fn set_phase_tick(&mut self, tick: bool) {
        self.phase = tick as usize;
    }

    /// Oublie les appuis vus par le pas de simulation qui vient de s'executer.
    pub(crate) fn consume_tick(&mut self) {
        self.keys_pressed[1].clear();
        self.wheel[1] = 0.0;
        self.mouse_pressed[1].clear();
        self.pad_pressed[1].clear();
        self.touch_started[1].clear();
    }

    /// Oublie les appuis vus par l'image qui vient d'etre affichee.
    pub(crate) fn end_frame(&mut self) {
        self.keys_pressed[0].clear();
        self.wheel[0] = 0.0;
        self.mouse_pressed[0].clear();
        self.pad_pressed[0].clear();
        self.touch_started[0].clear();
        self.typed.clear();
    }

    pub(crate) fn type_text(&mut self, text: &str) {
        self.typed.extend(text.chars().filter(|c| !c.is_control()));
    }

    pub fn key(&self, k: KeyCode) -> bool {
        self.keys.contains(&k)
    }
    pub fn key_pressed(&self, k: KeyCode) -> bool {
        self.keys_pressed[self.phase].contains(&k)
    }
    pub fn mouse(&self, b: MouseButton) -> bool {
        self.mouse.contains(&b)
    }
    pub fn mouse_pressed(&self, b: MouseButton) -> bool {
        self.mouse_pressed[self.phase].contains(&b)
    }
    pub fn pad(&self, b: Button) -> bool {
        self.pad_buttons.contains(&b)
    }
    pub fn pad_pressed(&self, b: Button) -> bool {
        self.pad_pressed[self.phase].contains(&b)
    }
    /// Premiere touche / bouton presse depuis la derniere lecture (reattribution des touches).
    pub fn any_key_pressed(&self) -> Option<KeyCode> {
        self.keys_pressed[self.phase].iter().next().copied()
    }
    pub fn any_mouse_pressed(&self) -> Option<MouseButton> {
        self.mouse_pressed[self.phase].iter().next().copied()
    }
    pub fn any_pad_pressed(&self) -> Option<Button> {
        self.pad_pressed[self.phase].iter().next().copied()
    }

    /// Caractere de la touche selon la disposition du clavier, si elle a deja ete pressee.
    pub fn key_label(&self, k: KeyCode) -> Option<&str> {
        self.key_labels.get(&k).map(|s| s.as_str())
    }

    pub(crate) fn learn_label(&mut self, code: KeyCode, text: &str) {
        let t = text.to_uppercase();
        if t.chars().count() == 1 && !t.chars().all(char::is_control) {
            self.key_labels.insert(code, t);
        }
    }

    pub fn has_pad(&self) -> bool {
        self.console || self.pad.is_some()
    }

    fn axis(&self, a: Axis) -> f32 {
        match (&self.gilrs, self.pad) {
            (Some(g), Some(id)) => g.connected_gamepad(id).map(|p| p.value(a)).unwrap_or(0.0),
            _ => 0.0,
        }
    }

    /// Stick (gauche ou droit) avec zone morte radiale ; y vers le bas.
    pub fn stick(&self, right: bool) -> Vec2 {
        let (ax, ay) = if right { (Axis::RightStickX, Axis::RightStickY) } else { (Axis::LeftStickX, Axis::LeftStickY) };
        let v = Vec2::new(self.axis(ax), -self.axis(ay));
        let l = v.length();
        const DEAD: f32 = 0.22;
        if l < DEAD {
            Vec2::ZERO
        } else {
            v / l * ((l - DEAD) / (1.0 - DEAD)).min(1.0)
        }
    }

}

/// Une touche, un bouton de souris ou de manette associe a une action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(Button),
}

/// Actions du jeu -> touches (lues dans `data/input.ron`, surchargees par les reglages).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Bindings {
    pub actions: std::collections::BTreeMap<String, Vec<Binding>>,
}

impl Bindings {
    fn list(&self, action: &str) -> &[Binding] {
        self.actions.get(action).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl Input {
    fn binding_down(&self, b: &Binding) -> bool {
        match *b {
            Binding::Key(k) => self.key(k),
            Binding::Mouse(m) => self.mouse(m),
            Binding::Pad(p) => self.pad(p),
        }
    }
    fn binding_pressed(&self, b: &Binding) -> bool {
        match *b {
            Binding::Key(k) => self.key_pressed(k),
            Binding::Mouse(m) => self.mouse_pressed(m),
            Binding::Pad(p) => self.pad_pressed(p),
        }
    }
    /// Action maintenue.
    pub fn action(&self, b: &Bindings, action: &str) -> bool {
        b.list(action).iter().any(|x| self.binding_down(x))
    }
    /// Action enfoncee depuis la derniere lecture.
    pub fn action_pressed(&self, b: &Bindings, action: &str) -> bool {
        b.list(action).iter().any(|x| self.binding_pressed(x))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_keys_become_pad_buttons() {
        let mut i = Input::new();
        i.console = true;
        i.key_event(KeyCode::F1, true, false);
        assert!(i.pad_pressed(Button::South) && i.pad(Button::South) && i.last_device == Device::Gamepad);
        assert!(!i.key(KeyCode::F1) && i.has_pad());
        i.key_event(KeyCode::F1, false, false);
        assert!(!i.pad(Button::South));
        i.key_event(KeyCode::KeyA, true, false);
        assert!(i.key(KeyCode::KeyA));
    }

    #[test]
    fn trigger_axes_become_buttons() {
        assert_eq!(trigger_axis(Axis::LeftZ, 0), Some(Button::LeftTrigger2));
        assert_eq!(trigger_axis(Axis::RightZ, 0), Some(Button::RightTrigger2));
        assert_eq!(trigger_axis(Axis::LeftStickX, 3 << 16), None);
        let linux = cfg!(target_os = "linux");
        assert_eq!(trigger_axis(Axis::Unknown, 3 << 16 | 0x0a), linux.then_some(Button::LeftTrigger2));
        assert_eq!(trigger_axis(Axis::Unknown, 3 << 16 | 0x09), linux.then_some(Button::RightTrigger2));
        assert_eq!(trigger_axis(Axis::Unknown, 3 << 16 | 0x10), None);
    }
}
