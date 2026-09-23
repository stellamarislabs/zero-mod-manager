//! One brand for the window, taskbar and the executable. No shell-cache deletion,
//! pin/unpin operations, or changes to shortcuts happen at application startup.
use std::path::Path;
use tauri::Manager;

pub fn apply(
    window: &tauri::WebviewWindow,
    cache_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let icon = window
        .app_handle()
        .default_window_icon()
        .cloned()
        .ok_or_else(|| std::io::Error::other("Missing embedded application icon"))?;
    window.set_icon(icon)?;
    #[cfg(windows)]
    {
        let exe = std::env::current_exe()?;
        let icon_path = shell_icon(&exe, cache_dir)?;
        set_taskbar_properties(
            window.hwnd()?.0,
            &window.app_handle().config().identifier,
            &exe,
            &icon_path,
        )?;
    }
    #[cfg(not(windows))]
    let _ = cache_dir;
    Ok(())
}

#[cfg(windows)]
const ICON: &[u8] = include_bytes!("../icons/icon.ico");

#[cfg(windows)]
fn shell_icon(exe: &Path, cache_dir: &Path) -> std::io::Result<std::path::PathBuf> {
    use sha2::{Digest, Sha256};
    let name = format!(
        "zero-brand-{}.ico",
        &hex::encode(Sha256::digest(ICON))[..12]
    );
    let installed = exe.parent().unwrap_or(Path::new(".")).join(&name);
    if std::fs::read(&installed).is_ok_and(|bytes| bytes == ICON) {
        return Ok(installed);
    }
    // A single-file portable build has no resource directory. Keep its shell
    // icon in its isolated application cache, not beside an unwritable EXE.
    let directory = cache_dir.join("branding");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    if !std::fs::read(&path).is_ok_and(|bytes| bytes == ICON) {
        std::fs::write(&path, ICON)?;
    }
    Ok(path)
}

#[cfg(windows)]
fn set_taskbar_properties(
    hwnd: *mut std::ffi::c_void,
    app_id: &str,
    executable: &Path,
    icon_path: &Path,
) -> windows::core::Result<()> {
    use windows::Win32::{
        Foundation::HWND,
        Storage::EnhancedStorage::{
            PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
            PKEY_AppUserModel_RelaunchDisplayNameResource, PKEY_AppUserModel_RelaunchIconResource,
        },
        System::Com::StructuredStorage::PROPVARIANT,
        UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
    };
    // The property store owns copies of the values. Rust's PROPVARIANT drops
    // release their allocations; the COM interface releases on return.
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(HWND(hwnd))?;
        for (key, value) in [
            (
                PKEY_AppUserModel_RelaunchCommand,
                format!("\"{}\"", executable.display()),
            ),
            (
                PKEY_AppUserModel_RelaunchDisplayNameResource,
                "Zero Mod Manager".into(),
            ),
            (
                PKEY_AppUserModel_RelaunchIconResource,
                format!("{},0", icon_path.display()),
            ),
            (PKEY_AppUserModel_ID, app_id.into()),
        ] {
            store.SetValue(&key, &PROPVARIANT::from(value.as_str()))?;
        }
        store.Commit()?;
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn shell_resource_is_identical_for_installed_and_portable_modes() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("manager.exe");
        let cache = root.path().join("cache");
        let portable = shell_icon(&exe, &cache).unwrap();
        assert_eq!(std::fs::read(&portable).unwrap(), ICON);
        assert!(portable.starts_with(&cache));
        let installed = root.path().join(portable.file_name().unwrap());
        std::fs::write(&installed, ICON).unwrap();
        assert_eq!(shell_icon(&exe, &cache).unwrap(), installed);
        std::fs::write(&installed, b"stale or damaged resource").unwrap();
        assert_eq!(shell_icon(&exe, &cache).unwrap(), portable);
    }

    #[test]
    fn taskbar_properties_use_the_same_identity_icon_and_executable() {
        use windows::Win32::{
            Foundation::HWND,
            Storage::EnhancedStorage::{
                PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
                PKEY_AppUserModel_RelaunchDisplayNameResource,
                PKEY_AppUserModel_RelaunchIconResource,
            },
            UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow};
        struct TestWindow(*mut std::ffi::c_void);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe { DestroyWindow(self.0) };
            }
        }
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // Hidden test-only window; no user app, taskbar pin or shortcut changes.
        let window = TestWindow(unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        });
        assert!(!window.0.is_null());
        let exe = Path::new("C:\\Test folder\\Zero Mod Manager.exe");
        let icon = Path::new("C:\\Test folder\\zero-brand-test.ico");
        set_taskbar_properties(window.0, "app.zeromodmanager.desktop", exe, icon).unwrap();
        unsafe {
            let store: IPropertyStore = SHGetPropertyStoreForWindow(HWND(window.0)).unwrap();
            for (key, expected) in [
                (PKEY_AppUserModel_ID, "app.zeromodmanager.desktop"),
                (
                    PKEY_AppUserModel_RelaunchCommand,
                    "\"C:\\Test folder\\Zero Mod Manager.exe\"",
                ),
                (
                    PKEY_AppUserModel_RelaunchDisplayNameResource,
                    "Zero Mod Manager",
                ),
                (
                    PKEY_AppUserModel_RelaunchIconResource,
                    "C:\\Test folder\\zero-brand-test.ico,0",
                ),
            ] {
                assert_eq!(store.GetValue(&key).unwrap().to_string(), expected);
            }
        }
    }
}
