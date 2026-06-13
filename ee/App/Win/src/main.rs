#![windows_subsystem = "windows"]

//! Binary entrypoint for the EasyEnglish Windows Platform App.

fn main() {
    #[cfg(target_os = "windows")]
    {
        // Enforce single instance check using a named global mutex
        unsafe {
            use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
            use windows_sys::Win32::System::Threading::CreateMutexW;

            let mutex_name = "Global\\EasyEnglishSingleInstanceMutex\0"
                .encode_utf16()
                .collect::<Vec<u16>>();
            let handle = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
            if handle == 0 {
                write_startup_failure("CreateMutexW failed");
                std::process::exit(1);
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                // Another instance is already running: a Start Menu launch should
                // wake the resident tray process rather than exiting silently.
                let _ = ee_win::wake_existing_instance();
                std::process::exit(0);
            }
            // Keep the mutex handle alive for the lifetime of this process
            let _keep_alive_mutex = handle;
        }

        let show_on_start = std::env::args().any(|arg| arg == "--show");

        println!("Initializing EasyEnglish Windows Search Overlay...");
        if let Err(e) = ee_win::run(show_on_start) {
            write_startup_failure(&format!("Fatal error running Windows App: {e}"));
            eprintln!("Fatal error running Windows App: {}", e);
            std::process::exit(1);
        }
    }

    #[cfg(target_os = "windows")]
    fn write_startup_failure(message: &str) {
        let _ = std::fs::create_dir_all("C:\\.ee");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:\\.ee\\crash.txt")
        {
            use std::io::Write;
            let _ = writeln!(f, "{message}");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("EasyEnglish Windows Search Overlay placeholder. Only supported on Windows target hosts.");
    }
}
