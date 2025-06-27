// #![windows_subsystem = "windows"] // this prevents the gui?

use std::env;
use std::fs;
// Removed: use std::path::Path;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use rustc_hash::FxHashSet;
use windows::core::HSTRING;

use crate::panic::setup_panic_hook;

mod charging_events;
mod elevate;
mod exe_scan_2;
mod full_win_scan;
mod hide_console;
mod hstring_utils;
mod notification;
mod optimus;
mod panic;
mod prevent_duplicate;
mod registry;
mod winapp_scan;

fn load_exclusions() -> FxHashSet<String> {
    let mut exclusions = FxHashSet::default();
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let exclusion_file_path = dir.join("excluded_exes.txt");
            if exclusion_file_path.exists() {
                match fs::read_to_string(&exclusion_file_path) {
                    Ok(content) => {
                        for line in content.lines() {
                            let trimmed_line = line.trim();
                            if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
                                exclusions.insert(trimmed_line.to_lowercase());
                            }
                        }
                        println!("Loaded {} exclusion(s) from {}", exclusions.len(), exclusion_file_path.display());
                    }
                    Err(e) => {
                        eprintln!(
                            "Error reading {}: {}. Proceeding without exclusions.",
                            exclusion_file_path.display(),
                            e
                        );
                    }
                }
            } else {
                println!("{} not found. No EXEs will be excluded.", exclusion_file_path.display());
            }
        }
    }
    exclusions
}

fn load_specific_exes(filename: &str) -> FxHashSet<String> {
    let mut specific_exes = FxHashSet::default();
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let specific_file_path = dir.join(filename);
            if specific_file_path.exists() {
                match fs::read_to_string(&specific_file_path) {
                    Ok(content) => {
                        for line in content.lines() {
                            let trimmed_line = line.trim();
                            if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
                                specific_exes.insert(trimmed_line.to_lowercase());
                            }
                        }
                        println!(
                            "Loaded {} entry(s) from {}",
                            specific_exes.len(),
                            specific_file_path.display()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Error reading {}: {}. Proceeding without its entries.",
                            specific_file_path.display(),
                            e
                        );
                    }
                }
            } else {
                println!("{} not found. No specific EXEs will be loaded from this file.", specific_file_path.display());
            }
        }
    }
    specific_exes
}

fn load_excluded_dirs() -> Vec<String> {
    let mut excluded_dirs = Vec::new();
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let excluded_dirs_file_path = dir.join("excluded_dirs.txt");
            if excluded_dirs_file_path.exists() {
                match fs::read_to_string(&excluded_dirs_file_path) {
                    Ok(content) => {
                        for line in content.lines() {
                            let trimmed_line = line.trim();
                            if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
                                let mut normalized_path = trimmed_line.to_lowercase();
                                // Ensure consistent trailing slash for starts_with matching
                                if !normalized_path.ends_with('\\') && !normalized_path.ends_with('/') {
                                    normalized_path.push('\\');
                                }
                                // Replace forward slashes with backslashes for consistency on Windows
                                excluded_dirs.push(normalized_path.replace('/', "\\"));
                            }
                        }
                        println!(
                            "Loaded {} director(y/ies) to exclude from {}",
                            excluded_dirs.len(),
                            excluded_dirs_file_path.display()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Error reading {}: {}. Proceeding without directory exclusions.",
                            excluded_dirs_file_path.display(),
                            e
                        );
                    }
                }
            } else {
                println!("{} not found. No directories will be excluded.", excluded_dirs_file_path.display());
            }
        }
    }
    excluded_dirs
}

// These functions are called by charging_events, they need access to the exclusion/specific lists.
// A simple way is to have them load the lists themselves, or we refactor how they are called.
// For now, let's assume they will load them. This is slightly inefficient but avoids major refactoring
// of charging_events callback registration.
// A better long-term solution might involve Arc<Mutex<ConfigData>> passed around.

fn unplug_action() {
    println!("Power event: Switching to Integrated GPU (with overrides)");
    let exclusions = load_exclusions();
    let dedicated_exes = load_specific_exes("dedicated_exes.txt");
    let integrated_exes = load_specific_exes("integrated_exes.txt");
    let excluded_dirs = load_excluded_dirs();
    let programs = PROGRAMS.get().expect("Programs not initialized before power event handling");

    let res = unsafe {
        registry::write_reg(
            programs,
            registry::GpuMode::Integrated,
            &dedicated_exes,
            &integrated_exes,
            &exclusions,
            &excluded_dirs,
        )
    };
    match res {
        Ok(_) => {
            println!("Wrote to registry for integrated mode based on power event!");
            notification::toast(
                "Set global GPU to integrated GPU (with overrides).\nRestart programs to see changes.",
            )
            .unwrap();
        }
        Err(e) => {
            notification::toast("Error writing to registry for power event.").unwrap();
            dbg!(e);
        }
    }
}

fn plug_action() {
    println!("Power event: Switching to Dedicated GPU (with overrides)");
    let exclusions = load_exclusions();
    let dedicated_exes = load_specific_exes("dedicated_exes.txt");
    let integrated_exes = load_specific_exes("integrated_exes.txt");
    let excluded_dirs = load_excluded_dirs();
    let programs = PROGRAMS.get().expect("Programs not initialized before power event handling");

    let res = unsafe {
        registry::write_reg(
            programs,
            registry::GpuMode::Dedicated,
            &dedicated_exes,
            &integrated_exes,
            &exclusions,
            &excluded_dirs,
        )
    };
    match res {
        Ok(_) => {
            println!("Wrote to registry for dedicated mode based on power event!");
            notification::toast(
                "Set global GPU to dedicated GPU (with overrides).\nRestart programs to see changes.",
            )
            .unwrap();
        }
        Err(e) => {
            notification::toast("Error writing to registry for power event.").unwrap();
            dbg!(e);
        }
    }
}

fn plug_optimus() {
    let res = optimus::dedicated(); // Removed unnecessary unsafe block
    match res {
        Ok(_) => {
            println!("Changed to dedicated GPU!");
            notification::toast(
                "Set global GPU to dedicated GPU.\nRestart programs to see changes.",
            )
            .unwrap();
        }
        Err(e) => {
            notification::toast("Error changing to dedicated GPU.").unwrap();
            dbg!(e);
        }
    }
}
fn unplug_optimus() {
    let res = optimus::integrated(); // Removed unnecessary unsafe block
    match res {
        Ok(_) => {
            println!("Changed to integrated GPU!");
            notification::toast(
                "Set global GPU to integrated GPU.\nRestart programs to see changes.",
            )
            .unwrap();
        }
        Err(e) => {
            notification::toast("Error changing to integrated GPU.").unwrap();
            dbg!(e);
        }
    }
}

static PROGRAMS: OnceLock<Vec<HSTRING>> = OnceLock::new();

fn kill_duplicate() -> Result<bool> {
    unsafe { prevent_duplicate::kill_older_process() }
}

// set_programs now loads all programs without any initial filtering.
// Filtering and prioritization logic is handled by registry::write_reg.
fn set_programs() -> Result<()> {
    PROGRAMS
        .set(full_win_scan::get_all_programs()?) // Will be changed in full_win_scan.rs
        .map_err(|_| anyhow!("Failed to store program list."))
}

fn core(
    use_optimus: bool,
    // These are loaded in main and passed here to ensure they are available for power events
    // if set_programs is called. However, plug_action/unplug_action now load them directly.
    // Keeping them in the signature for now in case set_programs needs them, but it currently doesn't.
    _exclusions: &FxHashSet<String>,
    _dedicated_exes: &FxHashSet<String>,
    _integrated_exes: &FxHashSet<String>,
    _excluded_dirs: &Vec<String>, // Added excluded_dirs
) -> Result<()> {
    setup_panic_hook();
    kill_duplicate()?;
    if !use_optimus {
        // Ensure programs are scanned and available for plug_action/unplug_action
        if PROGRAMS.get().is_none() {
            set_programs()?;
        }
    }
    notification::register()?;
    // only hide console in release mode
    // RustRover's debug mode counts as an attached console and gets freed
    if cfg!(not(debug_assertions)) {
        unsafe { hide_console::hide_console()? }
    }
    if use_optimus {
        unsafe { charging_events::register_events(unplug_optimus, plug_optimus)? }
    } else {
        // Pass the new action functions that handle loading their own lists
        unsafe { charging_events::register_events(unplug_action, plug_action)? }
    }
    Ok(())
}

fn prog() -> Result<String> {
    // modified from https://stackoverflow.com/a/58113997/9044183
    env::current_exe()?
        .file_name()
        .ok_or(anyhow!("No file name"))?
        .to_os_string()
        .into_string()
        .map_err(|e| anyhow!("Failed to convert {e:?} to String"))
}

fn main() -> Result<()> {
    elevate::elevate_if_needed()?;
    // optimus::testing()?; // Commented out to prevent panic from incomplete UI automation
    let mut pargs = pico_args::Arguments::from_env();
    let use_optimus = pargs.contains(["-o", "--optimus"]);

    // Load exclusions early
    let exclusions = load_exclusions();
    let dedicated_exes = load_specific_exes("dedicated_exes.txt");
    let integrated_exes = load_specific_exes("integrated_exes.txt");
    let excluded_dirs = load_excluded_dirs();

    match pargs.subcommand()? {
        None => {
            elevate::elevate_if_needed()?;
            core(use_optimus, &exclusions, &dedicated_exes, &integrated_exes, &excluded_dirs)?;
        }
        Some(arg) => {
            match arg.as_str() {
                "shutdown" => {
                    elevate::elevate_if_needed()?;
                    let older_proc = kill_duplicate()?;
                    if !older_proc {
                        println!("No other instance of Win Global GPU found.")
                    }
                }
                "dedicated" => {
                    elevate::elevate_if_needed()?;
                    set_programs()?;
                    unsafe {
                        if use_optimus {
                            optimus::dedicated()?;
                        } else {
                            registry::write_reg(
                                PROGRAMS.get().unwrap(),
                                registry::GpuMode::Dedicated,
                                &dedicated_exes,
                                &integrated_exes,
                                &exclusions,
                                &excluded_dirs,
                            )?
                        }
                    };
                    println!("Set global GPU to dedicated GPU.");
                }
                "integrated" => {
                    elevate::elevate_if_needed()?;
                    set_programs()?;
                    unsafe {
                        if use_optimus {
                            optimus::integrated()?;
                        } else {
                            registry::write_reg(
                                PROGRAMS.get().unwrap(),
                                registry::GpuMode::Integrated,
                                &dedicated_exes,
                                &integrated_exes,
                                &exclusions,
                                &excluded_dirs,
                            )?
                        }
                    }
                    println!("Set global GPU to integrated GPU.");
                }
                "reset" => {
                    elevate::elevate_if_needed()?;
                    set_programs()?;
                    unsafe {
                        if use_optimus {
                            optimus::reset()?;
                        } else {
                            registry::write_reg(
                                PROGRAMS.get().unwrap_or(&vec![]),
                                registry::GpuMode::None,
                                &dedicated_exes,
                                &integrated_exes,
                                &exclusions,
                                &excluded_dirs,
                            )?
                        }
                    };
                    println!("Reset!");
                }
                // env!("CARGO_PKG_VERSION") is the cargo version
                // idk why its an env variable but whatever
                "help" => {
                    println!(
                        include_str!("../help.txt"),
                        env!("CARGO_PKG_VERSION"),
                        prog()?
                    )
                }
                "about" => {
                    println!(include_str!("../about.txt"), env!("CARGO_PKG_VERSION"))
                }
                err => {
                    eprintln!(
                        "Invalid argument `{err}`. Run `{} help` to see all commands.",
                        prog()?
                    )
                }
            }
        }
    }
    Ok(())
}
