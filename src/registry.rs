// HKEY_CURRENT_USER\Software\Microsoft\DirectX\UserGpuPreferences

use anyhow::Result;
use bytemuck;
use rustc_hash::FxHashSet;
use std::path::Path; // For filename extraction
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CommitTransaction, CreateTransaction, RollbackTransaction,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyTransactedW, RegDeleteTreeW, RegOpenKeyTransactedW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE, REG_SZ,
};

#[derive(PartialEq, Clone, Copy, Debug)] // Added Debug and Copy
pub enum GpuMode {
    Dedicated,
    Integrated,
    None, // Represents the mode for reset or when no global preference is set
}

// Helper to get filename from HSTRING path
// Note: This is a simplified version. Windows App paths might not be simple file paths.
// This will primarily work for EXEs from exe_scan_2.rs
fn get_filename_from_hstring(hstring_path: &HSTRING) -> Option<String> {
    let path_str = String::from_utf16_lossy(hstring_path.as_wide());
    Path::new(&path_str)
        .file_name()
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.to_lowercase())
}

pub unsafe fn write_reg(
    programs: &Vec<HSTRING>,
    global_mode: GpuMode,
    dedicated_exes: &FxHashSet<String>,
    integrated_exes: &FxHashSet<String>,
    excluded_exes: &FxHashSet<String>,
) -> Result<()> {
    let transaction = CreateTransaction(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        0,
        0,
        0,
        0,
        PCWSTR::null(),
    )?;
    match write_reg_transaction(
        transaction,
        programs,
        global_mode,
        dedicated_exes,
        integrated_exes,
        excluded_exes,
    ) {
        Ok(_) => {
            CommitTransaction(transaction)?;
            println!("Registry transaction committed successfully.");
            Ok(())
        }
        Err(e) => {
            RollbackTransaction(transaction)?;
            eprintln!("Registry transaction failed and rolled back: {}", e);
            Err(e)
        }
    }
}

pub unsafe fn write_reg_transaction(
    transaction: HANDLE,
    programs: &Vec<HSTRING>,
    global_mode: GpuMode,
    dedicated_exes: &FxHashSet<String>,
    integrated_exes: &FxHashSet<String>,
    excluded_exes: &FxHashSet<String>,
) -> Result<()> {
    let mut key = HKEY::default();
    match RegOpenKeyTransactedW(
        HKEY_CURRENT_USER,
        &HSTRING::from(r"Software\Microsoft\DirectX\UserGpuPreferences"),
        0,
        KEY_ALL_ACCESS,
        &mut key as *mut HKEY,
        transaction,
        None,
    ) {
        Ok(_) => {
            // Key exists, delete its current values to ensure a clean slate.
            // RegDeleteTreeW deletes subkeys and values but not the key itself if it has no subpath.
            // We want to clear all values under UserGpuPreferences.
            // It's often simpler to delete and recreate the key, or ensure it's empty.
            // For now, let's assume RegDeleteTreeW(key, None) clears values if key is UserGpuPreferences itself.
            // If it deletes the key itself, RegCreateKeyTransactedW will recreate it.
            println!("Registry: Clearing existing UserGpuPreferences values.");
            RegDeleteTreeW(key, PCWSTR::null())?; // Using PCWSTR::null() for current key's values
            RegCloseKey(key)?; // Close and reopen/recreate for a truly clean state or if DeleteTree deleted it
            // Re-create the key to ensure it exists cleanly
            RegCreateKeyTransactedW(
                HKEY_CURRENT_USER,
                &HSTRING::from(r"Software\Microsoft\DirectX\UserGpuPreferences"),
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
                None,
                &mut key as *mut HKEY,
                None,
                transaction,
                None,
            )?;
        }
        Err(e) => {
            if e == ERROR_FILE_NOT_FOUND.into() {
                println!("Registry: UserGpuPreferences key doesn't exist, creating.");
                RegCreateKeyTransactedW(
                    HKEY_CURRENT_USER,
                    &HSTRING::from(r"Software\Microsoft\DirectX\UserGpuPreferences"),
                    0,
                    None,
                    REG_OPTION_NON_VOLATILE,
                    KEY_ALL_ACCESS,
                    None,
                    &mut key as *mut HKEY,
                    None,
                    transaction,
                    None,
                )?;
            } else {
                return Err(e.into());
            }
        }
    }

    println!(
        "Registry: Applying rules. Global mode: {:?}, {} programs, {} dedicated, {} integrated, {} excluded.",
        global_mode,
        programs.len(),
        dedicated_exes.len(),
        integrated_exes.len(),
        excluded_exes.len()
    );

    for program_hstring_path in programs {
        let filename_option = get_filename_from_hstring(program_hstring_path);

        if filename_option.is_none() {
            // This might be a Windows Store app with a non-standard path, or other issue.
            // For now, these will only be affected by global_mode if they don't match simple filenames.
            // This matches previous behavior where only file EXEs were primarily targeted by exclusions.
            // If global_mode is None (reset), these are effectively ignored after clearing.
            if global_mode != GpuMode::None {
                 println!("Registry: Program path {} does not yield a simple filename. Applying global mode {:?}.", String::from_utf16_lossy(program_hstring_path.as_wide()), global_mode);
                let reg_value_str = match global_mode {
                    GpuMode::Dedicated => "GpuPreference=2;",
                    GpuMode::Integrated => "GpuPreference=1;",
                    GpuMode::None => continue, // Should not happen if global_mode != GpuMode::None check is there
                };
                let mut reg_value_u16: Vec<u16> = reg_value_str.encode_utf16().collect();
                reg_value_u16.push(0); // Null terminator
                RegSetValueExW(key, program_hstring_path, 0, REG_SZ, Some(bytemuck::cast_slice(&reg_value_u16)))?;
            } else {
                 println!("Registry: Program path {} does not yield a simple filename. Global mode is None, skipping.", String::from_utf16_lossy(program_hstring_path.as_wide()));
            }
            continue;
        }

        let filename = filename_option.unwrap(); // We know it's Some from check above.
        let prog_disp_name = String::from_utf16_lossy(program_hstring_path.as_wide());


        let is_in_dedicated = dedicated_exes.contains(&filename);
        let is_in_integrated = integrated_exes.contains(&filename);

        let mut effective_mode: Option<GpuMode> = None;

        if is_in_dedicated && is_in_integrated {
            // Conflict: Apply global_mode
            println!("Registry: EXE '{}' ({}) is in both dedicated and integrated lists. Applying global mode: {:?}.", filename, prog_disp_name, global_mode);
            if global_mode != GpuMode::None {
                effective_mode = Some(global_mode);
            }
        } else if is_in_dedicated {
            println!("Registry: EXE '{}' ({}) is in dedicated_exes.txt. Setting to Dedicated.", filename, prog_disp_name);
            effective_mode = Some(GpuMode::Dedicated);
        } else if is_in_integrated {
            println!("Registry: EXE '{}' ({}) is in integrated_exes.txt. Setting to Integrated.", filename, prog_disp_name);
            effective_mode = Some(GpuMode::Integrated);
        } else if excluded_exes.contains(&filename) {
            println!("Registry: EXE '{}' ({}) is in excluded_exes.txt. Skipping.", filename, prog_disp_name);
            // No effective_mode, so it remains cleared.
        } else {
            // Not in any specific list, apply global_mode
            if global_mode != GpuMode::None {
                 println!("Registry: EXE '{}' ({}) not in specific lists. Applying global mode: {:?}.", filename, prog_disp_name, global_mode);
                effective_mode = Some(global_mode);
            } else {
                 println!("Registry: EXE '{}' ({}) not in specific lists. Global mode is None (reset). Skipping.", filename, prog_disp_name);
            }
        }

        if let Some(mode_to_apply) = effective_mode {
            let reg_value_str = match mode_to_apply {
                GpuMode::Dedicated => "GpuPreference=2;",
                GpuMode::Integrated => "GpuPreference=1;",
                GpuMode::None => continue, // Should not happen given the logic paths to Some(effective_mode)
            };
            // Convert HSTRING to Vec<u16> (UTF-16 bytes for registry)
            // The value needs to be null-terminated.
            let mut reg_value_u16: Vec<u16> = reg_value_str.encode_utf16().collect();
            reg_value_u16.push(0); // Null terminator

            RegSetValueExW(key, program_hstring_path, 0, REG_SZ, Some(bytemuck::cast_slice(&reg_value_u16)))?;
        }
    }

    RegCloseKey(key)?;
    Ok(())
}
