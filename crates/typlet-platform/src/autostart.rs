// SPDX-License-Identifier: GPL-3.0-or-later
//! Starting Typlet when the user logs in.
//!
//! * Windows: a value in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//!   (no admin rights needed).
//! * Linux: a desktop entry in `~/.config/autostart`.

use std::path::Path;

use crate::PlatformError;

/// Command-line flag passed when started automatically.
pub const AUTOSTART_FLAG: &str = "--autostart";

/// Enables or disables starting `executable` at login.
///
/// # Errors
///
/// Fails if the autostart entry cannot be written or removed.
pub fn set(enabled: bool, executable: &Path) -> Result<(), PlatformError> {
    imp::set(enabled, executable)
}

/// Returns `true` if the autostart entry exists.
#[must_use]
pub fn is_enabled() -> bool {
    imp::is_enabled()
}

#[cfg(target_os = "linux")]
mod imp {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::AUTOSTART_FLAG;
    use crate::PlatformError;

    fn entry_path() -> Option<PathBuf> {
        directories::BaseDirs::new()
            .map(|d| d.config_dir().join("autostart").join("typlet.desktop"))
    }

    /// The command to start: inside an AppImage the binary path changes on
    /// every start, so the AppImage file itself is used.
    fn command(executable: &Path) -> String {
        let target = std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .unwrap_or_else(|| executable.to_path_buf());
        format!("\"{}\" {AUTOSTART_FLAG}", target.display())
    }

    pub(super) fn set(enabled: bool, executable: &Path) -> Result<(), PlatformError> {
        let path = entry_path().ok_or_else(|| PlatformError::Other("no home folder".into()))?;
        if enabled {
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=Typlet\nComment=Text expander\nExec={}\nIcon=typlet-editor\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
                command(executable)
            );
            typlet_core::fsutil::write_atomic(&path, entry.as_bytes())?;
        } else if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub(super) fn is_enabled() -> bool {
        entry_path().is_some_and(|p| p.exists())
    }
}

#[cfg(windows)]
mod imp {
    use std::path::Path;

    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
        RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows::core::{HSTRING, PCWSTR, w};

    use super::AUTOSTART_FLAG;
    use crate::PlatformError;

    const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
    const VALUE: PCWSTR = w!("Typlet");

    fn open(access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
        let mut key = HKEY::default();
        // SAFETY: valid constant strings and an out-pointer to a local.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, access, &raw mut key) };
        (status == ERROR_SUCCESS).then_some(key)
    }

    pub(super) fn set(enabled: bool, executable: &Path) -> Result<(), PlatformError> {
        let key = open(KEY_SET_VALUE)
            .ok_or_else(|| PlatformError::Other("cannot open the Run registry key".into()))?;
        let result = if enabled {
            let command = format!("\"{}\" {AUTOSTART_FLAG}", executable.display());
            let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes: Vec<u8> = wide.iter().flat_map(|u| u.to_le_bytes()).collect();
            // SAFETY: the key is open; the data is a NUL-terminated UTF-16 string.
            unsafe { RegSetValueExW(key, VALUE, None, REG_SZ, Some(&bytes)) }
        } else {
            // SAFETY: the key is open. A missing value is not an error for us.
            let status = unsafe { RegDeleteValueW(key, VALUE) };
            if status.0 == 2 { ERROR_SUCCESS } else { status }
        };
        // SAFETY: closing the key we opened above.
        let _ = unsafe { RegCloseKey(key) };
        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(PlatformError::Other(format!(
                "registry error {} for {}",
                result.0,
                HSTRING::from("Typlet")
            )))
        }
    }

    pub(super) fn is_enabled() -> bool {
        let Some(key) = open(KEY_QUERY_VALUE) else {
            return false;
        };
        // SAFETY: the key is open; we only query whether the value exists.
        let status = unsafe { RegQueryValueExW(key, VALUE, None, None, None, None) };
        // SAFETY: closing the key we opened above.
        let _ = unsafe { RegCloseKey(key) };
        status == ERROR_SUCCESS
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod imp {
    use std::path::Path;

    use crate::PlatformError;

    pub(super) fn set(_enabled: bool, _executable: &Path) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }

    pub(super) fn is_enabled() -> bool {
        false
    }
}
