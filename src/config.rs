use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    #[default]
    All,
    Folders,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub search_directory: String,
    pub archive_directory: String,
    pub editor_path: String,
    pub search_type: SearchType,
}

impl Default for AppConfig {
    fn default() -> Self {
        let home = UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        let search_dir = UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(PathBuf::from))
            .filter(|path| path.is_dir())
            .or_else(|| home.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let archive_dir = home
            .map(|path| path.join("Archive"))
            .unwrap_or_else(|| search_dir.join("Archive"));

        Self {
            search_directory: search_dir.to_string_lossy().into_owned(),
            archive_directory: archive_dir.to_string_lossy().into_owned(),
            editor_path: String::new(),
            search_type: SearchType::All,
        }
    }
}

impl AppConfig {
    pub fn load() -> (Self, Option<String>) {
        let path = config_path();
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(config) => (config, None),
                Err(error) => (
                    Self::default(),
                    Some(format!(
                        "Could not read settings; defaults are active: {error}"
                    )),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                let warning = config
                    .save()
                    .err()
                    .map(|save_error| format!("Could not create the settings file: {save_error}"));
                (config, warning)
            }
            Err(error) => (
                Self::default(),
                Some(format!(
                    "Could not read settings; defaults are active: {error}"
                )),
            ),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        let parent = path
            .parent()
            .ok_or_else(|| "The settings directory is unavailable".to_owned())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let contents = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        fs::write(path, contents).map_err(|error| error.to_string())
    }

    pub fn search_path(&self) -> PathBuf {
        expand_home(&self.search_directory)
    }

    pub fn archive_path(&self) -> PathBuf {
        expand_home(&self.archive_directory)
    }

    pub fn editor(&self) -> PathBuf {
        expand_home(&self.editor_path)
    }
}

fn config_path() -> PathBuf {
    ProjectDirs::from("com", "classicfoo", "Search")
        .map(|dirs| dirs.config_dir().join("config.json"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("search-config.json")
        })
}

fn expand_home(value: &str) -> PathBuf {
    if value == "~" || value.starts_with("~/") || value.starts_with("~\\") {
        if let Some(home) = UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            if value == "~" {
                return home;
            }
            let rest = value[2..].replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR);
            return home.join(rest);
        }
    }
    PathBuf::from(value)
}
