use rand::Rng;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Idle,
    Walk,
    Run,
    Sleep,
    Petted,
    Dragged,
    Airborne,
    Fall,
}
impl State {
    pub const ALL: [Self; 8] = [
        Self::Idle,
        Self::Walk,
        Self::Run,
        Self::Sleep,
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
}
impl Default for Brain {
    fn default() -> Self {
        Self {
            state: State::Idle,
            direction: 1,
            roaming: true,
            remaining: 4.,
        }
    }
}
impl Brain {
    pub fn change(&mut self, state: State, seconds: f32) {
        self.state = state;
        self.remaining = seconds;
    }
    pub fn set_roaming(&mut self, value: bool) {
        self.roaming = value;
        if !value && self.state.moving() {
            self.change(State::Idle, 5.);
        }
    }
    pub fn tick(&mut self, delta: f32, rng: &mut impl Rng) {
        if !delta.is_finite()
            || delta <= 0.
            || matches!(self.state, State::Dragged | State::Airborne)
        {
            return;
        }
        self.remaining -= delta;
        if self.remaining > 0. {
            return;
        }
        let (s, t) = match self.state {
            State::Petted | State::Fall => (State::Idle, rng.random_range(3.0..6.0)),
            State::Walk | State::Run => (State::Idle, rng.random_range(3.0..7.0)),
            State::Sleep => (State::Idle, rng.random_range(4.0..8.0)),
            _ if self.roaming && rng.random_bool(0.65) => {
                self.direction = if rng.random_bool(0.5) { -1 } else { 1 };
                if rng.random_bool(0.25) {
                    (State::Run, rng.random_range(1.2..2.8))
                } else {
                    (State::Walk, rng.random_range(2.0..5.0))
                }
            }
            _ => (State::Sleep, rng.random_range(6.0..12.0)),
        };
        self.change(s, t);
    }
}
