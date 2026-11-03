use anyhow::{Result, bail};
use glam::{Affine3A, Mat3, Vec3};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Definition {
    pub node: String,
    pub profile_mesh: String,
    pub anchors: Vec<String>,
    pub axis: [f32; 3],
    pub start: f32,
    pub length: f32,
    pub sections: usize,
    pub support_height: f32,
    pub floor_height: f32,
    pub compression: f32,
    pub gravity: f32,
    pub stiffness: f32,
    pub damping: f32,
}
impl Default for Definition {
    fn default() -> Self {
        Self {
            node: String::new(),
            profile_mesh: String::new(),
            anchors: vec![],
            axis: [0., 0., -1.],
            start: 0.,
            length: 2.3,
            sections: 28,
            support_height: -0.82,
            floor_height: -1.01,
            compression: 0.18,
            gravity: 9.8,
            stiffness: 42.,
            damping: 10.,
        }
    }
}
impl Definition {
    pub fn validate(&self) -> Result<()> {
        for n in [&self.node, &self.profile_mesh] {
            if n.len() > 256 || !crate::assets::safe_relative(n) {
                bail!("软体 node 需要模型内部名称或相对节点路径")
            }
        }
        if self.anchors.len() > 32
            || self
                .anchors
                .iter()
                .any(|s| s.len() > 256 || !crate::assets::safe_relative(s))
        {
            bail!("软体 anchors 最多 32 个承托节点")
        }
        let axis = Vec3::from(self.axis);
        if !axis.is_finite()
            || axis.length() < 0.0001
            || axis.normalize().y.abs() > 0.01
            || !(4..=48).contains(&self.sections)
        {
            bail!("软体 axis 或 sections 无效")
        }
        for (n, lo, hi) in [
            (self.start, -10000., 10000.),
            (self.length, 0.001, 10000.),
            (self.support_height, -10000., 10000.),
            (self.floor_height, -10000., 10000.),
            (self.compression, 0., 0.5),
            (self.gravity, 0., 9.8),
            (self.stiffness, 1., 200.),
            (self.damping, 0.5, 40.),
        ] {
            if !n.is_finite() || n < lo || n > hi {
                bail!("软体参数超出允许范围")
            }
        }
        if self.floor_height > self.support_height {
            bail!("软体 floor_height 不能高于 support_height")
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct SoftBody {
    pub def: Definition,
    pub lower: Vec<f32>,
    pub upper: Vec<f32>,
    pub mesh_lower: Vec<f32>,
    pub load: f32,
    pub sway: Vec3,
    /// 0 = normal support, 1 = hanging from a hand rather than resting on feet.
    pub suspension: f32,
    load_velocity: f32,
    sway_velocity: Vec3,
    accumulator: f32,
    pub transforms: Vec<Affine3A>,
}
fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
impl SoftBody {
    pub fn sample(def: Definition, vertices: &[Vec3]) -> Result<Self> {
        def.validate()?;
        if vertices.is_empty() {
            bail!("软体需要有效的轮廓参考网格")
        }
        let n = def.sections;
        let axis = Vec3::from(def.axis).normalize();
        let mut rings: Vec<(f32, f32, f32)> = vertices
            .iter()
            .map(|p| ((p.dot(axis) * 10000.).round(), p.y, p.y))
            .collect();
        rings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f32, f32, f32)> = vec![];
        for r in rings {
            if let Some(last) = merged.last_mut()
                && last.0 == r.0
            {
                last.1 = last.1.min(r.1);
                last.2 = last.2.max(r.2);
                continue;
            }
            merged.push(r)
        }
        let mut lower = vec![0.; n];
        let mut upper = vec![0.; n];
        for i in 0..n {
            let at = (def.start + def.length * i as f32 / (n - 1) as f32) * 10000.;
            let index = merged.partition_point(|r| r.0 < at).saturating_sub(1);
            let a = merged[index];
            let b = merged[(index + 1).min(merged.len() - 1)];
            let t = ((at - a.0) / (b.0 - a.0).max(1.)).clamp(0., 1.);
            lower[i] = a.1 + (b.1 - a.1) * t;
            upper[i] = a.2 + (b.2 - a.2) * t
        }
        let load = def.gravity / 9.8;
        let mut s = Self {
            def,
            mesh_lower: lower.clone(),
            lower,
            upper,
            load,
            sway: Vec3::ZERO,
            suspension: 0.,
            load_velocity: 0.,
            sway_velocity: Vec3::ZERO,
            accumulator: 0.,
            transforms: vec![Affine3A::IDENTITY; n],
        };
        s.rebuild(true, 0.);
        Ok(s)
    }
    pub fn coordinate(&self, p: Vec3) -> f32 {
        ((p.dot(Vec3::from(self.def.axis).normalize()) - self.def.start) / self.def.length
            * (self.def.sections - 1) as f32)
            .clamp(0., (self.def.sections - 1) as f32)
    }
    pub fn include_vertex(&mut self, p: Vec3) {
        let i = self.coordinate(p).floor() as usize;
        let j = (i + 1).min(self.def.sections - 1);
        self.mesh_lower[i] = self.mesh_lower[i].min(p.y);
        self.mesh_lower[j] = self.mesh_lower[j].min(p.y)
    }
    pub fn reset(&mut self) {
        self.load = self.def.gravity / 9.8;
        self.load_velocity = 0.;
        self.sway = Vec3::ZERO;
        self.sway_velocity = Vec3::ZERO;
        self.accumulator = 0.
    }
    pub fn advance(
        &mut self,
        dt: f32,
        acceleration: Vec3,
        local_acceleration: Vec3,
        desktop_gravity: f32,
        grounded: bool,
        flex: f32,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        if dt > 0.25 {
            self.reset();
            self.rebuild(grounded, flex);
            return;
        }
        let target = self.def.gravity / 9.8
            * (1. - acceleration.y / desktop_gravity.max(0.001)).clamp(0., 1.12)
            * (1. - self.suspension.clamp(0., 1.) * 0.72);
        self.accumulator += dt;
        let step = 1. / 120.;
        while self.accumulator >= step {
            self.load_velocity += ((target - self.load) * self.def.stiffness
                - self.load_velocity * self.def.damping)
                * step;
            self.load = (self.load + self.load_velocity * step).clamp(0., 1.2);
            self.sway_velocity += (-local_acceleration * 0.10
                - self.sway * self.def.stiffness
                - self.sway_velocity * self.def.damping)
                * step;
            self.sway =
                (self.sway + self.sway_velocity * step).clamp_length_max(self.def.length * 0.06);
            self.accumulator -= step
        }
        self.rebuild(grounded, flex)
    }
    pub fn rebuild(&mut self, grounded: bool, flex: f32) {
        let axis = Vec3::from(self.def.axis).normalize();
        let side = axis.cross(Vec3::Y).normalize();
        for i in 0..self.def.sections {
            let t = i as f32 / (self.def.sections - 1) as f32;
            let influence = smooth(0., 0.55, t);
            let soft = smooth(0., 1., t);
            let squash = 1. - self.def.compression * soft * self.load;
            let bottom = self.lower[i]
                + (self.lower[i].min(self.def.support_height) - self.lower[i])
                    * influence
                    * self.load;
            let mut shift = bottom - self.lower[i] * squash + self.sway.y * soft
                - self.def.length * 0.045 * soft * self.suspension;
            if grounded {
                shift = shift.max(self.def.floor_height - self.mesh_lower[i] * squash)
            }
            let spread = 1. + (1. - squash) * 0.35;
            let lean = 0.12 * soft * self.load;
            let basis = Mat3::from_cols(
                Vec3::X + side * side.x * (spread - 1.),
                Vec3::Y * squash + axis * lean,
                Vec3::Z + side * side.z * (spread - 1.),
            );
            let offset = Vec3::Y * shift - axis * lean * self.lower[i]
                + Vec3::new(self.sway.x, 0., self.sway.z) * soft
                + Vec3::Y * (t * std::f32::consts::PI).sin() * flex.max(0.) * 0.12
                - axis * soft * flex * 0.11;
            self.transforms[i] = Affine3A::from_mat3_translation(basis, offset)
        }
    }
    pub fn deform_point(&self, p: Vec3) -> Vec3 {
        let t = self.coordinate(p);
        let i = t.floor() as usize;
        let j = (i + 1).min(self.def.sections - 1);
        self.transforms[i]
            .transform_point3(p)
            .lerp(self.transforms[j].transform_point3(p), t - i as f32)
    }
}
