//! Damped suspension response in screen space, independent of the renderer.
use glam::Vec2;

/// Tail pitch relative to its animated rest pose (+Y is up, tail points toward -Z).
#[derive(Clone, Debug, Default)]
pub struct TailResponse {
    pub pitch: f32,
    velocity: f32,
    unsupported: bool,
    contact_remaining: f32,
}
impl TailResponse {
    /// A physical impact can occur while the animation remains Airborne across bounces.
    pub fn impact(&mut self, speed: f32) {
        if !speed.is_finite() {
            return;
        }
        let strength = (speed / 1500.).clamp(0., 1.);
        self.velocity = -3.2 - strength * 1.8;
        self.contact_remaining = 0.09 + strength * 0.035;
    }
    pub fn contact_weight(&self) -> f32 {
        (self.contact_remaining / 0.055).clamp(0., 1.)
    }
    pub fn cancel_contact(&mut self) {
        self.contact_remaining = 0.;
        self.velocity = 0.;
    }
    pub fn advance(&mut self, dt: f32, unsupported: bool, speed: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        if dt > 0.25 {
            self.pitch = if unsupported { 0.10 } else { -0.27 };
            self.velocity = 0.;
            self.contact_remaining = 0.;
            self.unsupported = unsupported;
            return;
        }
        if self.unsupported && !unsupported && self.contact_remaining <= 0. {
            self.impact(speed);
        }
        self.unsupported = unsupported;
        let steps = (dt * 120.).ceil().max(1.) as usize;
        let step = dt / steps as f32;
        for _ in 0..steps {
            let contact = self.contact_weight();
            let free = if unsupported { 0.10 } else { -0.27 };
            let target = free + (-0.35 - free) * contact;
            self.velocity += ((target - self.pitch) * (260. + contact * 200.)
                - self.velocity * (23. + contact * 3.))
                * step;
            self.pitch = (self.pitch + self.velocity * step).clamp(-0.42, 0.18);
            if self.pitch <= -0.42 {
                self.velocity = self.velocity.max(0.);
            }
            self.contact_remaining = (self.contact_remaining - step).max(0.);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Suspension {
    pub weight: f32,
    pub roll: f32,
    pub stretch: f32,
    pub lag: Vec2,
    pub speed: f32,
    roll_velocity: f32,
    stretch_velocity: f32,
    previous_position: Option<Vec2>,
    velocity: Vec2,
}
impl Suspension {
    /// Positions are in unscaled desktop pixels; positive Y points down.
    pub fn advance(&mut self, dt: f32, position: Vec2, held: bool, airborne: bool) {
        if !dt.is_finite() || dt <= 0. || !position.is_finite() {
            return;
        }
        if dt > 0.25 {
            *self = Self::default();
            self.previous_position = Some(position);
            return;
        }
        let previous = self.previous_position.replace(position).unwrap_or(position);
        let measured = ((position - previous) / dt).clamp_length_max(2200.);
        let filtered = self.velocity.lerp(measured, 1. - (-dt * 18.).exp());
        let acceleration = ((filtered - self.velocity) / dt).clamp_length_max(9000.);
        self.velocity = filtered;
        self.speed = filtered.length();
        let target_weight = if held {
            1.
        } else if airborne {
            0.65
        } else {
            0.
        };
        self.weight += (target_weight - self.weight) * (1. - (-dt * 10.).exp());
        // A body lags behind the hand; changing direction excites a damped pendulum.
        let target_roll = if held || airborne {
            (-filtered.x * 0.00018 - acceleration.x * 0.000025).clamp(-0.23, 0.23)
        } else {
            0.
        };
        let target_stretch = if held {
            (0.045 - filtered.y * 0.000035 - acceleration.y * 0.000004).clamp(0.015, 0.10)
        } else if airborne {
            0.018
        } else {
            0.
        };
        let steps = (dt * 120.).ceil().max(1.) as usize;
        let step = dt / steps as f32;
        for _ in 0..steps {
            self.roll_velocity +=
                ((target_roll - self.roll) * 65. - self.roll_velocity * 11.) * step;
            self.roll = (self.roll + self.roll_velocity * step).clamp(-0.3, 0.3);
            self.stretch_velocity +=
                ((target_stretch - self.stretch) * 85. - self.stretch_velocity * 13.) * step;
            self.stretch = (self.stretch + self.stretch_velocity * step).clamp(-0.04, 0.12);
        }
        let target_lag = if held || airborne {
            -filtered * 0.000045
        } else {
            Vec2::ZERO
        };
        self.lag = self
            .lag
            .lerp(target_lag.clamp_length_max(0.065), 1. - (-dt * 8.).exp());
    }
    /// Local foot motion: front/rear and left/right use different phases.
    pub fn foot(&self, clock: f32, rear: bool, side: f32, direction: f32) -> (f32, f32, f32) {
        // Keep phase continuous when the hand accelerates; speed changes amplitude only.
        let phase = clock * 6.5 + if rear { std::f32::consts::PI } else { 0. } + side * 0.7;
        let kick = phase.sin();
        let activity = 0.10 + (self.speed / 1800.).min(0.16);
        let pitch = (0.18 + kick * activity + self.roll * direction * 0.8) * self.weight;
        let drop = (-0.055 + kick.max(0.) * 0.055) * self.weight;
        let reach = (kick * 0.075 - self.lag.x * direction * 1.3) * self.weight;
        (pitch, drop, reach)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_physical_bounce_drops_the_tail_without_an_animation_state_change() {
        use crate::{desktop::Rect, motion::Motion};
        for hz in [30., 60., 144.] {
            let mut motion = Motion::default();
            motion.launch(Vec2::new(100., 10.), Vec2::new(0., 1200.));
            let mut tail = TailResponse::default();
            for _ in 0..hz as usize {
                tail.advance(1. / hz, true, 0.);
            }
            let mut contacts = 0;
            let mut before = 0.;
            let mut since_contact = None;
            let mut checks = 0;
            for _ in 0..(hz * 3.) as usize {
                let impact = motion.advance(1. / hz, Rect::new(0., 0., 800., 450.), None);
                if impact.landed {
                    contacts += 1;
                    before = tail.pitch;
                    since_contact = Some(0.);
                    tail.impact(impact.speed);
                }
                // Deliberately stay Airborne, just like the desktop throughout rebound motion.
                tail.advance(1. / hz, true, 0.);
                if let Some(age) = &mut since_contact {
                    *age += 1. / hz;
                    if *age >= 0.05 {
                        assert!(
                            tail.pitch < before - 0.07,
                            "contact {contacts} at {hz} FPS: {} -> {}",
                            before,
                            tail.pitch
                        );
                        checks += 1;
                        since_contact = None;
                    }
                }
            }
            assert!(
                contacts >= 3 && checks == contacts,
                "all rebounds must respond"
            );
            for _ in 0..hz as usize {
                tail.advance(1. / hz, false, 0.);
            }
            assert!((tail.pitch + 0.27).abs() < 0.001);
        }
    }
    #[test]
    fn tail_drops_promptly_after_landing_then_settles_at_different_frame_rates() {
        for hz in [30., 60., 144.] {
            let mut tail = TailResponse::default();
            for _ in 0..hz as usize {
                tail.advance(1. / hz, true, 900.);
            }
            assert!(tail.pitch > 0.08);
            for _ in 0..(hz * 0.15).ceil() as usize {
                tail.advance(1. / hz, false, 900.);
            }
            assert!(
                tail.pitch < -0.20,
                "must drop within 150 ms: {}",
                tail.pitch
            );
            for _ in 0..hz as usize {
                tail.advance(1. / hz, false, 0.);
            }
            assert!((tail.pitch + 0.27).abs() < 0.001);
            tail.advance(2., true, 10000.);
            assert_eq!(tail.pitch, 0.10);
        }
    }
    #[test]
    fn suspension_tracks_lift_reversal_and_settles_after_landing() {
        let mut response = Suspension::default();
        let mut p = Vec2::ZERO;
        for _ in 0..60 {
            p += Vec2::new(4., -2.);
            response.advance(1. / 60., p, true, false);
        }
        assert!(response.roll < -0.02 && response.stretch > 0.04);
        let front = response.foot(0.5, false, 1., 1.);
        let rear = response.foot(0.5, true, 1., 1.);
        assert!((front.0 - rear.0).abs() > 0.05);
        for _ in 0..60 {
            p.x -= 5.;
            response.advance(1. / 60., p, true, false);
        }
        assert!(response.roll > 0.02);
        for _ in 0..240 {
            response.advance(1. / 60., p, false, false);
        }
        assert!(response.roll.abs() < 0.0001);
        assert!(response.stretch.abs() < 0.0001 && response.weight < 0.0001);
        response.advance(2., p + Vec2::splat(10000.), true, false);
        assert_eq!(response.roll, 0.);
    }
}
