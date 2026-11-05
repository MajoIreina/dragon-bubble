//! Timed pointer practice; owns no model, image, GPU resource or persistent worker.
use crate::runtime::Runtime;
use bevy::prelude::*;
use pet_core::{behavior::State, preferences::atomic_write, rand_support::Rng};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub const PAGE: usize = 6;
pub const LENGTH: Duration = Duration::from_secs(60);
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub timestamp: u64,
    pub hits: u32,
    pub screen: usize,
    pub width: u32,
    pub height: u32,
}
#[derive(Default)]
pub struct Game {
    pub countdown: Option<Instant>,
    pub started: Option<Instant>,
    pub screen: usize,
    pub hits: u32,
    pub history: Vec<Record>,
    pub result: Option<Record>,
    pub message: String,
    origin: Option<(Vec2, f32)>,
    last_hit: Option<Instant>,
    layout_poll: Option<Instant>,
}
impl Game {
    pub fn load(path: &Path) -> Self {
        let history = path
            .metadata()
            .ok()
            .filter(|m| m.len() < 1024 * 1024)
            .and_then(|_| std::fs::read(path).ok())
            .and_then(|b| serde_json::from_slice::<Vec<Record>>(&b).ok())
            .unwrap_or_default()
            .into_iter()
            .rev()
            .take(300)
            .collect::<Vec<_>>();
        Self {
            history: history.into_iter().rev().collect(),
            ..Self::default()
        }
    }
    pub fn active(&self) -> bool {
        self.countdown.is_some() || self.started.is_some()
    }
    pub fn remaining(&self, now: Instant) -> f32 {
        self.started.map_or(60., |start| {
            LENGTH
                .saturating_sub(now.saturating_duration_since(start))
                .as_secs_f32()
        })
    }
    pub fn accepts(&self, now: Instant) -> bool {
        self.started
            .is_some_and(|start| now.saturating_duration_since(start) < LENGTH)
            && self
                .last_hit
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_millis(80))
    }
}
pub fn start(rt: &mut Runtime) {
    if rt.game.active() || rt.obscured {
        return;
    }
    rt.game.origin = Some((rt.position, rt.prefs.zoom));
    rt.game.screen = rt.game.screen.min(rt.desktop.areas.len().saturating_sub(1));
    rt.game.hits = 0;
    rt.game.result = None;
    rt.game.message.clear();
    rt.game.last_hit = None;
    rt.game.countdown = Some(Instant::now());
    rt.pressed = false;
    rt.dragging = false;
    rt.set_zoom(0.55);
    rt.brain.wake();
    rt.brain.change(State::Idle, 120.);
    next_target(rt);
    rt.motion.stop(rt.position);
}
fn restore(rt: &mut Runtime) {
    rt.game.started = None;
    rt.game.countdown = None;
    if let Some((position, zoom)) = rt.game.origin.take() {
        rt.set_zoom(zoom);
        rt.position = rt.desktop.clamp_origin(position);
    }
    rt.motion.stop(rt.position);
    rt.brain.change(State::Idle, 4.);
    rt.page = PAGE;
    rt.show_settings = true;
    rt.dirty = true;
}
pub fn cancel(rt: &mut Runtime, message: &str) {
    if !rt.game.active() {
        return;
    }
    rt.game.message = message.into();
    restore(rt);
}
fn next_target(rt: &mut Runtime) {
    let Some(area) = rt.desktop.areas.get(rt.game.screen).copied() else {
        return;
    };
    let min = area.min + Vec2::splat(8.);
    let max = (area.max - rt.desktop.extent - Vec2::splat(8.)).max(min);
    let mut rng = pet_core::rand_support::rng();
    let old = rt.position;
    let mut best = min;
    for _ in 0..12 {
        let candidate = min + (max - min) * Vec2::new(rng.random::<f32>(), rng.random::<f32>());
        if candidate.distance_squared(old) > best.distance_squared(old) {
            best = candidate;
        }
        if candidate.distance(old) >= (max - min).length().min(400.) * 0.35 {
            best = candidate;
            break;
        }
    }
    rt.position = best;
    rt.motion.stop(best);
}
pub fn hit(rt: &mut Runtime, now: Instant) {
    if !rt.game.accepts(now) {
        return;
    }
    rt.game.last_hit = Some(now);
    rt.game.hits += 1;
    rt.pet_reaction = rt.pet_reaction.wrapping_add(1);
    next_target(rt);
}
/// Wall-clock deadlines prevent frame stalls/focus loss from extending a round.
pub fn tick(rt: &mut Runtime) -> bool {
    if !rt.game.active() {
        return false;
    }
    if rt.obscured || rt.game.screen >= rt.desktop.areas.len() {
        cancel(rt, "练习已中止：全屏应用或屏幕变化。本轮不计入历史。");
        return false;
    }
    let now = Instant::now();
    if rt
        .game
        .layout_poll
        .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(250))
    {
        rt.game.layout_poll = Some(now);
        let layout = crate::platform::desktop();
        if layout.areas != rt.desktop.areas {
            rt.desktop.areas = layout.areas;
            cancel(rt, "屏幕布局发生变化，本轮已中止。");
            return false;
        }
    }
    if rt
        .game
        .countdown
        .is_some_and(|start| now.duration_since(start) >= Duration::from_secs(3))
    {
        rt.game.countdown = None;
        rt.game.started = Some(now);
        rt.show_settings = false;
    }
    if rt
        .game
        .started
        .is_some_and(|start| now.duration_since(start) >= LENGTH)
    {
        let area = rt.desktop.areas[rt.game.screen];
        let record = Record {
            id: format!(
                "{:016x}{:016x}",
                pet_core::rand_support::random::<u64>(),
                pet_core::rand_support::random::<u64>()
            ),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            hits: rt.game.hits,
            screen: rt.game.screen,
            width: area.size().x as u32,
            height: area.size().y as u32,
        };
        rt.game.history.push(record.clone());
        if rt.game.history.len() > 300 {
            rt.game.history.remove(0);
        }
        rt.game.result = Some(record.clone());
        if let Err(e) = serde_json::to_vec(&rt.game.history)
            .map_err(std::io::Error::other)
            .and_then(|body| atomic_write(&rt.data.join("practice-history.json"), &body))
        {
            rt.game.message = format!("历史记录保存失败：{e}");
        }
        rt.leaderboard.submit(&record);
        restore(rt);
        return false;
    }
    rt.brain.change(State::Idle, 120.);
    true
}
/// Calendar conversion for portable, unambiguous history labels (UTC).
pub fn format_timestamp(seconds: u64) -> String {
    let days = (seconds / 86400).min(3652425) as i64 + 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds / 3600 % 24,
        seconds / 60 % 60
    )
}

#[derive(Default)]
pub struct Probe {
    phase: usize,
    clock: f64,
    origin: Vec2,
    zoom: f32,
    last: f64,
}
pub fn probe(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    avatars: Query<&crate::avatar::Avatar>,
    windows: Query<Entity, With<crate::platform::SettingsWindow>>,
    mut commands: Commands,
    mut check: Local<Probe>,
    mut exit: MessageWriter<AppExit>,
) {
    assert!(time.elapsed_secs() < 100., "Practice probe timed out");
    if !rt
        .active
        .and_then(|e| avatars.get(e).ok())
        .is_some_and(|a| a.ready)
    {
        return;
    }
    let t = time.elapsed_secs_f64();
    if check.phase == 0 {
        rt.game.history.clear();
        rt.game.result = None;
        rt.page = PAGE;
        rt.show_settings = true;
        rt.prefs.theme = "light".into();
        check.clock = t;
        check.phase = 1;
    }
    let mut screenshot = None;
    match check.phase {
        1 if t - check.clock > 1. => {
            screenshot = Some("practice-light");
            check.phase = 2;
            check.clock = t;
        }
        2 if t - check.clock > 1. => {
            rt.prefs.theme = "dark".into();
            check.phase = 3;
            check.clock = t;
        }
        3 if t - check.clock > 1. => {
            screenshot = Some("practice-dark");
            check.phase = 4;
            check.clock = t;
        }
        4 if t - check.clock > 1. => {
            check.origin = rt.position;
            check.zoom = rt.prefs.zoom;
            start(&mut rt);
            hit(&mut rt, Instant::now());
            assert_eq!(rt.game.hits, 0);
            check.phase = 5;
            check.clock = t;
        }
        5 => {
            if rt.game.countdown.is_some() && t - check.clock > 1. && t - check.clock < 1.2 {
                screenshot = Some("practice-countdown");
            }
            if rt.game.started.is_some() && t - check.last > 0.14 {
                let area = rt.desktop.areas[rt.game.screen];
                assert!(rt.position.cmpge(area.min).all());
                assert!(
                    (rt.position + rt.desktop.extent)
                        .cmple(area.max + Vec2::splat(1.))
                        .all()
                );
                hit(&mut rt, Instant::now());
                check.last = t;
            }
            if rt.game.result.is_some() {
                assert_eq!(rt.game.history.len(), 1);
                assert!(rt.game.hits > 200 && rt.game.hits <= 430);
                assert_eq!(rt.prefs.zoom, check.zoom);
                assert!(rt.position.distance(check.origin) < 1.);
                assert!(rt.show_settings);
                let loaded = Game::load(&rt.data.join("practice-history.json"));
                assert_eq!(loaded.history[0].hits, rt.game.hits);
                check.phase = 6;
                check.clock = t;
            }
        }
        6 if t - check.clock > 1. => {
            screenshot = Some("practice-result-dark");
            rt.prefs.theme = "light".into();
            check.phase = 7;
            check.clock = t;
        }
        7 if t - check.clock > 1. => {
            screenshot = Some("practice-result-light");
            check.phase = 8;
            check.clock = t;
        }
        8 if t - check.clock > 1. => {
            start(&mut rt);
            cancel(&mut rt, "Probe cancellation");
            assert_eq!(rt.game.history.len(), 1);
            start(&mut rt);
            rt.obscured = true;
            tick(&mut rt);
            rt.obscured = false;
            assert!(!rt.game.active());
            assert_eq!(rt.game.history.len(), 1);
            const {
                assert!(!crate::leaderboard::ENABLED);
            }
            println!(
                "PRACTICE PASS: actual 3-second countdown and 60-second round, random targets, hit debounce, persisted history, restoration and cancellation; network disabled"
            );
            exit.write(AppExit::Success);
        }
        _ => (),
    }
    if let Some(name) = screenshot
        && let Ok(window) = windows.single()
    {
        let folder = rt.data.join("screenshots");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("{name}.png"));
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::window(window))
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .unwrap()
                        .save(&path)
                        .unwrap();
                },
            );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_is_bounded_and_calendar_labels_are_readable() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_timestamp(1709164800), "2024-02-29 00:00 UTC");
        let path =
            std::env::temp_dir().join(format!("practice-history-{}.json", std::process::id()));
        let entries = (0..400)
            .map(|i| Record {
                id: i.to_string(),
                timestamp: i,
                hits: 1,
                screen: 0,
                width: 1920,
                height: 1080,
            })
            .collect::<Vec<_>>();
        std::fs::write(&path, serde_json::to_vec(&entries).unwrap()).unwrap();
        let game = Game::load(&path);
        assert_eq!(game.history.len(), 300);
        assert_eq!(game.history[0].timestamp, 100);
        assert_eq!(game.history[299].timestamp, 399);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn countdown_deadline_and_hit_debounce() {
        let now = Instant::now();
        let mut game = Game {
            countdown: Some(now),
            ..Default::default()
        };
        assert!(!game.accepts(now));
        game.started = Some(now);
        assert!(game.accepts(now));
        game.last_hit = Some(now);
        assert!(!game.accepts(now + Duration::from_millis(79)));
        assert!(game.accepts(now + Duration::from_millis(80)));
        assert!(!game.accepts(now + LENGTH));
        assert_eq!(game.remaining(now + LENGTH), 0.);
    }
}
