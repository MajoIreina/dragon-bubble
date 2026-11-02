fn main() {
    println!("cargo:rerun-if-changed=../../assets/tray.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../../assets/tray.ico")
            .set("ProductName", "My Dragon Bubble")
            .set(
                "FileDescription",
                "My Dragon Bubble · Rust desktop companion",
            );
        resource.compile().expect("Windows icon resource");
    }
}
