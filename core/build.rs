fn main() {
    // Embed the app icon + version info so the daemon exe looks right in
    // Task Manager, autostart listings and Explorer.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../ui/src-tauri/icons/icon.ico");
        res.set("ProductName", "Keyscape");
        res.set("FileDescription", "Keyscape lighting core");
        res.set("LegalCopyright", "MIT (c) 2026 Keyscape contributors");
        res.compile().expect("embed windows resources");
    }
}
