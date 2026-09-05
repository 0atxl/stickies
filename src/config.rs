use std::{
    error::Error,
    fmt, fs,
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenEdge {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub edge: ScreenEdge,
    pub marker_offset: f64,
    pub edge_hover_enabled: bool,
    pub launch_at_login: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            edge: ScreenEdge::Right,
            marker_offset: 0.5,
            edge_hover_enabled: true,
            launch_at_login: false,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                return Ok(config);
            }
            Err(error) => return Err(error.into()),
        };
        let config: Self = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        self.validate()?;
        let contents = toml::to_string_pretty(self)?;
        let parent = path.parent().ok_or(ConfigError::MissingParent)?;
        fs::create_dir_all(parent)?;

        let temporary = path.with_extension("toml.tmp");
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if !self.marker_offset.is_finite() || !(0.0..=1.0).contains(&self.marker_offset) {
            return Err(ConfigError::InvalidMarkerOffset(self.marker_offset));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    InvalidMarkerOffset(f64),
    MissingParent,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "could not access the configuration: {error}"),
            Self::Parse(error) => write!(formatter, "invalid configuration: {error}"),
            Self::Serialize(error) => {
                write!(formatter, "could not encode the configuration: {error}")
            }
            Self::InvalidMarkerOffset(offset) => {
                write!(
                    formatter,
                    "marker_offset must be between 0 and 1, got {offset}"
                )
            }
            Self::MissingParent => write!(formatter, "configuration path has no parent directory"),
        }
    }
}

impl Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::Parse(error)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(error: toml::ser::Error) -> Self {
        Self::Serialize(error)
    }
}

pub fn config_path() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("stickies")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn test_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "stickies-config-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time must follow the Unix epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn a_missing_file_is_created_with_defaults() {
        let directory = test_directory("missing");
        let path = directory.join("config.toml");
        assert_eq!(
            Config::load(&path).expect("defaults should load"),
            Config::default()
        );
        assert!(path.is_file());

        fs::remove_dir_all(directory).expect("test directory should be removable");
    }

    #[test]
    fn configuration_round_trips_through_an_atomic_save() {
        let directory = test_directory("round-trip");
        let path = directory.join("config.toml");
        let config = Config {
            edge: ScreenEdge::Left,
            marker_offset: 0.25,
            edge_hover_enabled: false,
            launch_at_login: true,
        };

        config.save(&path).expect("configuration should save");
        assert_eq!(
            Config::load(&path).expect("configuration should load"),
            config
        );
        assert!(!path.with_extension("toml.tmp").exists());

        fs::remove_dir_all(directory).expect("test directory should be removable");
    }

    #[test]
    fn partial_configuration_keeps_defaults() {
        let directory = test_directory("partial");
        let path = directory.join("config.toml");
        fs::create_dir_all(&directory).expect("test directory should be created");
        fs::write(&path, "edge = \"top\"\n").expect("test configuration should be written");

        let config = Config::load(&path).expect("partial configuration should load");
        assert_eq!(config.edge, ScreenEdge::Top);
        assert_eq!(config.marker_offset, 0.5);
        assert!(config.edge_hover_enabled);
        assert!(!config.launch_at_login);

        fs::remove_dir_all(directory).expect("test directory should be removable");
    }

    #[test]
    fn invalid_values_and_unknown_keys_are_rejected() {
        let directory = test_directory("invalid");
        let path = directory.join("config.toml");
        fs::create_dir_all(&directory).expect("test directory should be created");
        fs::write(&path, "marker_offset = 1.5\n").expect("test configuration should be written");
        assert!(matches!(
            Config::load(&path),
            Err(ConfigError::InvalidMarkerOffset(offset)) if offset == 1.5
        ));

        fs::write(&path, "future_setting = true\n").expect("test configuration should be written");
        assert!(matches!(Config::load(&path), Err(ConfigError::Parse(_))));
        assert_eq!(
            fs::read_to_string(&path).expect("invalid configuration should remain readable"),
            "future_setting = true\n"
        );

        fs::remove_dir_all(directory).expect("test directory should be removable");
    }
}
