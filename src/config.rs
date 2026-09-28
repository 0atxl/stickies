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

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    #[serde(alias = "glass")]
    Minimal,
    Paper,
    Playful,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    #[default]
    Plain,
    Dots,
    Grid,
    Ruled,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Square,
    Ticket,
    Torn,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TabLayout {
    #[default]
    Adjacent,
    Overlapping,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    #[default]
    Theme,
    Grotesk,
    Mono,
    Handwritten,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NoteStyle {
    pub font: Option<Font>,
    pub paper: Option<Paper>,
    pub shape: Option<Shape>,
}

impl NoteStyle {
    pub fn resolve(&self, defaults: &Config) -> Config {
        let mut config = defaults.clone();
        if let Some(font) = self.font {
            config.font = font;
        }
        if let Some(paper) = self.paper {
            config.paper = paper;
        }
        if let Some(shape) = self.shape {
            config.shape = shape;
        }
        config
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub edge: ScreenEdge,
    pub marker_offset: f64,
    pub edge_hover_enabled: bool,
    pub launch_at_login: bool,
    pub theme: Theme,
    pub appearance: Appearance,
    pub font: Font,
    // Accept obsolete saved values, but no longer write this setting.
    #[serde(rename = "font_size", skip_serializing)]
    pub legacy_font_size: Option<u32>,
    pub paper: Paper,
    pub shape: Shape,
    pub white_paper: bool,
    pub tab_layout: TabLayout,
    pub visible_notes: usize,
    pub inactivity_seconds: u64,
    pub reduced_motion: bool,
    pub hide_in_fullscreen: bool,
    pub hide_in_dnd: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            edge: ScreenEdge::Right,
            marker_offset: 0.5,
            edge_hover_enabled: true,
            launch_at_login: false,
            theme: Theme::Paper,
            appearance: Appearance::System,
            font: Font::Theme,
            legacy_font_size: None,
            paper: Paper::Plain,
            shape: Shape::Ticket,
            white_paper: false,
            tab_layout: TabLayout::Adjacent,
            visible_notes: 5,
            inactivity_seconds: 120,
            reduced_motion: false,
            hide_in_fullscreen: true,
            hide_in_dnd: true,
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
        if !matches!(self.visible_notes, 4 | 5) {
            return Err(ConfigError::InvalidValue("visible_notes must be 4 or 5"));
        }
        if !(30..=1800).contains(&self.inactivity_seconds) {
            return Err(ConfigError::InvalidValue(
                "inactivity_seconds must be between 30 and 1800",
            ));
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
    InvalidValue(&'static str),
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
            Self::InvalidValue(message) => formatter.write_str(message),
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
    #[test]
    fn legacy_glass_preserves_preferences_and_saves_as_minimal() {
        let config: super::Config =
            toml::from_str("theme = 'glass'\nappearance = 'dark'\nfont = 'handwritten'").unwrap();
        assert_eq!(config.theme, super::Theme::Minimal);
        assert_eq!(config.appearance, super::Appearance::Dark);
        assert_eq!(config.font, super::Font::Handwritten);
        let saved = toml::to_string(&config).unwrap();
        assert!(saved.contains("theme = \"minimal\""));
        assert_eq!(toml::from_str::<super::Config>(&saved).unwrap(), config);
    }

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
    fn obsolete_font_size_loads_but_is_not_written_back() {
        let config: Config =
            toml::from_str("font_size = 18\nfont = 'mono'\nedge = 'left'").unwrap();
        config.validate().unwrap();
        assert_eq!(config.font, Font::Mono);
        assert_eq!(config.edge, ScreenEdge::Left);
        let saved = toml::to_string(&config).unwrap();
        assert!(!saved.contains("font_size"));
        let loaded: Config = toml::from_str(&saved).unwrap();
        assert_eq!(loaded.font, Font::Mono);
        assert_eq!(loaded.edge, ScreenEdge::Left);
    }

    #[test]
    fn ui_settings_validate_limits_and_round_trip() {
        let config = Config {
            theme: Theme::Paper,
            appearance: Appearance::Dark,
            font: Font::Handwritten,
            paper: Paper::Grid,
            shape: Shape::Ticket,
            visible_notes: 4,
            tab_layout: TabLayout::Overlapping,
            inactivity_seconds: 90,
            ..Config::default()
        };
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        for invalid in [
            Config {
                visible_notes: 6,
                ..config.clone()
            },
            Config {
                inactivity_seconds: 0,
                ..config.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
        let style = NoteStyle {
            font: Some(Font::Mono),
            ..NoteStyle::default()
        };
        assert_eq!(style.resolve(&config).font, Font::Mono);
        assert_eq!(style.resolve(&config).paper, Paper::Grid);
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
            ..Config::default()
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
