fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Sunline");
        res.set("FileDescription", "Sunline - launcher for VIctory");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon not embedded: {e}");
        }
    }
}
