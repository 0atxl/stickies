use std::{error::Error, fmt, path::Path};

#[derive(Debug)]
pub enum LockError {
    AlreadyLocked,
    Io(std::io::Error),
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyLocked => write!(f, "Another instance of Stickies is already running."),
            Self::Io(err) => write!(f, "Could not acquire owner lock: {err}"),
        }
    }
}

impl Error for LockError {}

impl From<std::io::Error> for LockError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub struct OwnerLock {
    #[cfg(target_os = "linux")]
    fd: std::os::fd::RawFd,
    #[cfg(target_os = "windows")]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl OwnerLock {
    pub fn acquire(path: &Path) -> Result<Self, LockError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::ffi::OsStrExt;
            let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path")
            })?;
            let fd = unsafe {
                libc::open(
                    c_path.as_ptr(),
                    libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(LockError::Io(std::io::Error::last_os_error()));
            }
            let res = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
            if res != 0 {
                let err = std::io::Error::last_os_error();
                unsafe { libc::close(fd) };
                if err.raw_os_error() == Some(libc::EWOULDBLOCK)
                    || err.raw_os_error() == Some(libc::EAGAIN)
                {
                    return Err(LockError::AlreadyLocked);
                }
                return Err(LockError::Io(err));
            }
            Ok(Self { fd })
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Foundation::{
                ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION, GetLastError, INVALID_HANDLE_VALUE,
            };
            use windows_sys::Win32::Storage::FileSystem::{
                CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
                OPEN_ALWAYS,
            };

            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                    0, // Exclusive access: no sharing prevents concurrent writers.
                    std::ptr::null(),
                    OPEN_ALWAYS,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                let err = unsafe { GetLastError() };
                if err == ERROR_SHARING_VIOLATION || err == ERROR_LOCK_VIOLATION {
                    return Err(LockError::AlreadyLocked);
                }
                return Err(LockError::Io(std::io::Error::from_raw_os_error(err as i32)));
            }
            Ok(Self { handle })
        }
    }
}

impl Drop for OwnerLock {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        unsafe {
            libc::flock(self.fd, libc::LOCK_UN);
            libc::close(self.fd);
        }

        #[cfg(target_os = "windows")]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
