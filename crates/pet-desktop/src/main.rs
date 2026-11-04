#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod autostart;
mod avatar;
mod bundled;
mod composite;
mod desktop_mode;
mod drag_probe;
mod effects;
mod expressions;
mod face;
mod layered;
mod platform;
mod popup;
mod rear_stance;
mod resource_probe;
mod runtime;
mod single_instance;
mod soft_body;
mod suspension;
mod temporal;
mod toon;
mod ui;
mod updates;
use bevy::{
    asset::{
        AssetPlugin,
        io::{AssetSource, file::FileAssetReader},
    },
    prelude::*,
    render::{
        RenderPlugin,
        settings::{Backends, RenderCreation, WgpuSettings},
    },
    window::{ExitCondition, WindowPlugin, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
fn project_root() -> std::path::PathBuf {
    let current = std::env::current_dir().unwrap_or_default();
    if current.join("Cargo.toml").is_file() && current.join("converters/model_to_glb.py").is_file()
    {
        return current;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut p = exe.parent();
        while let Some(parent) = p {
            if parent.join("skins").is_dir() || parent.join("converters/model_to_glb.py").is_file()
            {
                return parent.to_owned();
            }
            p = parent.parent();
        }
        if let Some(parent) = exe.parent() {
            return parent.to_owned();
        }
    }
    current
}
fn main() -> AppExit {
    let _instance = match single_instance::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            eprintln!("Dragon Bubble is already running.");
            return AppExit::Success;
        }
        Err(error) => {
            eprintln!("Cannot acquire single-instance lock: {error}");
            return AppExit::error();
        }
    };
    let drag_test = std::env::args().any(|arg| arg == "--drag-test");
    let resource_test = std::env::args().any(|arg| arg == "--resource-test");
    let menu_test = std::env::args().any(|arg| arg == "--menu-test");
    let bounce_test = std::env::args().any(|s| s == "--bounce-test");
    let smoke = drag_test
        || resource_test
        || menu_test
        || bounce_test
        || std::env::args().any(|s| s == "--smoke-test");
    let root = project_root();
    let mut runtime = runtime::Runtime::new(root.clone(), smoke);
    runtime.loading = Some(runtime.prefs.skin_id.clone());
    runtime.desktop.extent = Vec2::new(
        320. * runtime.prefs.zoom,
        runtime.ground * runtime.prefs.zoom,
    );
    let data = runtime.data.clone();
    let mut app = App::new();
    app.register_asset_source(
        "user",
        AssetSource::build().with_reader(move || Box::new(FileAssetReader::new(data.clone()))),
    );
    app.insert_resource(runtime)
        .init_resource::<layered::DesktopFrame>()
        .init_resource::<popup::MenuState>()
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.91, 0.93, 1.),
            brightness: 700.,
            ..default()
        })
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    memory_hints: bevy::render::settings::MemoryHints::MemoryUsage,
                    ..default()
                }),
                // Keep the old frame visible until a new material pipeline is ready.
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(AssetPlugin {
                file_path: root.to_string_lossy().into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "龙泡泡 · My Dragon Bubble".into(),
                    resolution: WindowResolution::new(320, 320).with_scale_factor_override(1.),
                    transparent: true,
                    decorations: false,
                    resizable: false,
                    skip_taskbar: true,
                    focused: false,
                    visible: !cfg!(windows),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                close_when_requested: false,
                ..default()
            }),
    )
    .add_plugins((
        bevy::anti_alias::fxaa::FxaaPlugin,
        temporal::TemporalAntiAliasPlugin,
        composite::CompositePlugin,
        toon::ToonPlugin,
    ))
    .add_plugins(EguiPlugin::default())
    .add_systems(
        Startup,
        (
            avatar::setup,
            ui::setup,
            popup::setup,
            platform::setup_tray,
            layered::setup,
        ),
    )
    .add_systems(
        Update,
        (
            desktop_mode::detect,
            platform::events,
            platform::wheel,
            platform::input,
            runtime::actions,
            avatar::request,
            avatar::load_scenes,
            runtime::simulate,
            avatar::animate,
            expressions::reset,
            suspension::reset,
            effects::update,
            avatar::resize_output,
            avatar::performance,
            desktop_mode::sync,
            platform::sync_windows,
            popup::sync,
            platform::frame_pacing,
        )
            .chain(),
    )
    .add_systems(
        PostUpdate,
        (expressions::apply, suspension::apply, face::update)
            .chain()
            .after(bevy::app::AnimationSystems)
            .before(bevy::transform::TransformSystems::Propagate),
    )
    .add_systems(
        PostUpdate,
        (avatar::finish, soft_body::install, soft_body::update)
            .chain()
            .after(bevy::transform::TransformSystems::Propagate),
    )
    .add_systems(EguiPrimaryContextPass, ui::draw)
    .add_systems(popup::MenuPass, popup::draw)
    .add_systems(Last, layered::present);
    if drag_test {
        app.add_systems(PostUpdate, drag_probe::run.after(soft_body::update));
    } else if resource_test {
        app.add_systems(Update, resource_probe::run.after(platform::frame_pacing));
    } else if menu_test {
        app.add_systems(Update, popup::probe.after(avatar::animate));
    } else if bounce_test {
        app.add_systems(Update, runtime::bounce_smoke.after(avatar::animate));
    } else {
        // Inspect final animated transforms, after the per-frame reset and pose layers.
        app.add_systems(PostUpdate, runtime::smoke.after(soft_body::update));
    }
    bundled::register(&mut app);
    app.run()
}
