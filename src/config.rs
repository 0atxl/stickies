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
    #[serde(alias = "glass")]
    Minimal,
    Paper,
    #[default]
    Playful,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayfulEdges {
    Wavy,
    #[default]
    Clean,
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
    #[serde(alias = "adjacent")]
    Classic,
    #[default]
    #[serde(alias = "overlapping")]
    Stacked,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerStyle {
    Dots,
    #[default]
    Bar,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    #[default]
    Theme,
    Grotesk,
    Mono,
    Handwritten,
    Kalam,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteTextSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpAccess {
    #[default]
    Off,
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LegacyTextWeight {
    #[default]
    Regular,
    Bold,
}

impl LegacyTextWeight {
    fn is_regular(&self) -> bool {
        *self == Self::Regular
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NoteStyle {
    pub font: Option<Font>,
    pub text_size: NoteTextSize,
    // Read/preserve existing whole-note bold styles; there is no weight control.
    #[serde(
        rename = "text_weight",
        skip_serializing_if = "LegacyTextWeight::is_regular"
    )]
    pub legacy_text_weight: LegacyTextWeight,
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
    pub playful_edges: PlayfulEdges,
    pub appearance: Appearance,
    // Rendering defaults; legacy global choices migrate into per-note styles.
    #[serde(skip_serializing)]
    pub font: Font,
    // Accept obsolete saved values, but no longer write this setting.
    #[serde(rename = "font_size", skip_serializing)]
    pub legacy_font_size: Option<u32>,
    #[serde(skip_serializing)]
    pub paper: Paper,
    pub shape: Shape,
    pub tab_layout: TabLayout,
    pub marker_style: MarkerStyle,
    pub visible_notes: usize,
    pub inactivity_seconds: u64,
    pub reduced_motion: bool,
    pub hide_in_fullscreen: bool,
    pub hide_in_dnd: bool,
    pub mcp_access: McpAccess,
    #[serde(skip)]
    pub legacy_note_defaults: [bool; 2],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            edge: ScreenEdge::Right,
            marker_offset: 0.5,
            edge_hover_enabled: true,
            launch_at_login: false,
            theme: Theme::Playful,
            playful_edges: PlayfulEdges::Clean,
            appearance: Appearance::System,
            font: Font::Theme,
            legacy_font_size: None,
            paper: Paper::Plain,
            shape: Shape::Ticket,
            tab_layout: TabLayout::Stacked,
            marker_style: MarkerStyle::Bar,
            visible_notes: 4,
            inactivity_seconds: 120,
            reduced_motion: false,
            hide_in_fullscreen: true,
            hide_in_dnd: true,
            mcp_access: McpAccess::Off,
            legacy_note_defaults: [false; 2],
        }
    }
}

impl Config {
    pub fn clean_playful(&self) -> bool {
        self.theme == Theme::Playful && self.playful_edges == PlayfulEdges::Clean
    }

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
        let mut preferences: toml::Table = toml::from_str(&contents)?;
        let legacy_note_defaults = [
            preferences.contains_key("font"),
            preferences.contains_key("paper"),
        ];
        let removed = preferences.remove("white_paper").is_some();
        // Accept configurations saved while the alternate footer was available.
        let removed_footer = preferences.remove("footer_style").is_some();
        let mut config: Self = preferences.try_into()?;
        config.legacy_note_defaults = legacy_note_defaults;
        // Older files allowed custom timeouts from 30 to 1800 seconds.
        // Use the default when a legacy value has no matching Behavior choice.
        if (30..=1800).contains(&config.inactivity_seconds)
            && !matches!(config.inactivity_seconds, 60 | 120 | 300)
        {
            config.inactivity_seconds = Self::default().inactivity_seconds;
        }
        config.validate()?;
        if (removed || removed_footer) && !legacy_note_defaults.into_iter().any(|present| present) {
            config.save(path)?;
        }
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
        if !matches!(self.inactivity_seconds, 0 | 60 | 120 | 300) {
            return Err(ConfigError::InvalidValue(
                "inactivity_seconds must be 0, 60, 120 or 300",
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
        let loaded: super::Config = toml::from_str(&saved).unwrap();
        assert_eq!(loaded.theme, config.theme);
        assert_eq!(loaded.appearance, config.appearance);
        assert_eq!(loaded.font, super::Font::Theme);
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
        assert_eq!(loaded.font, Font::Theme);
        assert_eq!(loaded.edge, ScreenEdge::Left);
    }

    #[test]
    fn ui_settings_validate_limits_and_round_trip() {
        let config = Config {
            theme: Theme::Paper,
            appearance: Appearance::Dark,
            font: Font::Theme,
            paper: Paper::Plain,
            shape: Shape::Ticket,
            tab_layout: TabLayout::Classic,
            marker_style: MarkerStyle::Dots,
            visible_notes: 4,
            inactivity_seconds: 60,
            ..Config::default()
        };
        config.validate().unwrap();
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        for invalid in [
            Config {
                visible_notes: 6,
                ..config.clone()
            },
            Config {
                inactivity_seconds: 30,
                ..config.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
        let style: NoteStyle = toml::from_str("font = 'mono'").unwrap();
        assert_eq!(style.font, Some(Font::Mono));
        assert_eq!(style.text_size, NoteTextSize::Medium);
        assert_eq!(style.legacy_text_weight, LegacyTextWeight::Regular);
        assert_eq!(style.resolve(&config).font, Font::Mono);
        assert!(toml::to_string(&style).unwrap().contains("font ="));
        assert_eq!(style.resolve(&config).paper, Paper::Plain);
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
    fn white_paper_is_removed_without_losing_restored_choices_or_file_only_preferences() {
        let directory = test_directory("retired-choices");
        let path = directory.join("config.toml");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, "white_paper = true\ntab_layout = 'adjacent'\nmarker_style = 'dots'\nedge = 'left'\nhide_in_fullscreen = false\nhide_in_dnd = false\n").unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.edge, ScreenEdge::Left);
        assert!(!config.hide_in_fullscreen);
        assert!(!config.hide_in_dnd);
        let saved = fs::read_to_string(&path).unwrap();
        assert!(!saved.contains("white_paper"));
        assert_eq!(config.tab_layout, TabLayout::Classic);
        assert_eq!(config.marker_style, MarkerStyle::Dots);
        assert!(saved.contains("tab_layout"));
        assert!(saved.contains("marker_style"));
        assert_eq!(Config::load(&path).unwrap(), config);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn docked_note_timeout_choices_persist_in_the_existing_config() {
        let directory = test_directory("timeout-choices");
        let path = directory.join("config.toml");
        for seconds in [0, 60, 120, 300] {
            let config = Config {
                inactivity_seconds: seconds,
                ..Config::default()
            };
            config.save(&path).unwrap();
            assert_eq!(Config::load(&path).unwrap(), config);
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_custom_timeouts_use_two_minutes_without_rewriting_the_file() {
        let directory = test_directory("legacy-timeout");
        let path = directory.join("config.toml");
        fs::create_dir_all(&directory).unwrap();
        for seconds in [30, 90, 1800] {
            let contents = format!("edge = 'left'\ninactivity_seconds = {seconds}\n");
            fs::write(&path, &contents).unwrap();
            let config = Config::load(&path).unwrap();
            assert_eq!(config.inactivity_seconds, 120);
            assert_eq!(config.edge, ScreenEdge::Left);
            assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        }
        for seconds in [1, 29, 1801, u64::MAX] {
            fs::write(&path, format!("inactivity_seconds = {seconds}\n")).unwrap();
            assert!(Config::load(&path).is_err());
        }
        fs::remove_dir_all(directory).unwrap();
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
        assert_eq!(config.inactivity_seconds, 120);

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
