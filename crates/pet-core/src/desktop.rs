use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}
impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            max: Vec2::new(x + w, y + h),
        }
    }
    pub fn contains(self, p: Vec2) -> bool {
        p.cmpge(self.min).all() && p.cmplt(self.max).all()
    }
    pub fn clamp(self, p: Vec2) -> Vec2 {
        p.clamp(self.min, self.max.max(self.min))
    }
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }
}
#[derive(Clone, Debug)]
pub struct Desktop {
    pub areas: Vec<Rect>,
    pub extent: Vec2,
    /// Transparent pixels above the visible avatar, in physical desktop pixels.
    pub top_inset: f32,
    pub surfaces: Vec<Surface>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub id: isize,
    pub window_left: f32,
    pub left: f32,
    pub right: f32,
    pub y: f32,
}
impl Default for Desktop {
    fn default() -> Self {
        Self {
            areas: vec![Rect::new(0., 0., 1920., 1080.)],
            extent: Vec2::new(320., 248.),
            top_inset: 0.,
            surfaces: vec![],
        }
    }
}
impl Desktop {
    pub fn screen_at(&self, p: Vec2) -> usize {
        if let Some(i) = self.areas.iter().position(|r| r.contains(p)) {
            return i;
        }
        self.areas
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                p.distance_squared(a.clamp(p))
                    .total_cmp(&p.distance_squared(b.clamp(p)))
            })
            .map_or(0, |(i, _)| i)
    }
    pub fn current(&self, p: Vec2) -> usize {
        self.screen_at(p + Vec2::new(self.extent.x * 0.5, self.extent.y - 1.))
    }
    pub fn bounds_for(&self, p: Vec2) -> Rect {
        let Some(area) = self.areas.get(self.current(p)).copied() else {
            return Rect::new(0., 0., 1600., 800.);
        };
        let mut low = area.min;
        low.y -= self.top_inset;
        let mut high = (area.max - self.extent).max(low);
        for other in &self.areas {
            if p.y >= other.min.y && p.y + self.extent.y <= other.max.y + 0.01 {
                if (area.max.x - other.min.x).abs() < 0.5 {
                    high.x = high.x.max(other.max.x - self.extent.x);
                }
                if (area.min.x - other.max.x).abs() < 0.5 {
                    low.x = low.x.min(other.min.x);
                }
            }
            if p.x >= other.min.x && p.x + self.extent.x <= other.max.x {
                if (area.max.y - other.min.y).abs() < 0.5 {
                    high.y = high.y.max(other.max.y - self.extent.y);
                }
                if (area.min.y - other.max.y).abs() < 0.5 {
                    low.y = low.y.min(other.min.y);
                }
            }
        }
        Rect {
            min: low,
            max: high.max(low),
        }
    }
    pub fn clamp_origin(&self, p: Vec2) -> Vec2 {
        let area = self
            .areas
            .get(self.current(p))
            .copied()
            .unwrap_or(Rect::new(0., 0., 1920., 1080.));
        let anchor = p + Vec2::new(self.extent.x * 0.5, self.extent.y - 1.);
        if area.contains(anchor) {
            self.bounds_for(p).clamp(p)
        } else {
            Rect {
                min: area.min - Vec2::new(0., self.top_inset),
                max: (area.max - self.extent).max(area.min),
            }
            .clamp(p)
        }
    }
    /// Only descending feet can land on a window; moving upward passes through.
    pub fn crossed_surface(&self, previous: Vec2, next: Vec2) -> Option<Surface> {
        let old_feet = previous.y + self.extent.y;
        let feet = next.y + self.extent.y;
        if feet < old_feet {
            return None;
        }
        self.surfaces
            .iter()
            .copied()
            .filter(|s| {
                let t = if feet > old_feet {
                    ((s.y - old_feet) / (feet - old_feet)).clamp(0., 1.)
                } else {
                    1.
                };
                let x = previous.x + (next.x - previous.x) * t + self.extent.x * 0.5;
                old_feet <= s.y + 0.5
                    && feet >= s.y
                    && x >= s.left
                    && x <= s.right
                    && s.y - self.extent.y >= self.bounds_for(next).min.y
            })
            .min_by(|a, b| a.y.total_cmp(&b.y))
    }
    pub fn support(&self, p: Vec2) -> Option<Surface> {
        let x = p.x + self.extent.x * 0.5;
        self.surfaces
            .iter()
            .copied()
            .find(|s| (p.y + self.extent.y - s.y).abs() < 1. && x >= s.left && x <= s.right)
    }
    pub fn place_on(&self, index: usize) -> Vec2 {
        let a = self.areas.get(index).copied().unwrap_or(self.areas[0]);
        Rect {
            min: a.min,
            max: (a.max - self.extent).max(a.min),
        }
        .clamp(a.max - self.extent - Vec2::new(40., 0.))
    }
}
