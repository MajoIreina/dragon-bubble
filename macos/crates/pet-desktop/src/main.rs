#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod avatar;
mod bundled;
mod composite;
mod effects;
mod expressions;
mod face;
mod layered;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod menu_font;
mod platform;
mod runtime;
mod soft_body;
mod startup;
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
    // Finder launches with an arbitrary working directory. Prefer bundled resources.
    if let Ok(exe) = std::env::current_exe()
        && let Some(contents) = exe.parent().and_then(std::path::Path::parent)
    {
        let resources = contents.join("Resources");
        if resources.join("converters/model_to_glb.py").is_file() {
            return resources;
        }
    }
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
    startup::install_diagnostics();
    run()
}
fn run() -> AppExit {
    let bounce_test = std::env::args().any(|s| s == "--bounce-test");
    let smoke = bounce_test || std::env::args().any(|s| s == "--smoke-test");
    let root = project_root();
    // NSScreen and NSStatusItem require an initialized AppKit event loop.
    // Startup runs after winit has created that event loop, on the main thread.
    let mut runtime =
        runtime::Runtime::new(root.clone(), smoke, pet_core::desktop::Desktop::default());
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
                    backends: Some(if cfg!(target_os = "macos") {
                        Backends::METAL
                    } else {
                        Backends::VULKAN
                    }),
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
                    resolution: if cfg!(target_os = "macos") {
                        WindowResolution::new(320, 320)
                    } else {
                        WindowResolution::new(320, 320).with_scale_factor_override(1.)
                    },
                    transparent: true,
                    // wgpu's Metal surface exposes Opaque/PostMultiplied only.
                    // PreMultiplied passes compilation but panics on configure.
                    composite_alpha_mode: bevy::window::CompositeAlphaMode::PostMultiplied,
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
        startup::desktop
            .before(avatar::setup)
            .before(ui::setup)
            .before(platform::setup_tray),
    )
    .add_systems(
        Startup,
        (
            avatar::setup,
            ui::setup,
            platform::setup_tray,
            layered::setup,
        ),
    )
    .add_systems(
        Update,
        (
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
            platform::sync_windows,
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
    .add_systems(Last, layered::present);
    if bounce_test {
        app.add_systems(Update, runtime::bounce_smoke.after(avatar::animate));
    } else {
        // Inspect final animated transforms, after the per-frame reset and pose layers.
        app.add_systems(PostUpdate, runtime::smoke.after(soft_body::update));
    }
    bundled::register(&mut app);
    if std::env::args().any(|arg| arg == "--startup-test") {
        app.add_systems(Update, startup::probe.after(platform::sync_windows));
    }
    app.run()
}
