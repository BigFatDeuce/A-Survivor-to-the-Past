//! Effet « lo-fi » (kira) : imite la sortie sonore des consoles portables.
//! Echantillonnage bloque (frequence reduite), resolution reduite, mono et passe-haut (petit haut-parleur).
use kira::effect::{Effect, EffectBuilder};
use kira::info::Info;
use kira::Frame;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Reglages partages avec le fil audio (lus a chaque bloc, sans verrou).
#[derive(Default)]
pub struct LoFiParams {
    /// Frequence d'echantillonnage simulee (Hz), 0 = desactive.
    hold_hz: AtomicU32,
    /// Resolution (bits), 0 = desactive.
    bits: AtomicU32,
    /// 1 = mono.
    mono: AtomicU32,
    /// Coupure du passe-haut (Hz), 0 = desactive.
    highpass_hz: AtomicU32,
}

impl LoFiParams {
    pub fn set(&self, hold_hz: f32, bits: f32, mono: bool, highpass_hz: f32) {
        self.hold_hz.store(hold_hz.to_bits(), Ordering::Relaxed);
        self.bits.store(bits.to_bits(), Ordering::Relaxed);
        self.mono.store(mono as u32, Ordering::Relaxed);
        self.highpass_hz.store(highpass_hz.to_bits(), Ordering::Relaxed);
    }
}

pub struct LoFiBuilder(pub Arc<LoFiParams>);

impl EffectBuilder for LoFiBuilder {
    type Handle = Arc<LoFiParams>;

    fn build(self) -> (Box<dyn Effect>, Self::Handle) {
        let fx = LoFi { params: self.0.clone(), phase: 0.0, held: Frame::ZERO, hp_x: Frame::ZERO, hp_y: Frame::ZERO };
        (Box::new(fx), self.0)
    }
}

struct LoFi {
    params: Arc<LoFiParams>,
    phase: f32,
    held: Frame,
    hp_x: Frame,
    hp_y: Frame,
}

impl Effect for LoFi {
    fn process(&mut self, input: &mut [Frame], dt: f64, _info: &Info) {
        let hold = f32::from_bits(self.params.hold_hz.load(Ordering::Relaxed));
        let bits = f32::from_bits(self.params.bits.load(Ordering::Relaxed));
        let mono = self.params.mono.load(Ordering::Relaxed) != 0;
        let hp = f32::from_bits(self.params.highpass_hz.load(Ordering::Relaxed));
        if hold <= 0.0 && bits <= 0.0 && !mono && hp <= 0.0 {
            return;
        }
        let dt = dt as f32;
        let q = if bits > 0.0 { 2f32.powf(bits - 1.0) } else { 0.0 };
        let a = if hp > 0.0 {
            let rc = 1.0 / (std::f32::consts::TAU * hp);
            rc / (rc + dt)
        } else {
            0.0
        };
        for f in input.iter_mut() {
            let mut x = *f;
            if hold > 0.0 {
                self.phase += hold * dt;
                if self.phase >= 1.0 {
                    self.phase -= self.phase.floor();
                    self.held = x;
                }
                x = self.held;
            }
            if q > 0.0 {
                x.left = (x.left * q).round() / q;
                x.right = (x.right * q).round() / q;
            }
            if mono {
                let m = (x.left + x.right) * 0.5;
                x = Frame { left: m, right: m };
            }
            if a > 0.0 {
                let y = Frame {
                    left: a * (self.hp_y.left + x.left - self.hp_x.left),
                    right: a * (self.hp_y.right + x.right - self.hp_x.right),
                };
                self.hp_x = x;
                self.hp_y = y;
                x = y;
            }
            *f = x;
        }
    }
}
