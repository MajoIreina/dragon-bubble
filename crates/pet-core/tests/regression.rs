use pet_core::{
    Vec2, Vec3, action, assets,
    behavior::State,
    desktop::{Desktop, Rect},
    motion::Motion,
    preferences::Preferences,
};
use serde_json::json;
use std::{collections::HashMap, fs, path::PathBuf};
fn project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[test]
fn windows_catch_descending_feet_and_release_when_removed() {
    use pet_core::desktop::Surface;
    let mut desktop = Desktop {
        extent: Vec2::new(200., 160.),
        surfaces: vec![Surface {
            id: 1,
            window_left: 200.,
            left: 200.,
            right: 800.,
            y: 500.,
        }],
        ..Default::default()
    };
    assert!(
        desktop
            .crossed_surface(Vec2::new(300., 400.), Vec2::new(300., 300.))
            .is_none()
    );
    assert!(
        desktop
            .crossed_surface(Vec2::new(900., 200.), Vec2::new(900., 500.))
            .is_none()
    );
    let mut motion = Motion::default();
    motion.launch(Vec2::new(300., 100.), Vec2::ZERO);
    for _ in 0..300 {
        motion.advance(
            1. / 120.,
            desktop.bounds_for(motion.position),
            Some(&desktop),
        );
    }
    assert!(!motion.active);
    assert!((motion.position.y - 340.).abs() < 0.01);
    assert_eq!(desktop.support(motion.position).unwrap().id, 1);
    desktop.surfaces.clear();
    motion.launch(motion.position, Vec2::ZERO);
    for _ in 0..500 {
        motion.advance(
            1. / 120.,
            desktop.bounds_for(motion.position),
            Some(&desktop),
        );
    }
    assert!(!motion.active);
    assert_eq!(motion.position.y, 920.);
}
#[test]
fn zoom_300_percent_uses_visible_top_and_keeps_negative_monitor_coordinates() {
    let desktop = Desktop {
        areas: vec![Rect::new(-1920., -1080., 1920., 1080.)],
        extent: Vec2::new(960., 744.),
        top_inset: 240.,
        ..Default::default()
    };
    let top = desktop.clamp_origin(Vec2::new(-1500., -5000.));
    assert_eq!(top.y + desktop.top_inset, -1080.);
    assert_eq!(top.x, -1500.);
    let mut motion = Motion::default();
    motion.launch(top, Vec2::new(0., -1000.));
    motion.advance(1. / 60., desktop.bounds_for(top), Some(&desktop));
    assert!(motion.position.y >= -1320.);
}
#[test]
fn behavior_settings_migrate_and_manual_sleep_waits_for_wake() {
    let prefs = Preferences::from_value(
        &json!({"behavior":{"idle":-1,"walk":999,"sleep":90,"idle_spin":false}}),
    );
    assert_eq!(prefs.behavior.idle, 3.);
    assert_eq!(prefs.behavior.walk, 30.);
    assert_eq!(prefs.behavior.sleep, 90.);
    assert!(!prefs.behavior.idle_spin);
    assert!(prefs.behavior.window_perching);
    let roundtrip = Preferences::from_value(&serde_json::to_value(&prefs).unwrap());
    assert_eq!(prefs, roundtrip);
    let mut brain = pet_core::behavior::Brain::default();
    brain.sleep_now();
    brain.tick(1000., &mut pet_core::rand_support::rng());
    assert_eq!(brain.state, State::Sleep);
    brain.wake();
    assert_eq!(brain.state, State::Idle);
    assert!(!brain.forced_sleep);
}
#[test]
fn profiles_preserve_explicit_choice_and_detect_character_or_sprite() {
    let round = Vec3::ONE;
    let bones = vec!["mixamorig:Hips".into(), "Head".into()];
    assert_eq!(
        action::profile_for("model3d", round, &bones, "auto"),
        "character"
    );
    assert_eq!(
        action::profile_for("model3d", round, &bones, "blob"),
        "blob"
    );
    assert_eq!(action::profile_for("sprite", round, &[], "auto"), "flat");
    assert_eq!(
        action::profile_for("model3d", Vec3::new(1., 2., 1.), &[], "auto"),
        "character"
    );
    assert_eq!(action::profile_for("model3d", round, &[], "auto"), "blob");
}
#[test]
fn multiscreen_negative_seams_steps_and_gravity() {
    let mut d = Desktop {
        areas: vec![
            Rect::new(-1280., 0., 1280., 984.),
            Rect::new(0., 0., 1920., 1040.),
        ],
        extent: Vec2::new(200., 160.),
        ..Default::default()
    };
    assert_eq!(d.screen_at(Vec2::new(-600., 50.)), 0);
    assert_eq!(d.bounds_for(Vec2::new(-300., 500.)).max.x, 1720.);
    let mut m = Motion::default();
    m.launch(Vec2::new(-240., 400.), Vec2::new(1300., -200.));
    for _ in 0..40 {
        m.advance(1. / 120., d.bounds_for(m.position), Some(&d));
    }
    assert!(m.position.x > 0. && m.velocity.x > 0.);
    for _ in 0..1200 {
        m.advance(1. / 120., d.bounds_for(m.position), Some(&d));
    }
    assert!(!m.active);
    assert!((m.position.y - 880.).abs() < 0.01);
    assert_eq!(d.bounds_for(Vec2::new(20., 880.)).min.x, 0.);
    assert_eq!(d.bounds_for(Vec2::new(20., 500.)).min.x, -1280.);
    d.areas = vec![
        Rect::new(0., -900., 1600., 900.),
        Rect::new(0., 0., 1600., 1000.),
    ];
    m.launch(Vec2::new(400., -200.), Vec2::new(0., 500.));
    for _ in 0..1200 {
        m.advance(1. / 120., d.bounds_for(m.position), Some(&d));
    }
    assert!(!m.active);
    assert!((m.position.y - 840.).abs() < 0.01);
}
#[test]
fn disconnected_hotplug_and_oversize() {
    let mut d = Desktop {
        areas: vec![
            Rect::new(0., 0., 1000., 800.),
            Rect::new(1200., 200., 800., 800.),
        ],
        extent: Vec2::new(200., 160.),
        ..Default::default()
    };
    assert_eq!(d.bounds_for(Vec2::new(750., 100.)).max.x, 800.);
    assert_eq!(d.clamp_origin(Vec2::new(1050., 400.)).x, 1200.);
    d.areas = vec![Rect::new(0., 0., 1920., 1040.)];
    assert_eq!(d.clamp_origin(Vec2::new(-1500., -500.)), Vec2::ZERO);
    d.extent = Vec2::new(2500., 1500.);
    assert_eq!(d.clamp_origin(Vec2::new(300., 400.)), Vec2::ZERO);
}
#[test]
fn throw_sampling_pause_cap_and_interrupt() {
    let mut m = Motion::default();
    m.begin_drag(Vec2::ZERO, 1.);
    m.record_drag(Vec2::new(1000., 0.), 1.02);
    assert!((m.release_velocity(1.02).length() - 2600.).abs() < 0.01);
    assert_eq!(m.release_velocity(2.), Vec2::ZERO);
    m.launch(Vec2::ZERO, Vec2::new(100., -200.));
    m.begin_drag(Vec2::new(20., 30.), 3.);
    assert!(!m.active);
    assert_eq!(m.velocity, Vec2::ZERO);
    assert_eq!(m.release_velocity(3.), Vec2::ZERO);
}
#[test]
fn floor_friction_and_wall_bounce() {
    let b = Rect::new(0., 0., 1000., 800.);
    let mut m = Motion::default();
    m.launch(Vec2::new(999., 200.), Vec2::new(1000., 0.));
    m.advance(0.05, b, None);
    assert!(m.velocity.x < 0.);
    for _ in 0..1000 {
        m.advance(1. / 120., b, None);
    }
    assert!(!m.active);
    assert_eq!(m.position.y, 800.);
    assert!(m.position.is_finite());
}
#[test]
fn actions_manual_disable_and_deterministic_aliases() {
    let clips = vec![
        "Z|Walk".into(),
        "A|Walk".into(),
        "Idle2".into(),
        "RESET".into(),
        "brunch".into(),
    ];
    let auto = action::resolve(&clips, &HashMap::new());
    assert_eq!(auto["walk"], "A|Walk");
    assert_eq!(auto["run"], "A|Walk");
    assert_eq!(auto["idle"], "Idle2");
    let manual = HashMap::from([
        ("walk".into(), "".into()),
        ("idle".into(), "nonexistent".into()),
    ]);
    let mapped = action::resolve(&clips, &manual);
    assert!(!mapped.contains_key("walk"));
    assert!(!mapped.contains_key("idle"));
    assert_eq!(mapped["run"], "A|Walk");
    assert!(!action::resolve(&["Outrun".into()], &HashMap::new()).contains_key("run"));
}
#[test]
fn tolerant_preferences_and_atomic_replace() {
    let p = Preferences::from_value(
        &json!({"skin_id":"builtin.mochi","zoom":99,"roaming":"bad","position":[-1200,50],"locale":"ja-JP","performance":{"fps":9999,"resolution":768,"aa":"taa"}}),
    );
    assert_eq!(p.skin_id, "builtin.shu");
    assert_eq!(p.zoom, 3.);
    assert!(p.roaming);
    assert_eq!(p.locale, "ja");
    assert_eq!(p.performance.fps, 60);
    assert_eq!(p.performance.resolution, 768);
    assert_eq!(p.performance.aa, "taa");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("preferences.json");
    p.save(&file).unwrap();
    let mut p2 = p.clone();
    p2.zoom = 0.2;
    p2.save(&file).unwrap();
    assert_eq!(Preferences::load(&file), p2);
}
#[test]
fn builtins_validate_and_sprite_decodes() {
    let (skins, errors) = assets::scan(&[project().join("skins")]);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(skins.len() >= 5);
    for skin in skins {
        if skin.r#type == "sprite" {
            let pack = pet_core::sprite::SpritePack::load(&skin).unwrap();
            assert!(!pack.frames.is_empty());
            assert!(pack.ground_offset > 146.);
            for state in State::ALL {
                assert!(pack.frame_at(state, 100., &skin.animations) < pack.frames.len());
            }
        }
    }
}
#[test]
fn paths_and_import_rollback() {
    for path in ["../a", "/a", "C:/a", "a/../b", "a\\b", "a//b", "."] {
        assert!(!assets::safe_relative(path), "{path}");
    }
    let source = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let file = source.path().join("bad.png");
    fs::write(&file, b"not a picture").unwrap();
    assert!(pet_core::import::install(&file, user.path(), &project()).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"not a picture");
    assert_eq!(fs::read_dir(user.path().join("skins")).unwrap().count(), 0);
}
#[test]
fn sprite_import_and_duplicate_are_transactional() {
    let source = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let file = source.path().join("image.png");
    image::RgbaImage::from_pixel(8, 12, image::Rgba([200, 100, 50, 255]))
        .save(&file)
        .unwrap();
    let skin = pet_core::import::install(&file, user.path(), &project()).unwrap();
    assert_eq!(
        pet_core::sprite::SpritePack::load(&skin).unwrap().size,
        [8, 12]
    );
    let bytes = fs::read(skin.root.join("skin.json")).unwrap();
    assert!(
        pet_core::import::install(&skin.root.join("skin.json"), user.path(), &project()).is_err()
    );
    assert_eq!(fs::read(skin.root.join("skin.json")).unwrap(), bytes);
}
#[test]
fn soft_body_floor_and_finite_springs() {
    let def = pet_core::soft_body::Definition {
        node: "Body".into(),
        profile_mesh: "Shell".into(),
        ..Default::default()
    };
    let points = (0..30)
        .flat_map(|i| {
            [
                Vec3::new(0., -1., -2.3 * i as f32 / 29.),
                Vec3::new(0., 1., -2.3 * i as f32 / 29.),
            ]
        })
        .collect::<Vec<_>>();
    let mut body = pet_core::soft_body::SoftBody::sample(def, &points).unwrap();
    for p in &points {
        body.include_vertex(*p);
    }
    for _ in 0..1000 {
        body.advance(
            1. / 120.,
            Vec3::ZERO,
            Vec3::new(1., -1., 1.),
            1800.,
            true,
            0.2,
        );
    }
    for p in points {
        let deformed = body.deform_point(p);
        assert!(deformed.is_finite());
        assert!(deformed.y >= -1.011);
    }
}
#[test]
fn gltf_dependencies_contained() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.gltf");
    fs::write(
        &file,
        serde_json::to_vec(
            &json!({"asset":{"version":"2.0"},"buffers":[{"uri":"../escape.bin","byteLength":4}]}),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(pet_core::import::repack_gltf(&file, &dir.path().join("out.glb")).is_err());
}
#[test]
fn translations_preserved() {
    assert_eq!(pet_core::i18n::tr("en", "待机"), "Idle");
    assert_ne!(pet_core::i18n::tr("ja", "待机"), "待机");
}

#[test]
fn continuous_render_settings_survive_save_and_reload() {
    let path = std::env::temp_dir().join(format!("lpp-sliders-{}.json", std::process::id()));
    let mut p = Preferences::default();
    p.performance.resolution = 816;
    p.performance.fps = 91;
    p.save(&path).unwrap();
    assert_eq!(Preferences::load(&path), p);
    for fps in [0, 24, 240] {
        assert_eq!(
            pet_core::preferences::RenderOptions::from_value(&json!({"fps":fps})).fps,
            fps
        );
    }
    for value in [json!(-1), json!(241), json!(60.5)] {
        assert_eq!(
            pet_core::preferences::RenderOptions::from_value(&json!({"fps":value})).fps,
            60
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn soft_body_unloads_during_freefall_and_hanging() {
    let def = pet_core::soft_body::Definition {
        node: "Body".into(),
        profile_mesh: "Shell".into(),
        ..Default::default()
    };
    let vertices = [Vec3::new(0., -1., 0.), Vec3::new(0., 1., -2.3)];
    let supported = pet_core::soft_body::SoftBody::sample(def, &vertices).unwrap();
    let mut free = supported.clone();
    let mut held = supported.clone();
    held.suspension = 1.;
    for _ in 0..480 {
        free.advance(
            1. / 120.,
            Vec3::new(0., 1800., 0.),
            Vec3::ZERO,
            1800.,
            false,
            0.,
        );
        held.advance(1. / 120., Vec3::ZERO, Vec3::ZERO, 1800., false, 0.);
    }
    assert!(
        free.load < 0.001,
        "freefall must remove support compression"
    );
    assert!(held.load > 0.2 && held.load < 0.35);
    assert!(held.deform_point(Vec3::new(0., 0., -2.3)).y < 0.);
    held.suspension = 0.;
    for _ in 0..480 {
        held.advance(1. / 120., Vec3::ZERO, Vec3::ZERO, 1800., true, 0.);
    }
    assert!((held.load - supported.load).abs() < 0.001);
}
