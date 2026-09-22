use std::{io, path::Path, process::Command};

/// Only a Windows elevation-required error may trigger one UAC request.
fn finish_launch(
    result: io::Result<()>,
    windows: bool,
    elevate: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    match result {
        Err(error) if windows && error.raw_os_error() == Some(740) => elevate(),
        result => result,
    }
}

pub fn launch(executable: &Path, directory: &Path) -> io::Result<()> {
    let result = Command::new(executable)
        .current_dir(directory)
        .spawn()
        .map(|_| ());
    finish_launch(result, cfg!(windows), || elevate(executable, directory))
}

pub fn error_message(error: &io::Error) -> String {
    if cfg!(windows) && error.raw_os_error() == Some(1223) {
        "Launch cancelled. Windows administrator approval was declined. The game was not started."
            .into()
    } else {
        format!("The game could not be launched: {error}")
    }
}

#[cfg(not(windows))]
fn elevate(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Elevation is Windows-only",
    ))
}

#[cfg(windows)]
fn elevate(executable: &Path, directory: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    fn wide(value: &std::ffi::OsStr) -> io::Result<Vec<u16>> {
        let mut result: Vec<u16> = value.encode_wide().collect();
        if result.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid executable path",
            ));
        }
        result.push(0);
        Ok(result)
    }
    let file = wide(executable.as_os_str())?;
    let cwd = wide(directory.as_os_str())?;
    // A dedicated STA avoids changing Tauri's COM apartment. No shell command
    // strings, arguments, registry writes, or elevation of the manager itself.
    std::thread::spawn(move || unsafe {
        let hr = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        if hr < 0 {
            return Err(io::Error::other(format!(
                "Windows launch initialization failed: {hr:#x}"
            )));
        }
        let verb: Vec<u16> = "runas\0".encode_utf16().collect();
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpDirectory = cwd.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        // Buffers stay alive until this synchronous call returns. No process
        // handle is requested. FLAG_NO_UI suppresses errors, not the UAC prompt.
        let result = if ShellExecuteExW(&mut info) == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        };
        CoUninitialize();
        result
    })
    .join()
    .map_err(|_| io::Error::other("Windows launch worker failed"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normal_launch_never_requests_elevation() {
        finish_launch(Ok(()), true, || panic!("unexpected UAC")).unwrap();
    }
    #[test]
    fn only_elevation_required_can_request_uac() {
        for code in [2, 5, 193, 1223] {
            assert_eq!(
                finish_launch(Err(io::Error::from_raw_os_error(code)), true, || panic!(
                    "unexpected UAC"
                ))
                .unwrap_err()
                .raw_os_error(),
                Some(code)
            );
        }
        assert!(
            finish_launch(Err(io::Error::from_raw_os_error(740)), false, || panic!(
                "non-Windows UAC"
            ))
            .is_err()
        );
    }
    #[test]
    fn elevation_approval_succeeds_and_cancel_is_not_retried() {
        finish_launch(Err(io::Error::from_raw_os_error(740)), true, || Ok(())).unwrap();
        let error = finish_launch(Err(io::Error::from_raw_os_error(740)), true, || {
            Err(io::Error::from_raw_os_error(1223))
        })
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(1223));
        #[cfg(windows)]
        assert!(error_message(&error).contains("cancelled"));
    }
}
