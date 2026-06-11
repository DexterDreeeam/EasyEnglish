//! Build script: embeds the application icon into the Windows executable.
//!
//! The icon is compiled in as resource ID `1`, which makes it the executable's
//! default icon (shown by Explorer / the taskbar) and lets the tray code load
//! it at runtime via `LoadIconW(h_instance, MAKEINTRESOURCEW(1))`.

fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=installer/easyenglish.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon_with_id("installer/easyenglish.ico", "1");
        if let Err(e) = res.compile() {
            println!("cargo:warning=failed to embed application icon: {e}");
        }
    }
}
