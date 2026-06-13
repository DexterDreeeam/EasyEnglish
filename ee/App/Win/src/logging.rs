//! Debug logging sink for the Windows overlay (timestamped file under C:\.ee).

#[cfg(debug_assertions)]
use std::sync::Mutex;

#[cfg(debug_assertions)]
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[cfg(debug_assertions)]
pub(crate) fn init_debug_logging() {
    let dir = std::path::Path::new("C:\\.ee");
    let _ = std::fs::create_dir_all(dir);

    unsafe {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut st = std::mem::zeroed::<SYSTEMTIME>();
        GetLocalTime(&mut st);

        let filename = format!(
            "easyenglish_{:04}{:02}{:02}_{:02}{:02}{:02}.log",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
        );
        let path = dir.join(filename);
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            *LOG_FILE.lock().unwrap() = Some(file);
        }
    }
}

#[cfg(debug_assertions)]
pub(crate) fn log_message(msg: &str) {
    println!("{}", msg);
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(file) = guard.as_mut() {
            use std::io::Write;
            let _ = writeln!(file, "{}", msg);
            let _ = file.flush();
        }
    }
}

#[cfg(not(debug_assertions))]
pub(crate) fn log_message(_msg: &str) {}
