// Removed: use std::path;

use anyhow::Result; // anyhow import for 'anyhow!' macro removed, Result kept.
use uiautomation::actions::Window;
use uiautomation::controls::WindowControl;
use uiautomation::{UIAutomation, UIElement};
// Removed: use windows::core::HSTRING;
// Removed: use windows::Management::Deployment::PackageManager;

/* // Commenting out unused function for now
fn get_control_panel_winapp_dir() -> Result<HSTRING> {
    let manager = PackageManager::new()?;
    if let Some(i) = manager
        .FindPackagesByPackageFamilyName(&HSTRING::from(
            "NVIDIACorp.NVIDIAControlPanel_56jybvy8sckqj",
        ))?
        .into_iter()
        .next()
    {
        return Ok(i.InstalledPath()?);
    }
    Err(anyhow!("Control Panel not found"))
}
*/

/* // Commenting out unused function for now
fn get_control_panel_path() -> Result<String> {
    const PATHS: [&str; 2] = [
        r"C:\Program Files\NVIDIA Corporation\Control Panel Client\nvcplui.exe",
        r"C:\Program Files\NVIDIA Corporation\Control Panel Client\nvcplui64.exe",
    ];
    for path in PATHS {
        if path::Path::new(&path).exists() {
            return Ok(path.to_string());
        }
    }
    let winapp_dir = get_control_panel_winapp_dir()?; // This would now error if uncommented due to above commenting
    const EXE_NAMES: [&str; 2] = ["nvcplui.exe", "nvcplui64.exe"];
    for exe_name in EXE_NAMES {
        let exe_path = format!("{}\\{}", winapp_dir, exe_name);
        if path::Path::new(&exe_path).exists() {
            return Ok(exe_path);
        }
    }
    Err(anyhow!("Control Panel not found"))
}
*/

pub fn testing() -> Result<()> {
    let automation = UIAutomation::new().unwrap();
    let matcher = automation
        .create_matcher()
        .filter_fn(Box::new(|_e: &UIElement| todo!())) // Prefixed e with _
        .timeout(0);
    let _element = matcher.find_first(); // Prefixed element with _
    if let Ok(notepad) = matcher.find_first() {
        println!(
            "Found: {} - {}",
            notepad.get_name().unwrap(),
            notepad.get_classname().unwrap()
        );

        let window: WindowControl = notepad.try_into().unwrap();
        window.maximize().unwrap();
    }
    Ok(())
}

pub fn integrated() -> Result<()> {
    todo!()
}
pub fn dedicated() -> Result<()> {
    todo!()
}
pub fn reset() -> Result<()> {
    todo!()
}
