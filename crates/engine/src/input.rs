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
    pad_buttons: HashSet<Button>,
    pad_pressed: [HashSet<Button>; 2],
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
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Input {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                log::warn!("manettes indisponibles : {e}");
                None
            }
        };
        let pad = gilrs.as_ref().and_then(|g| g.gamepads().next().map(|(id, _)| id));
        Input {
            keys: HashSet::new(),
            keys_pressed: Default::default(),
            mouse: HashSet::new(),
            mouse_pressed: Default::default(),
            mouse_px: Vec2::ZERO,
            pad_buttons: HashSet::new(),
            pad_pressed: Default::default(),
            phase: 0,
            gilrs,
            pad,
            last_device: Device::Keyboard,
            focused: true,
            key_labels: HashMap::new(),
            typed: Vec::new(),
        }
    }

    pub(crate) fn key_event(&mut self, code: KeyCode, down: bool, repeat: bool) {
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

    pub(crate) fn mouse_moved(&mut self, p: Vec2) {
        if (p - self.mouse_px).length_squared() > 4.0 {
            self.last_device = Device::Mouse;
        }
        self.mouse_px = p;
    }

    pub(crate) fn focus_lost(&mut self) {
        self.keys.clear();
        self.mouse.clear();
        self.focused = false;
    }

    /// Lit les evenements manette (une fois par image).
    pub(crate) fn poll_gamepad(&mut self) {
        let Some(g) = self.gilrs.as_mut() else { return };
        while let Some(ev) = g.next_event() {
            match ev.event {
                EventType::Connected => {
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
                EventType::ButtonPressed(b, _) => {
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
                EventType::AxisChanged(_, v, _) => {
                    if v.abs() > 0.5 {
                        self.pad = Some(ev.id);
                        self.last_device = Device::Gamepad;
                    }
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
        self.mouse_pressed[1].clear();
        self.pad_pressed[1].clear();
    }

    /// Oublie les appuis vus par l'image qui vient d'etre affichee.
    pub(crate) fn end_frame(&mut self) {
        self.keys_pressed[0].clear();
        self.mouse_pressed[0].clear();
        self.pad_pressed[0].clear();
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
        self.pad.is_some()
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

    /// Gachettes analogiques (0..1).
    pub fn trigger(&self, right: bool) -> f32 {
        let b = if right { Button::RightTrigger2 } else { Button::LeftTrigger2 };
        match (&self.gilrs, self.pad) {
            (Some(g), Some(id)) => g
                .connected_gamepad(id)
                .and_then(|p| p.button_data(b).map(|d| d.value()))
                .unwrap_or(0.0),
            _ => 0.0,
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
            Binding::Pad(Button::LeftTrigger2) => self.trigger(false) > 0.35 || self.pad(Button::LeftTrigger2),
            Binding::Pad(Button::RightTrigger2) => self.trigger(true) > 0.35 || self.pad(Button::RightTrigger2),
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
