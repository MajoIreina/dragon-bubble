use rand::Rng;
use serde::{Deserialize, Serialize};
/// Seconds per activity; a little variation keeps autonomous motion natural.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorOptions {
    pub idle: f32,
    pub walk: f32,
    pub run: f32,
    pub sleep: f32,
    pub spin_interval: f32,
    pub idle_spin: bool,
    pub window_perching: bool,
}
impl Default for BehaviorOptions {
    fn default() -> Self {
        Self {
            idle: 12.,
            walk: 4.,
            run: 2.,
            sleep: 30.,
            spin_interval: 8.,
            idle_spin: true,
            window_perching: true,
        }
    }
}
impl BehaviorOptions {
    pub fn from_value(v: &serde_json::Value) -> Self {
        let mut result = Self::default();
        for (key, value, min, max) in [
            ("idle", &mut result.idle, 3., 120.),
            ("walk", &mut result.walk, 1., 30.),
            ("run", &mut result.run, 0.5, 15.),
            ("sleep", &mut result.sleep, 5., 600.),
            ("spin_interval", &mut result.spin_interval, 4., 60.),
        ] {
            if let Some(n) = v[key].as_f64().filter(|n| n.is_finite()) {
                *value = n.clamp(min, max) as f32;
            }
        }
        if let Some(b) = v["idle_spin"].as_bool() {
            result.idle_spin = b;
        }
        if let Some(b) = v["window_perching"].as_bool() {
            result.window_perching = b;
        }
        result
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Idle,
    Walk,
    Run,
    Sleep,
    Stretch,
    Petted,
    Dragged,
    Airborne,
    Fall,
}
impl State {
    pub const ALL: [Self; 9] = [
        Self::Idle,
        Self::Walk,
        Self::Run,
        Self::Sleep,
        Self::Stretch,
        Self::Petted,
        Self::Dragged,
        Self::Airborne,
        Self::Fall,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Walk => "walk",
            Self::Run => "run",
            Self::Sleep => "sleep",
            Self::Stretch => "stretch",
            Self::Petted => "petted",
            Self::Dragged => "dragged",
            Self::Airborne => "airborne",
            Self::Fall => "fall",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "待机",
            Self::Walk => "走路",
            Self::Run => "跑步",
            Self::Sleep => "睡眠",
            Self::Stretch => "站起扒屏",
            Self::Petted => "摸摸回应",
            Self::Dragged => "提起",
            Self::Airborne => "空中 / 下落",
            Self::Fall => "落地",
        }
    }
    pub fn moving(self) -> bool {
        matches!(self, Self::Walk | Self::Run)
    }
    pub fn looping(self) -> bool {
        !matches!(self, Self::Petted | Self::Fall)
    }
    pub fn speed(self) -> f32 {
        match self {
            Self::Walk => 42.,
            Self::Run => 112.,
            _ => 0.,
        }
    }
}
#[derive(Debug)]
pub struct Brain {
    pub state: State,
    pub direction: i32,
    pub roaming: bool,
    pub remaining: f32,
    pub options: BehaviorOptions,
    pub forced_sleep: bool,
    pub inactive: f32,
}
impl Default for Brain {
    fn default() -> Self {
        Self {
            state: State::Idle,
            direction: 1,
            roaming: true,
            remaining: 4.,
            options: Default::default(),
            forced_sleep: false,
            inactive: 0.,
        }
    }
}
impl Brain {
    pub fn change(&mut self, state: State, seconds: f32) {
        self.state = state;
        self.remaining = seconds;
    }
    pub fn sleep_now(&mut self) {
        self.forced_sleep = true;
        self.change(State::Sleep, self.options.sleep);
    }
    pub fn wake(&mut self) {
        self.inactive = 0.;
        self.forced_sleep = false;
        self.change(State::Idle, self.options.idle);
    }
    pub fn interact(&mut self) {
        self.inactive = 0.;
        self.forced_sleep = false;
        if matches!(self.state, State::Sleep | State::Stretch) {
            self.change(State::Idle, self.options.idle);
        }
    }
    /// Turn only when the obstacle is ahead; silhouette changes can clamp the trailing edge.
    pub fn avoid_edge(&mut self, x: f32, min: f32, max: f32) {
        if (self.direction > 0 && x >= max - 1.) || (self.direction < 0 && x <= min + 1.) {
            self.direction *= -1;
        }
    }
    pub fn set_roaming(&mut self, value: bool) {
        self.roaming = value;
        if !value && self.state.moving() {
            self.change(State::Idle, self.options.idle);
        }
    }
    pub fn tick(&mut self, delta: f32, rng: &mut impl Rng) {
        if !delta.is_finite()
            || delta <= 0.
            || matches!(self.state, State::Dragged | State::Airborne)
            || (self.forced_sleep && self.state == State::Sleep)
        {
            return;
        }
        self.inactive += delta;
        if self.state == State::Sleep {
            return;
        }
        if self.inactive >= self.options.sleep
            && matches!(self.state, State::Idle | State::Walk | State::Run)
        {
            self.change(State::Sleep, self.options.sleep);
            return;
        }
        self.remaining -= delta;
        if self.remaining > 0. {
            return;
        }
        let s = match self.state {
            State::Petted
            | State::Fall
            | State::Walk
            | State::Run
            | State::Stretch
            | State::Sleep => State::Idle,
            _ if self.roaming && rng.random_bool(0.65) => {
                self.direction = if rng.random_bool(0.5) { -1 } else { 1 };
                if rng.random_bool(0.25) {
                    State::Run
                } else {
                    State::Walk
                }
            }
            _ if rng.random_bool(0.3) => State::Stretch,
            _ => State::Idle,
        };
        let seconds = match s {
            State::Idle => self.options.idle,
            State::Walk => self.options.walk,
            State::Run => self.options.run,
            State::Sleep => self.options.sleep,
            State::Stretch => 5.5,
            _ => self.options.idle,
        };
        self.change(s, seconds * rng.random_range(0.85..1.15));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    #[test]
    fn changing_silhouette_cannot_reverse_an_inward_turn_at_the_right_edge() {
        let mut brain = Brain::default();
        brain.avoid_edge(1800., 0., 1800.);
        assert_eq!(brain.direction, -1);
        for width in [1790., 1770., 1750., 1790., 1800.] {
            brain.avoid_edge(width, 0., width);
            assert_eq!(brain.direction, -1);
        }
        brain.avoid_edge(0., 0., 1800.);
        assert_eq!(brain.direction, 1);
    }
    #[test]
    fn sleep_requires_uninterrupted_inactivity_and_wakes_on_contact() {
        let mut brain = Brain::default();
        brain.options.sleep = 60.;
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..1000 {
            brain.interact();
            brain.tick(1., &mut rng);
            assert_ne!(brain.state, State::Sleep);
        }
        for _ in 0..70 {
            brain.tick(1., &mut rng);
        }
        assert_eq!(brain.state, State::Sleep);
        brain.tick(600., &mut rng);
        assert_eq!(brain.state, State::Sleep);
        brain.interact();
        assert_eq!(brain.state, State::Idle);
        assert_eq!(brain.inactive, 0.);
        brain.change(State::Dragged, 1.);
        brain.tick(600., &mut rng);
        assert_eq!(brain.inactive, 0.);
    }
}
