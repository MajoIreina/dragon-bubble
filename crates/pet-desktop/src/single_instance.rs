//! Acquire before configuration, models, windows or GPU initialization.
use std::io;

pub struct Guard {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(target_os = "macos")]
    _file: std::fs::File,
}

pub fn acquire() -> io::Result<Option<Guard>> {
    #[cfg(windows)]
    {
        // Must match installer/windows.iss AppMutex, across installed/portable copies.
        acquire_named("DragonBubbleDesktopPet")
    }
    #[cfg(target_os = "macos")]
    {
        // Independent of LPP_DATA_DIR: changing profiles must not allow two pets.
        let home = std::env::var_os("HOME").ok_or_else(|| io::Error::other("Missing HOME"))?;
        let folder = std::path::PathBuf::from(home).join("Library/Caches/LPPDesktopPet");
        std::fs::create_dir_all(&folder)?;
        acquire_file(&folder.join("instance.lock"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unsupported desktop platform",
        ))
    }
}

#[cfg(windows)]
fn acquire_named(name: &str) -> io::Result<Option<Guard>> {
    use windows_sys::Win32::{Foundation::*, System::Threading::CreateMutexW};
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    // Non-inheritable; existence is the lock, no worker thread or periodic polling.
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    let error = unsafe { GetLastError() };
    if handle.is_null() {
        return Err(io::Error::from_raw_os_error(error as i32));
    }
    if error == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        return Ok(None);
    }
    Ok(Some(Guard { handle }))
}

#[cfg(windows)]
impl Drop for Guard {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(target_os = "macos")]
fn acquire_file(path: &std::path::Path) -> io::Result<Option<Guard>> {
    use std::{
        fs::OpenOptions,
        os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    };
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(Guard { _file: file }));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(None)
    } else {
        Err(error)
    }
    // Keep the file on disk: unlinking can let another process lock a new inode.
    // The kernel releases the lock when the owning process closes/exits/crashes.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicates_and_releases_after_guard_drop() {
        #[cfg(windows)]
        let name = format!("DragonBubbleSingleInstanceTest-{}", std::process::id());
        #[cfg(windows)]
        let lock = || acquire_named(&name);
        #[cfg(target_os = "macos")]
        let path =
            std::env::temp_dir().join(format!("dragon-instance-test-{}.lock", std::process::id()));
        #[cfg(target_os = "macos")]
        let lock = || acquire_file(&path);
        #[cfg(any(windows, target_os = "macos"))]
        {
            let first = lock().unwrap().expect("first owner");
            for _ in 0..64 {
                assert!(lock().unwrap().is_none());
            }
            drop(first);
            let next = lock().unwrap().expect("released lock");
            assert!(lock().unwrap().is_none());
            drop(next);
        }
        #[cfg(target_os = "macos")]
        std::fs::remove_file(path).unwrap();
    }
}
