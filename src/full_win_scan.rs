use crate::exe_scan_2;
use crate::winapp_scan;
use anyhow::Result;
use rustc_hash::FxHashSet;
use std::path::Path;
use windows::core::HSTRING;

// Helper function to check if a given HSTRING path's filename is in the exclusion list
fn is_excluded(hstring_path: &HSTRING, exclusions: &FxHashSet<String>) -> bool {
    // Convert HSTRING to String.
    // HSTRINGs from exe_scan_2 are expected to be valid file paths.
    let path_str = String::from_utf16_lossy(hstring_path.as_wide());

    if let Some(filename_osstr) = Path::new(&path_str).file_name() {
        if let Some(filename_str) = filename_osstr.to_str() {
            // Compare with lowercase version from exclusions set
            return exclusions.contains(&filename_str.to_lowercase());
        }
    }
    // If path is malformed or filename can't be extracted/converted, don't exclude.
    false
}

pub fn get_all_programs(exclusions: &FxHashSet<String>) -> Result<Vec<HSTRING>> {
    // Windows Store apps are not currently subject to the exclusion list.
    let mut programs = winapp_scan::find_windows_apps()?;
    let initial_win_app_count = programs.len();
    println!("Found {} Windows Store apps.", initial_win_app_count);

    let mut files = unsafe { exe_scan_2::get_all_files()? };
    let initial_exe_count = files.len();

    files.retain(|hstring_path| !is_excluded(hstring_path, exclusions));
    let excluded_count = initial_exe_count - files.len();
    if excluded_count > 0 {
        println!("Excluded {} EXE(s) based on excluded_exes.txt.", excluded_count);
    }

    programs.append(&mut files);
    println!(
        "Total programs to manage: {} ({} Windows Store apps, {} EXEs after exclusions).",
        programs.len(),
        initial_win_app_count,
        files.len()
    );
    Ok(programs)
}
