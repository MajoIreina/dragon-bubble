//! Engine-independent desktop pet domain. No window, renderer, or Godot dependency.

pub mod action;
pub mod assets;
pub mod behavior;
pub mod desktop;
pub mod i18n;
pub mod import;
pub mod interaction;
mod model_import;
pub mod motion;
#[path = "../../../../crates/pet-core/src/preferences.rs"]
pub mod preferences;
pub mod skin;
pub mod soft_body;
pub mod sprite;
pub use glam::{Vec2, Vec3};
pub use rand as rand_support;
