use std::fs;
use std::path::PathBuf;

use rand::Rng;
use serde::Deserialize;
use thiserror::Error;

pub use self::buffer::Buffer;
pub use self::channel::Channel;
pub use self::keys::Keyboard;
pub use self::notification::{Notification, Notifications};
pub use self::server::Server;
pub use self::sidebar::Sidebar;
use crate::environment::config_dir;
use crate::server::Map as ServerMap;
use crate::theme::Palette;
use crate::{environment, Theme};

pub mod buffer;
pub mod channel;
mod keys;
pub mod notification;
pub mod server;
pub mod sidebar;

const CONFIG_TEMPLATE: &str = include_str!("../../config.toml");
const DEFAULT_THEME_FILE_NAME: &str = "ferra.toml";

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub themes: Themes,
    pub servers: ServerMap,
    pub font: Font,
    pub scale_factor: ScaleFactor,
    pub buffer: Buffer,
    pub sidebar: Sidebar,
    pub keyboard: Keyboard,
    pub notifications: Notifications,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ScaleFactor(f64);

impl Default for ScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

impl From<f64> for ScaleFactor {
    fn from(value: f64) -> Self {
        ScaleFactor(value.clamp(0.1, 3.0))
    }
}

impl From<ScaleFactor> for f64 {
    fn from(value: ScaleFactor) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Themes {
    pub default: Theme,
    pub all: Vec<Theme>,
}

impl Default for Themes {
    fn default() -> Self {
        Self {
            default: Theme::default(),
            all: vec![Theme::default()],
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let dir = environment::config_dir();

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create config folder");
        }

        dir
    }

    fn themes_dir() -> PathBuf {
        let dir = Self::config_dir().join("themes");

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create themes folder");
        }

        dir
    }

    fn path() -> PathBuf {
        Self::config_dir().join(environment::CONFIG_FILE_NAME)
    }

    pub fn load() -> Result<Self, Error> {
        #[derive(Deserialize)]
        pub struct Configuration {
            #[serde(default)]
            pub theme: String,
            pub servers: ServerMap,
            #[serde(default)]
            pub font: Font,
            #[serde(default)]
            pub scale_factor: ScaleFactor,
            #[serde(default)]
            pub buffer: Buffer,
            #[serde(default)]
            pub sidebar: Sidebar,
            #[serde(default)]
            pub keyboard: Keyboard,
            #[serde(default)]
            pub notifications: Notifications,
        }

        let path = Self::path();
        let content = fs::read_to_string(path).map_err(|e| Error::Read(e.to_string()))?;

        let Configuration {
            theme,
            servers,
            font,
            scale_factor,
            buffer,
            sidebar,
            keyboard,
            notifications,
        } = toml::from_str(content.as_ref()).map_err(|e| Error::Parse(e.to_string()))?;

        let themes = Self::load_themes(&theme).unwrap_or_default();

        Ok(Config {
            themes,
            servers,
            font,
            scale_factor,
            buffer,
            sidebar,
            keyboard,
            notifications,
        })
    }

    fn load_themes(default_key: &str) -> Result<Themes, Error> {
        #[derive(Deserialize)]
        pub struct Data {
            #[serde(default)]
            pub name: String,
            #[serde(default)]
            pub palette: Palette,
        }

        let read_entry = |entry: fs::DirEntry| {
            let content = fs::read_to_string(entry.path())?;

            let Data { name, palette } =
                toml::from_str(content.as_ref()).map_err(|e| Error::Parse(e.to_string()))?;

            Ok::<Theme, Error>(Theme::new(name, &palette))
        };

        let mut all = vec![];
        let mut default = Theme::default();
        let mut has_halloy_theme = false;

        for entry in fs::read_dir(Self::themes_dir())? {
            let Ok(entry) = entry else {
                continue;
            };

            let Some(file_name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };

            if file_name.ends_with(".toml") {
                if let Ok(theme) = read_entry(entry) {
                    if file_name.strip_suffix(".toml").unwrap_or_default() == default_key {
                        default = theme.clone();
                    }
                    if file_name == DEFAULT_THEME_FILE_NAME {
                        has_halloy_theme = true;
                    }

                    all.push(theme);
                }
            }
        }

        if !has_halloy_theme {
            all.push(Theme::default());
        }

        Ok(Themes { default, all })
    }

    pub fn create_template_config() {
        // Checks if a config file is there
        let config_file = Self::path();
        if config_file.exists() {
            return;
        }

        // Generate a unique nick
        let mut rng = rand::thread_rng();
        let rand_digit: u16 = rng.gen_range(1000..=9999);
        let rand_nick = format!("halloy{rand_digit}");

        // Replace placeholder nick with unique nick
        let config_template_string = CONFIG_TEMPLATE.replace("__NICKNAME__", rand_nick.as_str());
        let config_template_bytes = config_template_string.as_bytes();

        // Create configuration template path.
        let config_template_path = Self::config_dir().join("config.template.toml");

        let _ = fs::write(config_template_path, config_template_bytes);
    }
}

pub fn create_themes_dir() {
    const CONTENT: &[u8] = include_bytes!("../../assets/themes/ferra.toml");

    // Create default theme file.
    let file = Config::themes_dir().join(DEFAULT_THEME_FILE_NAME);
    if !file.exists() {
        let _ = fs::write(file, CONTENT);
    }
}

/// Has YAML configuration file.
pub fn has_yaml_config() -> bool {
    config_dir().join("config.yaml").exists()
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    Read(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Parse(String),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
