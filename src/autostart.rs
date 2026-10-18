use std::{io, path::Path, path::PathBuf};

#[cfg(target_os = "linux")]
const DESKTOP_FILE_NAME: &str = "dev.stickies.Stickies.desktop";
#[cfg(target_os = "linux")]
const DESKTOP_ENTRY: &str = include_str!("../data/dev.stickies.Stickies.desktop");

#[cfg(target_os = "windows")]
const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const APP_VALUE_NAME: &str = "Stickies";

#[cfg(target_os = "linux")]
pub fn reconcile(path: &Path, enabled: bool) -> io::Result<()> {
    use std::{fs, io::Write};
    if !enabled {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };
    }

    if fs::read_to_string(path).is_ok_and(|contents| contents == DESKTOP_ENTRY) {
        return Ok(());
    }

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "autostart path has no parent")
    })?;
    fs::create_dir_all(parent)?;

    let temporary = path.with_extension("desktop.tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(DESKTOP_ENTRY.as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary, path)
}

#[cfg(target_os = "windows")]
pub fn reconcile(_path: &Path, enabled: bool) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let command = format!("\"{}\"", exe.display());
    reconcile_registry_key(RUN_SUBKEY, &command, enabled)
}

#[cfg(target_os = "windows")]
fn reconcile_registry_key(subkey: &str, command: &str, enabled: bool) -> io::Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(subkey)?;
    if enabled {
        if key
            .get_value::<String, _>(APP_VALUE_NAME)
            .is_ok_and(|current| current == command)
        {
            return Ok(());
        }
        key.set_value(APP_VALUE_NAME, &command)
    } else {
        match key.delete_value(APP_VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

pub fn autostart_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        gtk::glib::user_config_dir()
            .join("autostart")
            .join(DESKTOP_FILE_NAME)
    }
    #[cfg(target_os = "windows")]
    {
        gtk::glib::user_config_dir().join("Stickies.autostart")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(target_os = "linux")]
    #[test]
    fn enabling_and_disabling_reconciles_the_desktop_entry() {
        let directory = std::env::temp_dir().join(format!(
            "stickies-autostart-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time must follow the Unix epoch")
                .as_nanos()
        ));
        let path = directory.join(DESKTOP_FILE_NAME);

        reconcile(&path, true).expect("autostart should enable");
        assert_eq!(
            std::fs::read_to_string(&path).expect("autostart entry should be readable"),
            DESKTOP_ENTRY
        );
        reconcile(&path, true).expect("enabling twice should be harmless");
        reconcile(&path, false).expect("autostart should disable");
        assert!(!path.exists());
        reconcile(&path, false).expect("disabling twice should be harmless");

        std::fs::remove_dir_all(directory).expect("test directory should be removable");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn enabling_and_disabling_reconciles_the_registry_value() {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let subkey = format!(
            r"Software\StickiesTest\Run-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time must follow the Unix epoch")
                .as_nanos()
        );
        let cmd = r#""C:\Program Files\Stickies\bin\stickies.exe""#;

        reconcile_registry_key(&subkey, cmd, true).expect("autostart should enable");
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey(&subkey).expect("subkey should exist");
        let actual: String = key.get_value(APP_VALUE_NAME).expect("value should exist");
        assert_eq!(actual, cmd);

        reconcile_registry_key(&subkey, cmd, true).expect("enabling twice should be harmless");
        reconcile_registry_key(&subkey, cmd, false).expect("autostart should disable");
        assert!(key.get_value::<String, _>(APP_VALUE_NAME).is_err());
        reconcile_registry_key(&subkey, cmd, false).expect("disabling twice should be harmless");

        drop(key);
        let _ = hkcu.delete_subkey_all(&subkey);
    }
}
