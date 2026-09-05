use std::{fs, io, io::Write, path::Path, path::PathBuf};

const DESKTOP_FILE_NAME: &str = "dev.stickies.Stickies.desktop";
const DESKTOP_ENTRY: &str = include_str!("../data/dev.stickies.Stickies.desktop");

pub fn reconcile(path: &Path, enabled: bool) -> io::Result<()> {
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

pub fn autostart_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("autostart")
        .join(DESKTOP_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn test_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "stickies-autostart-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time must follow the Unix epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn enabling_and_disabling_reconciles_the_desktop_entry() {
        let directory = test_directory();
        let path = directory.join(DESKTOP_FILE_NAME);

        reconcile(&path, true).expect("autostart should enable");
        assert_eq!(
            fs::read_to_string(&path).expect("autostart entry should be readable"),
            DESKTOP_ENTRY
        );
        reconcile(&path, true).expect("enabling twice should be harmless");
        reconcile(&path, false).expect("autostart should disable");
        assert!(!path.exists());
        reconcile(&path, false).expect("disabling twice should be harmless");

        fs::remove_dir_all(directory).expect("test directory should be removable");
    }
}
