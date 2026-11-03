//! Finder has no terminal: retain stderr, including panics and Metal failures.
use bevy::prelude::*;

pub fn install_diagnostics() {
    #[cfg(target_os = "macos")]
    {
        use std::{fs::OpenOptions, os::fd::AsRawFd};
        let folder = pet_core::preferences::user_dir().join("logs");
        if std::fs::create_dir_all(&folder).is_ok() {
            let path = folder.join("latest.log");
            if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
                if std::env::var_os("LPP_LOG_TO_STDERR").is_none() {
                    // SAFETY: dup2 duplicates an open owned file descriptor; the
                    // duplicated descriptors stay valid when File is dropped.
                    unsafe {
                        libc::dup2(file.as_raw_fd(), libc::STDOUT_FILENO);
                        libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO);
                    }
                }
                eprintln!(
                    "\nDragon Bubble {} startup pid={} time={:?} exe={:?} cwd={:?}",
                    crate::updates::VERSION,
                    std::process::id(),
                    std::time::SystemTime::now(),
                    std::env::current_exe(),
                    std::env::current_dir()
                );
            }
        }
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        previous(panic);
        eprintln!(
            "Startup/runtime backtrace:\n{}",
            std::backtrace::Backtrace::force_capture()
        );
    }));
}

pub fn desktop(world: &mut World) {
    let layout = crate::platform::desktop();
    let mut rt = world.resource_mut::<crate::runtime::Runtime>();
    rt.desktop.areas = layout.areas;
    rt.position = if rt.prefs.position[0] == -99999 {
        rt.desktop.place_on(0)
    } else {
        rt.desktop.clamp_origin(Vec2::new(
            rt.prefs.position[0] as f32,
            rt.prefs.position[1] as f32,
        ))
    };
    info!(
        "Desktop initialized: {} screens; resources {:?}; {} skins",
        rt.desktop.areas.len(),
        rt.root,
        rt.skins.len()
    );
}

pub fn probe(
    time: Res<Time>,
    mut rt: ResMut<crate::runtime::Runtime>,
    avatars: Query<&crate::avatar::Avatar>,
    frame: Res<crate::layered::DesktopFrame>,
    mut exit: MessageWriter<AppExit>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    if time.elapsed_secs() > 90. {
        eprintln!("STARTUP FAILED: no rendered pet: {}", rt.status);
        *done = true;
        exit.write(AppExit::error());
        return;
    }
    let ready = rt
        .active
        .and_then(|entity| avatars.get(entity).ok())
        .is_some_and(|avatar| avatar.ready);
    if !ready || frame.serial < 5 || time.elapsed_secs() < 8. {
        return;
    }
    assert!(
        frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] > 32),
        "pet must render visible pixels"
    );
    rt.save();
    image::save_buffer(
        rt.data.join("startup-pet.png"),
        &frame.rgba,
        frame.size,
        frame.size,
        image::ColorType::Rgba8,
    )
    .expect("save startup render evidence");
    std::fs::write(rt.data.join("startup-pass.json"), serde_json::json!({
        "version": crate::updates::VERSION, "backend": if cfg!(target_os = "macos") { "Metal" } else { "Vulkan" },
        "frames": frame.serial, "size": frame.size, "resources": rt.root, "settings_visible": rt.show_settings,
        "embedded_shu": cfg!(feature = "embedded-shu")
    }).to_string()).expect("save startup result");
    println!(
        "STARTUP PASS: {} rendered frames; {}px",
        frame.serial, frame.size
    );
    *done = true;
    exit.write(AppExit::Success);
}
