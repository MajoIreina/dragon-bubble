//! A normal Bevy window keeps simulation and rendering running while the menu is open.
use crate::runtime::{Action, Runtime};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    window::{WindowLevel, WindowRef, WindowResolution},
};
use bevy_egui::{EguiContext, EguiMultipassSchedule, egui};
use pet_core::behavior::State;

#[derive(Component)]
pub struct MenuWindow;
#[derive(Component)]
pub struct MenuCamera;
#[derive(bevy::ecs::schedule::ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MenuPass;

#[derive(Resource, Default)]
pub struct MenuState {
    pub visible: bool,
    requested: Option<Vec2>,
    focused_once: bool,
    opened_at: f64,
    height: f32,
}
impl MenuState {
    pub fn open(&mut self, position: Vec2) {
        self.requested = Some(position);
        self.visible = true;
        self.focused_once = false;
    }
}

pub fn setup(mut commands: Commands) {
    let window = commands
        .spawn((
            Window {
                title: "龙泡泡 · 菜单".into(),
                resolution: WindowResolution::new(278, 426),
                decorations: false,
                transparent: !cfg!(windows),
                #[cfg(target_os = "macos")]
                composite_alpha_mode: bevy::window::CompositeAlphaMode::PostMultiplied,
                resizable: false,
                skip_taskbar: true,
                visible: false,
                focused: false,
                window_level: WindowLevel::AlwaysOnTop,
                ..default()
            },
            MenuWindow,
        ))
        .id();
    commands.spawn((
        Camera2d,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(window)),
            order: 3,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderLayers::layer(4),
        MenuCamera,
        EguiMultipassSchedule::new(MenuPass),
    ));
}

pub fn sync(
    time: Res<Time>,
    rt: Res<Runtime>,
    mut menu: ResMut<MenuState>,
    mut window: Single<(Entity, &mut Window), With<MenuWindow>>,
    mut camera: Single<&mut Camera, With<MenuCamera>>,
    #[cfg(windows)] mut region_size: Local<Option<(u32, u32, u32)>>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    let (entity, win) = &mut *window;
    if let Some(position) = menu.requested.take() {
        menu.opened_at = time.elapsed_secs_f64();
        // Windows desktop coordinates are physical pixels; AppKit uses logical points.
        bevy::winit::WINIT_WINDOWS.with(|windows| {
            if let Some(native) = windows.borrow().get_window(*entity) {
                let scale = if cfg!(target_os = "macos") {
                    1.
                } else {
                    native.scale_factor() as f32
                };
                let bounds = rt
                    .desktop
                    .areas
                    .iter()
                    .find(|area| {
                        position.x >= area.min.x
                            && position.x <= area.max.x
                            && position.y >= area.min.y
                            && position.y <= area.max.y
                    })
                    .unwrap_or(&rt.desktop.areas[0]);
                let size = Vec2::new(win.width(), win.height()) * scale;
                let origin = position
                    .max(bounds.min)
                    .min((bounds.max - size).max(bounds.min));
                #[cfg(target_os = "macos")]
                crate::macos::position_window(native, origin);
                #[cfg(not(target_os = "macos"))]
                {
                    win.position = bevy::window::WindowPosition::At(origin.as_ivec2());
                }
                native.set_visible(true);
                native.focus_window();
            }
        });
    }
    if menu.height > 0. && (win.height() - menu.height).abs() > 0.5 {
        win.resolution.set(278., menu.height);
    }
    #[cfg(windows)]
    bevy::winit::WINIT_WINDOWS.with(|windows| {
        if let Some(native) = windows.borrow().get_window(*entity) {
            let scale = native.scale_factor() as f32;
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = native.window_handle()
                && let RawWindowHandle::Win32(handle) = handle.as_raw()
            {
                let pixels = native.inner_size();
                let radius = (24. * scale).round() as i32;
                let key = (pixels.width, pixels.height, radius as u32);
                if *region_size != Some(key) {
                    unsafe {
                        let region = windows_sys::Win32::Graphics::Gdi::CreateRoundRectRgn(
                            0,
                            0,
                            pixels.width as i32 + 1,
                            pixels.height as i32 + 1,
                            radius,
                            radius,
                        );
                        if windows_sys::Win32::Graphics::Gdi::SetWindowRgn(
                            handle.hwnd.get() as _,
                            region,
                            1,
                        ) == 0
                        {
                            windows_sys::Win32::Graphics::Gdi::DeleteObject(region);
                        } else {
                            *region_size = Some(key);
                        }
                    }
                }
            }
        }
    });
    if menu.visible {
        menu.focused_once |= win.focused;
        if menu.focused_once && !win.focused && time.elapsed_secs_f64() - menu.opened_at > 0.2 {
            menu.visible = false;
        }
    }
    win.visible = menu.visible;
    camera.is_active = menu.visible;
}

pub fn draw(
    mut context: Single<&mut EguiContext, With<MenuCamera>>,
    mut rt: ResMut<Runtime>,
    mut menu: ResMut<MenuState>,
    mut initialized: Local<bool>,
    mut camera: Single<&mut Camera, With<MenuCamera>>,
    #[cfg(target_os = "macos")] _main_thread: bevy::ecs::system::NonSendMarker,
) {
    let ctx = context.get_mut();
    if !*initialized {
        crate::ui::configure_context(ctx);
        *initialized = true;
        return;
    }
    if !menu.visible {
        return;
    }
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        menu.visible = false;
    }
    if cfg!(windows) {
        let fill = ctx.style().visuals.panel_fill;
        camera.clear_color = ClearColorConfig::Custom(Color::srgb_u8(fill.r(), fill.g(), fill.b()));
    }
    let tr = |text: &str| pet_core::i18n::tr(&rt.prefs.locale, text);
    let mut action = None;
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            let panel = egui::Frame::new()
                .fill(ui.visuals().panel_fill)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(12)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 3.;
                    ui.label(
                        egui::RichText::new(tr("龙泡泡"))
                            .strong()
                            .color(ui.visuals().hyperlink_color),
                    );
                    ui.separator();
                    let sleeping = rt.brain.forced_sleep || rt.brain.state == State::Sleep;
                    for (label, item) in [
                        (tr("设置"), Action::ShowSettings(0)),
                        (
                            tr(if sleeping { "唤醒" } else { "一键睡觉" }),
                            Action::Sleep(!sleeping),
                        ),
                        (tr("检查更新"), Action::CheckUpdates),
                        (
                            format!(
                                "{} {}",
                                if rt.prefs.always_on_top { "✓" } else { "  " },
                                tr("始终置顶")
                            ),
                            Action::Top(!rt.prefs.always_on_top),
                        ),
                        (
                            format!(
                                "{} {}",
                                if rt.prefs.roaming { "✓" } else { "  " },
                                tr("自由走动")
                            ),
                            Action::Roam(!rt.prefs.roaming),
                        ),
                        (tr("性能与画质"), Action::ShowSettings(4)),
                        (tr("重置位置"), Action::Reset),
                    ] {
                        if ui
                            .add_sized(
                                [ui.available_width(), 32.],
                                egui::Button::new(label).frame(false),
                            )
                            .clicked()
                        {
                            action = Some(item);
                        }
                    }
                    ui.separator();
                    egui::ComboBox::from_id_salt("menu-skin")
                        .width(ui.available_width() - 8.)
                        .selected_text(tr("外观"))
                        .show_ui(ui, |ui| {
                            for skin in &rt.skins {
                                if ui
                                    .selectable_label(skin.id == rt.prefs.skin_id, tr(&skin.name))
                                    .clicked()
                                {
                                    action = Some(Action::SelectSkin(skin.id.clone()));
                                }
                            }
                        });
                    egui::ComboBox::from_id_salt("menu-screen")
                        .width(ui.available_width() - 8.)
                        .selected_text(tr("移动到屏幕"))
                        .show_ui(ui, |ui| {
                            for index in 0..rt.desktop.areas.len() {
                                if ui.button(format!("{} {}", tr("屏幕"), index + 1)).clicked() {
                                    action = Some(Action::MoveScreen(index));
                                }
                            }
                        });
                    ui.separator();
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.],
                            egui::Button::new(tr("退出")).frame(false),
                        )
                        .clicked()
                    {
                        action = Some(Action::Quit);
                    }
                });
            menu.height = panel.response.rect.height().ceil();
        });
    if let Some(action) = action {
        rt.actions.push(action);
        menu.visible = false;
    }
}

/// Native menu/rendering regression probe, using the isolated smoke data directory.
#[allow(clippy::too_many_arguments)]
pub fn probe(
    time: Res<Time>,
    mut rt: ResMut<Runtime>,
    mut menu: ResMut<MenuState>,
    avatars: Query<(&crate::avatar::Avatar, &Transform)>,
    menus: Query<Entity, With<MenuWindow>>,
    settings: Query<Entity, With<crate::platform::SettingsWindow>>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
    mut started: Local<Option<f64>>,
    mut captured: Local<bool>,
    shadow_map: Res<bevy::light::DirectionalLightShadowMap>,
) {
    if time.elapsed_secs() > 60. {
        panic!("Menu rendering probe timed out: {}", rt.status);
    }
    let Some((avatar, transform)) = rt.active.and_then(|entity| avatars.get(entity).ok()) else {
        return;
    };
    if !avatar.ready {
        return;
    }
    if started.is_none() {
        rt.prefs.locale = "zh_CN".into();
        rt.prefs.roaming = false;
        rt.brain.sleep_now();
        rt.show_settings = true;
        rt.page = 4;
        rt.prefs.performance = pet_core::preferences::RenderOptions::preset("extreme");
        let position = rt.desktop.areas[0].min + Vec2::new(360., 160.);
        menu.open(position);
        *started = Some(time.elapsed_secs_f64());
        return;
    }
    let start = started.unwrap();
    let elapsed = time.elapsed_secs_f64() - start;
    if elapsed > 4. && !*captured {
        assert!(menu.visible, "Menu must remain open during simulation");
        assert_eq!(avatar.state, State::Sleep);
        assert!(
            avatar.clock > 2.,
            "Animation must advance with the menu open"
        );
        assert!(
            transform.scale.y / transform.scale.x > 0.9,
            "Sleep silhouette must remain full"
        );
        assert_eq!(
            shadow_map.size, 4096,
            "Extreme quality must apply to rendering"
        );
        let folder = rt.data.join("screenshots");
        std::fs::create_dir_all(&folder).expect("probe screenshot directory");
        for (entity, filename) in [
            (menus.single().unwrap(), "rounded-menu.png"),
            (settings.single().unwrap(), "five-level-settings.png"),
        ] {
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::window(entity))
                .observe(bevy::render::view::screenshot::save_to_disk(
                    folder.join(filename),
                ));
        }
        println!(
            "MENU PASS: animation advanced {:.2}s with menu open; full sleep silhouette; extreme quality",
            avatar.clock
        );
        *captured = true;
    }
    if elapsed > 8. {
        exit.write(AppExit::Success);
    }
}
