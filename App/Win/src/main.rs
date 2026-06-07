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
                std::process::exit(1);
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                // Another instance is already running! Exit immediately.
                std::process::exit(0);
            }
            // Keep the mutex handle alive for the lifetime of this process
            let _keep_alive_mutex = handle;
        }

        println!("Initializing EasyEnglish Windows Search Overlay...");
        if let Err(e) = ee_win::run() {
            eprintln!("Fatal error running Windows App: {}", e);
            std::process::exit(1);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("EasyEnglish Windows Search Overlay placeholder. Only supported on Windows target hosts.");
    }
}
