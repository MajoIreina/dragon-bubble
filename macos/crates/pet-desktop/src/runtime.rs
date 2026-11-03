use bevy::prelude::*;
use pet_core::{
    assets::Skin,
    behavior::{Brain, State},
    desktop::Desktop,
    motion::Motion,
    preferences::{self, Preferences},
};
use std::{path::PathBuf, sync::mpsc};
#[derive(Clone, Debug)]
pub enum Action {
    ShowSettings(usize),
    SelectSkin(String),
    Import(PathBuf),
    MoveScreen(usize),
    Reset,
    Scale(f32),
    Roam(bool),
    Top(bool),
    Locale(String),
    Preview(State),
    ResetAdaptation,
    Quit,
    ShowSize,
}
#[derive(Resource)]
pub struct Runtime {
    pub root: PathBuf,
    pub data: PathBuf,
    pub prefs: Preferences,
    pub skins: Vec<Skin>,
    pub diagnostics: Vec<String>,
    pub brain: Brain,
    pub motion: Motion,
    pub desktop: Desktop,
    pub position: Vec2,
    pub ground: f32,
    pub actions: Vec<Action>,
    pub show_settings: bool,
    pub show_size: bool,
    pub page: usize,
    pub status: String,
    pub loading: Option<String>,
    pub active: Option<Entity>,
    pub preview: Option<Entity>,
    pub need_preview: bool,
    pub preview_state: State,
    pub importing: Option<std::sync::Mutex<mpsc::Receiver<Result<Skin, String>>>>,
    pub pressed: bool,
    pub dragging: bool,
    pub press_cursor: Vec2,
    pub drag_facing_x: f32,
    pub press_origin: Vec2,
    pub grab_point: Vec2,
    pub last_left: bool,
    pub last_right: bool,
    pub poll: f32,
    pub dirty: bool,
    pub save_timer: f32,
    pub frame_seconds: f32,
    pub quit: bool,
    pub quit_after_import: bool,
    pub smoke: bool,
    pub smoke_step: usize,
    pub smoke_clock: f32,
    pub smoke_original: String,
}
impl Runtime {
    pub fn new(root: PathBuf, smoke: bool, desktop: Desktop) -> Self {
        let data = if smoke {
            root.join("work/rust-smoke-data")
        } else {
            preferences::user_dir()
        };
        let _ = std::fs::create_dir_all(&data);
        let mut prefs = Preferences::load(&data.join("preferences.json"));
        let (skins, diagnostics) =
            pet_core::assets::scan(&[root.join("skins"), data.join("skins")]);
        if !skins.iter().any(|s| s.id == prefs.skin_id) {
            prefs.skin_id = skins
                .iter()
                .find(|s| s.id == "builtin.shu")
                .or(skins.first())
                .map(|s| s.id.clone())
                .unwrap_or_default();
        }
        let mut brain = Brain::default();
        brain.set_roaming(prefs.roaming);
        let position = if prefs.position[0] == -99999 {
            desktop.place_on(0)
        } else {
            desktop.clamp_origin(Vec2::new(
                prefs.position[0] as f32,
                prefs.position[1] as f32,
            ))
        };
        let original = prefs.skin_id.clone();
        Self {
            root,
            data: data.clone(),
            show_settings: !data.join("welcome_seen").exists(),
            prefs,
            skins,
            diagnostics,
            brain,
            motion: Motion::default(),
            desktop,
            position,
            ground: 248.,
            actions: vec![],
            show_size: false,
            page: 0,
            status: String::new(),
            loading: None,
            active: None,
            preview: None,
            need_preview: false,
            preview_state: State::Idle,
            importing: None,
            pressed: false,
            dragging: false,
            press_cursor: Vec2::ZERO,
            drag_facing_x: 0.,
            press_origin: Vec2::ZERO,
            grab_point: Vec2::new(160., 125.),
            last_left: false,
            last_right: false,
            poll: 0.,
            dirty: false,
            save_timer: 0.,
            frame_seconds: 1. / 60.,
            quit: false,
            quit_after_import: false,
            smoke,
            smoke_step: 0,
            smoke_clock: 0.,
            smoke_original: original,
        }
    }
    pub fn save(&mut self) {
        self.prefs.position = [
            self.position.x.round() as i32,
            self.position.y.round() as i32,
        ];
        if let Err(e) = self.prefs.save(&self.data.join("preferences.json")) {
            self.status = format!("设置保存失败：{e}")
        } else {
            self.dirty = false
        }
    }
    pub fn effective_skin(&self, id: &str) -> Option<Skin> {
        self.skins
            .iter()
            .find(|s| s.id == id)
            .map(|s| preferences::overrides(s, &self.data.join("appearance-overrides.json")))
    }
    pub fn rescan(&mut self) {
        (self.skins, self.diagnostics) =
            pet_core::assets::scan(&[self.root.join("skins"), self.data.join("skins")]);
    }
    pub fn set_zoom(&mut self, value: f32) {
        let old = self.prefs.zoom;
        self.prefs.zoom = preferences::zoom(value);
        self.position.y += self.ground * (old - self.prefs.zoom);
        self.desktop.extent = Vec2::new(320. * self.prefs.zoom, self.ground * self.prefs.zoom);
        self.position = self.desktop.clamp_origin(self.position);
        self.motion.position = self.position;
        self.dirty = true;
    }
}
pub fn actions(mut rt: ResMut<Runtime>) {
    let actions = std::mem::take(&mut rt.actions);
    for action in actions {
        match action {
            Action::ShowSettings(page) => {
                rt.show_settings = true;
                rt.page = page;
                rt.need_preview = true;
                let _ = std::fs::write(rt.data.join("welcome_seen"), "1");
            }
            Action::ShowSize => rt.show_size = true,
            Action::SelectSkin(id) => rt.loading = Some(id),
            Action::Scale(v) => rt.set_zoom(v),
            Action::Roam(v) => {
                rt.prefs.roaming = v;
                rt.brain.set_roaming(v);
                rt.dirty = true
            }
            Action::Top(v) => {
                rt.prefs.always_on_top = v;
                rt.dirty = true
            }
            Action::Locale(v) => {
                rt.prefs.locale = pet_core::i18n::normalize(&v);
                rt.dirty = true
            }
            Action::MoveScreen(i) => {
                rt.position = rt.desktop.place_on(i);
                let p = rt.position;
                rt.motion.stop(p);
                rt.brain.change(State::Idle, 4.);
                rt.dirty = true
            }
            Action::Reset => {
                rt.position = rt.desktop.place_on(0);
                let p = rt.position;
                rt.motion.stop(p);
                rt.brain.change(State::Idle, 4.);
                rt.dirty = true
            }
            Action::Preview(s) => {
                rt.preview_state = s;
            }
            Action::ResetAdaptation => {
                let id = rt.prefs.skin_id.clone();
                let _ = preferences::save_override(
                    &id,
                    None,
                    &rt.data.join("appearance-overrides.json"),
                );
                rt.loading = Some(id);
            }
            Action::Import(path) => {
                if rt.importing.is_none() {
                    let (send, recv) = mpsc::channel();
                    let data = rt.data.clone();
                    let root = rt.root.clone();
                    rt.importing = Some(std::sync::Mutex::new(recv));
                    rt.status = "正在导入外观…".into();
                    std::thread::spawn(move || {
                        let result = pet_core::import::install(&path, &data, &root)
                            .map_err(|e| format!("{e:#}"));
                        let _ = send.send(result);
                    });
                }
            }
            Action::Quit => {
                if rt.importing.is_some() {
                    rt.quit_after_import = true;
                    rt.status = "正在导入外观，完成后自动退出。".into()
                } else {
                    rt.save();
                    rt.quit = true
                }
            }
        }
    }
    let imported = rt
        .importing
        .as_ref()
        .and_then(|r| r.lock().ok()?.try_recv().ok());
    if let Some(result) = imported {
        rt.importing = None;
        match result {
            Ok(s) => {
                rt.rescan();
                rt.loading = Some(s.id);
                rt.status = String::new()
            }
            Err(e) => rt.status = format!("导入失败：{e}"),
        }
        if rt.quit_after_import {
            rt.save();
            rt.quit = true;
        }
    }
}
pub fn simulate(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    avatars: Query<&crate::avatar::Avatar>,
    #[cfg(target_os = "macos")] _main_thread: bevy::ecs::system::NonSendMarker,
) {
    let dt = time.delta_secs().min(0.1);
    rt.frame_seconds += (time.delta_secs() - rt.frame_seconds) * 0.05;
    rt.poll += dt;
    if rt.poll >= 1. {
        rt.poll = 0.;
        let layout = crate::platform::desktop();
        if rt.desktop.areas != layout.areas {
            rt.desktop.areas = layout.areas;
            rt.position = rt.desktop.clamp_origin(rt.position);
            let p = rt.position;
            rt.motion.stop(p);
            rt.brain.change(State::Idle, 2.);
            rt.dirty = true
        }
    }
    let turning = rt
        .active
        .and_then(|e| avatars.get(e).ok())
        .is_some_and(|a| a.turning);
    if !rt.pressed && !rt.motion.active && !rt.show_size && !turning {
        rt.brain.tick(dt, &mut rand_rng());
    }
    let blocked = rt.pressed || rt.motion.active || rt.show_size;
    if rt.brain.state.moving() && !blocked && !turning {
        let bounds = rt.desktop.bounds_for(rt.position);
        let distance = if rt.brain.direction > 0 {
            bounds.max.x - rt.position.x
        } else {
            rt.position.x - bounds.min.x
        };
        let braking =
            (distance / (rt.brain.state.speed() * rt.prefs.zoom * 0.18).max(2.)).clamp(0.22, 1.);
        let resume = rt
            .active
            .and_then(|e| avatars.get(e).ok())
            .map_or(1., |a| a.resume_rate());
        rt.position.x += dt
            * rt.brain.state.speed()
            * rt.prefs.zoom
            * braking
            * resume
            * rt.brain.direction as f32;
        let clamped = rt.desktop.clamp_origin(rt.position);
        if clamped != rt.position || distance <= 1. {
            rt.brain.direction *= -1
        }
        rt.position = clamped;
        rt.dirty = true;
    }
    if !rt.pressed && !rt.show_size {
        let bounds = rt.desktop.bounds_for(rt.position);
        if !rt.motion.active && rt.position.y < bounds.max.y - 1. {
            let p = rt.position;
            rt.motion.launch(p, Vec2::ZERO);
            rt.brain.change(State::Airborne, 2.)
        }
        if rt.motion.active {
            let desktop = rt.desktop.clone();
            let impact = rt.motion.advance(dt, bounds, Some(&desktop));
            rt.position = rt.motion.position;
            if rt.motion.velocity.x.abs() > 20. {
                rt.brain.direction = if rt.motion.velocity.x > 0. { 1 } else { -1 }
            }
            if impact.settled {
                rt.brain.change(State::Fall, 0.8);
                rt.dirty = true
            }
        }
    }
    rt.save_timer += dt;
    if rt.dirty && rt.save_timer > 0.6 && !rt.pressed {
        rt.save_timer = 0.;
        rt.save();
    }
}
fn rand_rng() -> impl pet_core::rand_support::Rng {
    pet_core::rand_support::rng()
}

/// Automated smoke run uses a dedicated data directory to keep user settings untouched.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // Bevy system parameters.
pub fn smoke(
    time: Res<Time>,
    mut commands: Commands,
    mut rt: ResMut<Runtime>,
    avatars: Query<&crate::avatar::Avatar>,
    mut settings: Query<(Entity, &mut Window), With<crate::platform::SettingsWindow>>,
    mut exit: MessageWriter<AppExit>,
    mut prepared: Local<bool>,
    views: Res<crate::avatar::RenderViews>,
    frame: Res<crate::layered::DesktopFrame>,
    feet: Query<(&crate::suspension::SuspendedFoot, Entity, &Transform)>,
    mut previous_feet: Local<std::collections::HashMap<Entity, Transform>>,
    mut moving_feet: Local<usize>,
    cameras: Query<
        (
            &Msaa,
            Option<&crate::temporal::TemporalAntiAliasing>,
            Option<&bevy::render::camera::TemporalJitter>,
            Option<&bevy::core_pipeline::prepass::DepthPrepass>,
            Option<&bevy::core_pipeline::prepass::MotionVectorPrepass>,
        ),
        With<crate::avatar::ViewCamera>,
    >,
) {
    const PERFORMANCE_CHECKS: usize = 23;
    const DIRECTION_CHECKS: [(State, i32, &str); 4] = [
        (State::Dragged, -1, "dragged-left"),
        (State::Dragged, 1, "dragged-right"),
        (State::Airborne, -1, "airborne-left"),
        (State::Airborne, 1, "airborne-right"),
    ];
    let skin_start = State::ALL.len() + DIRECTION_CHECKS.len();
    if !rt.smoke {
        return;
    }
    rt.smoke_clock += time.delta_secs();
    if time.elapsed_secs() > 180. {
        eprintln!("SMOKE FAILED: timed out: {}", rt.status);
        exit.write(AppExit::error());
        return;
    }
    if !rt.status.is_empty() && rt.status.contains("失败") {
        eprintln!("SMOKE FAILED: {}", rt.status);
        exit.write(AppExit::error());
        return;
    }
    let Some(active) = rt.active.and_then(|e| avatars.get(e).ok()) else {
        return;
    };
    if active.ready && active.skin.id == "builtin.shu" && active.state == State::Dragged {
        let mut count = 0;
        for (foot, entity, transform) in &feet {
            if Some(foot.owner) == rt.active {
                count += 1;
                if let Some(previous) = previous_feet.insert(entity, *transform)
                    && !previous.rotation.abs_diff_eq(transform.rotation, 0.0001)
                {
                    *moving_feet += 1;
                }
            }
        }
        assert_eq!(count, 4, "Shu must have four suspended feet");
    }
    if !active.ready || rt.smoke_clock < 1.2 {
        return;
    }
    let folder = rt.data.join("screenshots");
    let _ = std::fs::create_dir_all(&folder);
    if !*prepared {
        // Exercise both desktop and compact layouts without changing user preferences.
        let narrow = rt.smoke_step == 7
            || rt.smoke_step == skin_start + rt.skins.len() + PERFORMANCE_CHECKS - 1;
        for (_, mut window) in &mut settings {
            let size = if narrow { (760., 560.) } else { (1120., 800.) };
            window.resolution.set(size.0, size.1);
        }
        if rt.smoke_step < 8 {
            let step = rt.smoke_step;
            rt.brain.change(State::ALL[step], 60.);
            rt.preview_state = State::ALL[step];
            rt.show_settings = true;
            rt.prefs.roaming = false;
            rt.page = if step >= 6 { 5 } else { step };
            rt.prefs.locale = match step {
                6 => "en",
                7 => "ja",
                _ => "zh_CN",
            }
            .into();
        } else if rt.smoke_step < skin_start {
            let (state, direction, _) = DIRECTION_CHECKS[rt.smoke_step - State::ALL.len()];
            rt.brain.change(state, 60.);
            rt.brain.direction = direction;
            rt.show_settings = false;
        } else if rt.smoke_step < skin_start + rt.skins.len() {
            let id = rt.skins[rt.smoke_step - skin_start].id.clone();
            rt.actions.push(Action::SelectSkin(id));
        } else if rt.smoke_step < skin_start + rt.skins.len() + PERFORMANCE_CHECKS {
            let step = rt.smoke_step - skin_start - rt.skins.len();
            rt.prefs.performance = match step {
                0 => pet_core::preferences::RenderOptions::preset("power"),
                1 => pet_core::preferences::RenderOptions::preset("high"),
                _ => pet_core::preferences::RenderOptions::preset("balanced"),
            };
            rt.set_zoom(1.);
            rt.show_settings = step >= 17;
            rt.page = if (17..19).contains(&step) { 0 } else { 4 };
            rt.prefs.locale = "zh_CN".into();
            rt.brain.change(State::Idle, 60.);
            if (2..8).contains(&step) {
                rt.prefs.performance.aa =
                    ["off", "fxaa", "msaa2", "msaa4", "msaa8", "taa"][step - 2].into();
            } else if step >= 8 {
                rt.prefs.performance.aa = "taa".into();
                match step {
                    8 => rt.prefs.performance.ao = true,
                    9 => rt.set_zoom(1.17), // Non-aligned GPU row pitch (374 pixels).
                    10 => rt.set_zoom(2.),
                    11 => rt.set_zoom(3.),
                    12 => rt.set_zoom(0.2),
                    13 | 14 => {
                        rt.brain.change(State::Dragged, 60.);
                        rt.brain.direction = if step == 13 { -1 } else { 1 };
                    }
                    15 => rt.prefs.performance.aa = "off".into(),
                    16 => rt.prefs.performance.aa = "msaa4".into(),
                    18 => rt.actions.push(Action::SelectSkin("example.sprite".into())),
                    19 => rt.actions.push(Action::SelectSkin("builtin.shu".into())),
                    20 => {
                        rt.prefs.performance.resolution = 816;
                        rt.prefs.performance.fps = 91;
                    }
                    21 => {
                        rt.prefs.performance.resolution = 1536;
                        rt.prefs.performance.fps = 240;
                    }
                    22 => {
                        rt.prefs.performance.resolution = 256;
                        rt.prefs.performance.fps = 24;
                        rt.prefs.locale = "en".into();
                    }
                    _ => {}
                }
            }
            if step == 0 {
                rt.actions.push(Action::SelectSkin("builtin.shu".into()));
            }
            rt.position.y = rt.desktop.bounds_for(rt.position).max.y;
            let position = rt.position;
            rt.motion.stop(position);
        }
        *prepared = true;
        rt.smoke_clock = 0.;
        return;
    }
    if rt.smoke_step < skin_start {
        let step = rt.smoke_step;
        let (state, file) = if step < State::ALL.len() {
            (State::ALL[step], format!("pet-{step}.png"))
        } else {
            let (state, direction, label) = DIRECTION_CHECKS[step - State::ALL.len()];
            let expected_yaw = direction as f32 * std::f32::consts::FRAC_PI_2;
            if (active.yaw - expected_yaw).abs() > 0.00001 || active.turning {
                eprintln!("SMOKE FAILED: {label} yaw {}", active.yaw);
                exit.write(AppExit::error());
                return;
            }
            println!("SMOKE direction {label}, yaw {}", active.yaw);
            (state, format!("{label}.png"))
        };
        if active.state != state {
            return;
        }
        let pet_path = folder.join(file);
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                views.output.clone(),
            ))
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                    let image = event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .expect("RGBA screenshot");
                    image
                        .save(&pet_path)
                        .expect("save transparent smoke screenshot");
                },
            );
        if let Ok((window, _)) = settings.single() {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::window(window))
                .observe(bevy::render::view::screenshot::save_to_disk(
                    folder.join(format!("settings-{step}.png")),
                ));
        }
        println!(
            "SMOKE state {} meshes ready, ground {}",
            state.key(),
            active.ground
        );
    } else if rt.smoke_step < skin_start + rt.skins.len() {
        let id = rt.skins[rt.smoke_step - skin_start].id.clone();
        if active.skin.id != id {
            return;
        }
        println!("SMOKE skin {id}");
        let path = folder.join(format!("skin-{id}.png"));
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                views.output.clone(),
            ))
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .expect("RGBA screenshot")
                        .save(&path)
                        .expect("save skin screenshot");
                },
            );
    } else if rt.smoke_step < skin_start + rt.skins.len() + PERFORMANCE_CHECKS {
        let step = rt.smoke_step - skin_start - rt.skins.len();
        if active.skin.id
            != if step == 18 {
                "example.sprite"
            } else {
                "builtin.shu"
            }
        {
            return;
        }
        if step == 13 || step == 14 {
            assert_eq!(active.state, State::Dragged);
            let expected = if step == 13 { -1. } else { 1. } * std::f32::consts::FRAC_PI_2;
            assert!((active.yaw - expected).abs() < 0.00001, "TAA drag reversal");
        }
        let p = &rt.prefs.performance;
        if step >= 19
            && let Ok((window, _)) = settings.single()
        {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::window(window))
                .observe(bevy::render::view::screenshot::save_to_disk(
                    folder.join(format!("sliders-{step}.png")),
                ));
        }
        let use_taa = p.aa == "taa" || (p.ao && p.aa.starts_with("msaa"));
        for (msaa, taa, jitter, depth, motion) in &cameras {
            assert_eq!(taa.is_some(), use_taa, "TAA enable/disable");
            assert_eq!(jitter.is_some(), use_taa, "jitter cleanup");
            assert_eq!(motion.is_some(), use_taa, "motion prepass cleanup");
            assert_eq!(depth.is_some(), use_taa || p.ao, "shared AO depth prepass");
            if use_taa {
                assert_eq!(*msaa, Msaa::Off);
            }
        }
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(frame.size, views.output_size, "native readback dimensions");
            assert_eq!(frame.rgba.len(), (frame.size * frame.size * 4) as usize);
        }
        println!(
            "SMOKE performance {step} {:?}, output {}px, TAA {use_taa}",
            rt.prefs.performance, views.output_size
        );
        let path = folder.join(format!(
            "performance-{}.png",
            rt.smoke_step - skin_start - rt.skins.len()
        ));
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                views.output.clone(),
            ))
            .observe(
                move |event: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
                    event
                        .image
                        .clone()
                        .try_into_dynamic()
                        .expect("RGBA screenshot")
                        .save(&path)
                        .expect("save performance screenshot");
                },
            );
    } else {
        assert!(
            *moving_feet > 20,
            "suspended feet must animate across frames"
        );
        println!("SMOKE suspended foot changes: {}", *moving_feet);
        println!(
            "SMOKE PASS: {} states, {} skins, {PERFORMANCE_CHECKS} performance modes, 4 side-facing checks",
            8,
            rt.skins.len()
        );
        let original = rt.smoke_original.clone();
        rt.prefs.skin_id = original;
        rt.save();
        exit.write(AppExit::Success);
    }
    rt.smoke_step += 1;
    *prepared = false;
    rt.smoke_clock = 0.;
}
