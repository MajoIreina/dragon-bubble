//! Headless import utility; shares the desktop application's transactional importer.
use std::path::PathBuf;
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let data = PathBuf::from(
        args.next()
            .ok_or_else(|| anyhow::anyhow!("usage: import <data-directory> <source>..."))?,
    );
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for source in args {
        let skin = pet_core::import::install(&PathBuf::from(source), &data, &project)?;
        println!("{}: {}", skin.id, skin.root.display());
    }
    Ok(())
}
