fn main() {
    println!("cargo:rerun-if-changed=assets/logo/search-app-icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resources = winres::WindowsResource::new();

        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            let host = std::env::var("HOST").unwrap_or_default();
            let target = std::env::var("TARGET").unwrap_or_default();
            if host != target {
                let architecture = target.split('-').next().unwrap_or("x86_64");
                let tool_prefix = format!("{architecture}-w64-mingw32");
                resources.set_windres_path(&format!("{tool_prefix}-windres"));
                resources.set_ar_path(&format!("{tool_prefix}-ar"));
            }
        }

        resources.set_icon("assets/logo/search-app-icon.ico");
        resources.set("FileDescription", "Search");
        resources.set("ProductName", "Search");
        resources.set("OriginalFilename", "search.exe");

        if let Err(error) = resources.compile() {
            panic!("Could not embed the Search app icon in the Windows executable: {error}");
        }
    }
}
