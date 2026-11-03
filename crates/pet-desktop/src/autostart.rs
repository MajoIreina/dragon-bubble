//! Per-user login startup. Never requires administrator privileges.
use std::path::Path;

#[cfg(windows)]
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "My Dragon Bubble";

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(any(windows, test))]
fn startup_command(exe: &Path) -> String {
    format!("\"{}\" --autostart", exe.display())
}

#[cfg(windows)]
pub fn enabled() -> std::io::Result<bool> {
    use windows_sys::Win32::System::Registry::*;
    let mut buffer = vec![0u16; 32768];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: all strings are terminated and the writable buffer matches size.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(VALUE_NAME).as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if result == 2 {
        return Ok(false);
    }
    if result != 0 {
        return Err(std::io::Error::from_raw_os_error(result as i32));
    }
    let value = String::from_utf16_lossy(&buffer[..(size as usize / 2).saturating_sub(1)]);
    Ok(value == startup_command(&std::env::current_exe()?))
}

#[cfg(windows)]
pub fn set_enabled(enabled: bool) -> std::io::Result<()> {
    use windows_sys::Win32::System::Registry::*;
    let result = if enabled {
        let command = wide(&startup_command(&std::env::current_exe()?));
        // RegSetKeyValueW creates the named subkey when necessary.
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                wide(RUN_KEY).as_ptr(),
                wide(VALUE_NAME).as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                wide(RUN_KEY).as_ptr(),
                wide(VALUE_NAME).as_ptr(),
            )
        }
    };
    if result == 0 || (!enabled && result == 2) {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(result as i32))
    }
}

#[cfg(any(target_os = "macos", test))]
fn launch_agent(exe: &Path) -> String {
    let escaped = exe
        .to_string_lossy()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>io.github.lppdesktop.my-dragon-bubble</string>
<key>ProgramArguments</key><array><string>{escaped}</string><string>--autostart</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>
"#
    )
}

#[cfg(target_os = "macos")]
fn agent_path() -> std::io::Result<std::path::PathBuf> {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| home.join("Library/LaunchAgents/io.github.lppdesktop.my-dragon-bubble.plist"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is unavailable"))
}

#[cfg(target_os = "macos")]
pub fn enabled() -> std::io::Result<bool> {
    match std::fs::read_to_string(agent_path()?) {
        Ok(value) => Ok(value == launch_agent(&std::env::current_exe()?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "macos")]
pub fn set_enabled(enabled: bool) -> std::io::Result<()> {
    let path = agent_path()?;
    if enabled {
        // launchd reads this at next login. Do not bootstrap a second running copy.
        pet_core::preferences::atomic_write(
            &path,
            launch_agent(&std::env::current_exe()?).as_bytes(),
        )
    } else {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn enabled() -> std::io::Result<bool> {
    Ok(false)
}
#[cfg(not(any(windows, target_os = "macos")))]
pub fn set_enabled(_: bool) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Unsupported platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_are_quoted_and_xml_escaped() {
        assert_eq!(
            startup_command(Path::new("C:/A B/pet.exe")),
            "\"C:/A B/pet.exe\" --autostart"
        );
        let plist = launch_agent(Path::new("/Apps/A & B/<Pet>.app/dragon-bubble"));
        assert!(plist.contains("A &amp; B/&lt;Pet&gt;.app"));
        assert!(!plist.contains("KeepAlive"));
    }
}
