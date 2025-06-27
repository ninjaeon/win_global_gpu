use crate::exe_scan_2;
use crate::winapp_scan;
use anyhow::Result;
// No longer need FxHashSet or Path here as filtering is moved
use windows::core::HSTRING;

// The is_excluded function is removed from this file.
// All filtering and prioritization logic will be handled in registry.rs.

pub fn get_all_programs() -> Result<Vec<HSTRING>> {
    // Get Windows Store apps
    let mut programs = winapp_scan::find_windows_apps()?;
    let win_app_count = programs.len();
    println!(
        "Scan: Found {} Windows Store app(s).",
        win_app_count
    );

    // Get EXE files from filesystem
    let mut files = unsafe { exe_scan_2::get_all_files()? };
    let exe_count = files.len();
    println!("Scan: Found {} EXE file(s) on the filesystem.", exe_count);

    // Combine the lists
    programs.append(&mut files);

    println!(
        "Scan: Total programs found (before any filtering/prioritization): {}.",
        programs.len()
    );
    Ok(programs)
}
