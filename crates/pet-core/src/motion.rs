use crate::desktop::{Desktop, Rect};
use glam::Vec2;
use std::collections::VecDeque;
pub const GRAVITY: f32 = 1800.;
pub const MAX_THROW_SPEED: f32 = 2600.;
const SAMPLE_WINDOW: f64 = 0.12;
#[derive(Default, Debug)]
pub struct Motion {
    pub position: Vec2,
    pub velocity: Vec2,
    pub active: bool,
    pub low_gravity: bool,
    samples: VecDeque<(Vec2, f64)>,
}
#[derive(Default, Debug)]
pub struct Impact {
    pub landed: bool,
    pub settled: bool,
    pub speed: f32,
}
impl Motion {
    pub fn begin_drag(&mut self, p: Vec2, t: f64) {
        self.stop(p);
        self.samples.clear();
        self.record_drag(p, t);
    }
    pub fn record_drag(&mut self, p: Vec2, t: f64) {
        if !p.is_finite() || !t.is_finite() {
            return;
        }
        self.position = p;
        if self.samples.back().is_some_and(|s| t <= s.1) {
            return;
        }
        self.samples.push_back((p, t));
        while self.samples.len() > 1 && t - self.samples[0].1 > SAMPLE_WINDOW {
            self.samples.pop_front();
        }
    }
    pub fn release_velocity(&self, t: f64) -> Vec2 {
        if self.samples.len() < 2 {
            return Vec2::ZERO;
        }
        let (first, tf) = self.samples[0];
        let (last, tl) = *self.samples.back().unwrap();
        if !t.is_finite() || t - tl > SAMPLE_WINDOW || tl - tf < 0.008 {
            return Vec2::ZERO;
        }
        ((last - first) / (tl - tf) as f32).clamp_length_max(MAX_THROW_SPEED)
    }
    pub fn launch(&mut self, p: Vec2, v: Vec2) {
        if !p.is_finite() || !v.is_finite() {
            return;
        }
        self.position = p;
        self.velocity = v.clamp_length_max(MAX_THROW_SPEED);
        self.active = true;
        self.samples.clear();
    }
    pub fn stop(&mut self, p: Vec2) {
        self.position = p;
        self.velocity = Vec2::ZERO;
        self.active = false;
    }
    pub fn advance(&mut self, dt: f32, mut bounds: Rect, desktop: Option<&Desktop>) -> Impact {
        let mut result = Impact::default();
        if !self.active || !dt.is_finite() || dt <= 0. {
            return result;
        }
        let rebound = if self.low_gravity { 0.86 } else { 0.48 };
        let floor_rebound = if self.low_gravity { 0.82 } else { 0.32 };
        let gravity = GRAVITY * if self.low_gravity { 0.18 } else { 1. };
        let mut remaining = dt.min(0.1);
        while remaining > 0.000001 {
            if let Some(d) = desktop {
                bounds = d.bounds_for(self.position);
                if let Some(s) = d.support(self.position) {
                    bounds.max.y = s.y - d.extent.y;
                }
            }
            let step = remaining.min(1. / 120.);
            remaining -= step;
            if self.position.y >= bounds.max.y - 0.01 && self.velocity.y.abs() < 0.00001 {
                self.position.y = bounds.max.y;
                self.velocity.y = 0.;
                self.velocity.x = self.velocity.x.signum()
                    * (self.velocity.x.abs()
                        - (if self.low_gravity { 180. } else { 1250. }) * step)
                        .max(0.);
            } else {
                self.velocity.y += gravity * step;
                self.velocity.x *= (-(if self.low_gravity { 0.045 } else { 0.25 }) * step).exp();
            }
            let previous = self.position;
            self.position += self.velocity * step;
            if let Some(d) = desktop {
                bounds = d.bounds_for(self.position);
                if self.velocity.y >= 0.
                    && let Some(s) = d.crossed_surface(previous, self.position)
                {
                    bounds.max.y = s.y - d.extent.y;
                    // Window tops catch gently instead of bouncing through a moving frame.
                    self.position.y = bounds.max.y;
                    result.landed = true;
                    result.speed = result.speed.max(self.velocity.y);
                    self.velocity.y = 0.;
                }
            }
            if self.position.x < bounds.min.x {
                self.position.x = bounds.min.x;
                if self.velocity.x < 0. {
                    self.velocity.x *= -rebound;
                }
            } else if self.position.x > bounds.max.x {
                self.position.x = bounds.max.x;
                if self.velocity.x > 0. {
                    self.velocity.x *= -rebound;
                }
            }
            if self.position.y < bounds.min.y {
                self.position.y = bounds.min.y;
                if self.velocity.y < 0. {
                    self.velocity.y *= -rebound;
                }
            }
            if self.position.y >= bounds.max.y {
                self.position.y = bounds.max.y;
                if self.velocity.y > 0. {
                    result.landed = true;
                    result.speed = result.speed.max(self.velocity.y);
                    self.velocity.y =
                        if self.velocity.y > (if self.low_gravity { 65. } else { 180. }) {
                            -self.velocity.y * floor_rebound
                        } else {
                            0.
                        };
                }
            }
            if self.position.y >= bounds.max.y - 0.01
                && self.velocity.y.abs() < 1.
                && self.velocity.x.abs() < 8.
            {
                self.velocity = Vec2::ZERO;
                self.active = false;
                result.settled = true;
                break;
            }
        }
        result
    }
}

#[cfg(test)]
mod mode_tests {
    use super::*;
    #[test]
    fn low_gravity_preserves_more_airtime_and_rebounds_without_escaping() {
        let bounds = Rect::new(0., 0., 1000., 700.);
        let simulate = |low| {
            let mut motion = Motion {
                low_gravity: low,
                ..Default::default()
            };
            motion.launch(Vec2::new(500., 300.), Vec2::new(1800., -1200.));
            let mut contacts = 0;
            let mut moving = 0;
            for _ in 0..1200 {
                let impact = motion.advance(1. / 120., bounds, None);
                contacts += usize::from(impact.landed);
                moving += usize::from(motion.active);
                assert!(
                    motion.position.cmpge(bounds.min).all()
                        && motion.position.cmple(bounds.max).all()
                );
            }
            (contacts, moving)
        };
        let normal = simulate(false);
        let low = simulate(true);
        assert!(low.1 > normal.1 * 2, "normal {normal:?}, low {low:?}");
        assert!(low.0 >= 2);
    }
}
