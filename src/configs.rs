use crate::error::Suggestion;
use error_stack::ResultExt;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs::canonicalize;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ConfigError {
    NoDefaultSearchPath,
    LoadError,
    IoError,
}

type Result<T> = error_stack::Result<T, ConfigError>;

impl std::error::Error for ConfigError {}

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDefaultSearchPath => write!(f, "No default search path was found"),
            Self::LoadError => write!(f, "Could not load configuration"),
            Self::IoError => write!(f, "IO error"),
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub search_dirs: Option<Vec<SearchDirectory>>,
}

impl Config {
    pub(crate) fn new() -> Result<Self> {
        let mut builder = config::Config::builder();
        let mut config_found = false; // Stores whether a valid config file was found
        if let Some(home_path) = dirs::home_dir() {
            config_found = true;
            let path = home_path.as_path().join(".config/tms/config.toml");
            builder = builder.add_source(config::File::from(path).required(false));
        }
        if !config_found {
            return Err(ConfigError::LoadError)
                            .attach_printable("Could not find a valid location for config file (both home and config dirs cannot be found)")
                            .attach(Suggestion("Try specifying a config file with the TMS_CONFIG_FILE environment variable."));
        }
        let config = builder
            .build()
            .change_context(ConfigError::LoadError)
            .attach_printable("Could not parse configuration")?;
        config
            .try_deserialize()
            .change_context(ConfigError::LoadError)
            .attach_printable("Could not deserialize configuration")
    }

    pub fn search_dirs(&self) -> Result<Vec<SearchDirectory>> {
        let search_dirs = if let Some(search_dirs) = self.search_dirs.as_ref() {
            search_dirs
                .iter()
                .map(|search_dir| {
                    let expanded_path = shellexpand::full(&search_dir.path.to_string_lossy())
                        .change_context(ConfigError::IoError)?
                        .to_string();

                    let path = canonicalize(expanded_path).change_context(ConfigError::IoError)?;

                    Ok(SearchDirectory::new(path, search_dir.depth))
                })
                .collect::<Result<_>>()
        } else {
            Ok(Vec::new())
        }?;

        if search_dirs.is_empty() {
            return Err(ConfigError::NoDefaultSearchPath)
            .attach_printable(
                "You must configure at least one default search path with the `config` subcommand. E.g `tms config` ",
            );
        }

        Ok(search_dirs)
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchDirectory {
    pub path: PathBuf,
    pub depth: usize,
}
impl SearchDirectory {
    pub fn new(path: PathBuf, depth: usize) -> Self {
        SearchDirectory { path, depth }
    }
}
