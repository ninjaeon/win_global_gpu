// #![windows_subsystem = "windows"] // this prevents the gui?

use std::env;
use std::fs;
use std::path::Path;
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

fn unplug() {
    let res =
        unsafe { registry::write_reg(PROGRAMS.get().unwrap(), registry::GpuMode::Integrated) };
    match res {
        Ok(_) => {
            println!("Wrote to registry!");
            notification::toast(
                "Set global GPU to integrated GPU.\nRestart programs to see changes.",
            )
            .unwrap();
        }
        Err(e) => {
            notification::toast("Error writing to registry.").unwrap();
            dbg!(e);
        }
    }
}
fn plug() {
    let res = unsafe { registry::write_reg(PROGRAMS.get().unwrap(), registry::GpuMode::Dedicated) };
    match res {
        Ok(_) => {
            println!("Wrote to registry!");
            notification::toast(
                "Set global GPU to dedicated GPU.\nRestart programs to see changes.",
            )
            .unwrap();
        }
        Err(e) => {
            notification::toast("Error writing to registry.").unwrap();
            dbg!(e);
        }
    }
}
fn plug_optimus() {
    let res = unsafe { optimus::dedicated() };
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
    let res = unsafe { optimus::integrated() };
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
fn set_programs(exclusions: &FxHashSet<String>) -> Result<()> {
    PROGRAMS
        .set(full_win_scan::get_all_programs(exclusions)?)
        .map_err(|_| anyhow!("Failed to store program list."))
}
fn core(use_optimus: bool, exclusions: &FxHashSet<String>) -> Result<()> {
    setup_panic_hook();
    kill_duplicate()?;
    if !use_optimus {
        set_programs(exclusions)?;
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
        unsafe { charging_events::register_events(unplug, plug)? }
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
    optimus::testing()?;
    let mut pargs = pico_args::Arguments::from_env();
    let use_optimus = pargs.contains(["-o", "--optimus"]);

    // Load exclusions early
    let exclusions = load_exclusions();

    match pargs.subcommand()? {
        None => {
            elevate::elevate_if_needed()?;
            core(use_optimus, &exclusions)?;
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
                    set_programs(&exclusions)?;
                    unsafe {
                        if use_optimus {
                            optimus::dedicated()?;
                        } else {
                            registry::write_reg(
                                PROGRAMS.get().unwrap(),
                                registry::GpuMode::Dedicated,
                            )?
                        }
                    };
                    println!("Set global GPU to dedicated GPU.");
                }
                "integrated" => {
                    elevate::elevate_if_needed()?;
                    set_programs(&exclusions)?;
                    unsafe {
                        if use_optimus {
                            optimus::integrated()?;
                        } else {
                            registry::write_reg(
                                PROGRAMS.get().unwrap(),
                                registry::GpuMode::Integrated,
                            )?
                        }
                    }
                    println!("Set global GPU to integrated GPU.");
                }
                "reset" => {
                    elevate::elevate_if_needed()?;
                    unsafe {
                        if use_optimus {
                            optimus::reset()?;
                        } else {
                            registry::write_reg(&vec![], registry::GpuMode::None)?
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
