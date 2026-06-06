#![windows_subsystem = "windows"]

//! Binary entrypoint for the EasyEnglish Windows Platform App.

fn main() {
    #[cfg(target_os = "windows")]
    {
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
