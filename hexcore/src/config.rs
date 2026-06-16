use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_undo_depth: usize,
    pub bytes_per_row: u64,
    pub mmap_threshold_mb: u64,
    pub show_ascii: bool,
    pub use_overwrite_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_undo_depth: 5000,
            bytes_per_row: 16,
            mmap_threshold_mb: 500,
            show_ascii: true,
            use_overwrite_mode: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
            })
        {
            dir.join("hexview").join("config.json")
        } else {
            PathBuf::from(".hexview_config.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.max_undo_depth, 5000);
        assert_eq!(cfg.bytes_per_row, 16);
        assert_eq!(cfg.mmap_threshold_mb, 500);
        assert!(cfg.show_ascii);
        assert!(cfg.use_overwrite_mode);
    }

    #[test]
    fn test_config_save_and_load() {
        let cfg = Config {
            max_undo_depth: 100,
            bytes_per_row: 32,
            mmap_threshold_mb: 1000,
            show_ascii: false,
            use_overwrite_mode: false,
        };
        cfg.save().unwrap();
        let loaded = Config::load();
        assert_eq!(loaded.max_undo_depth, 100);
        assert_eq!(loaded.bytes_per_row, 32);
        assert_eq!(loaded.mmap_threshold_mb, 1000);
        assert!(!loaded.show_ascii);
        assert!(!loaded.use_overwrite_mode);
        // Clean up
        let path = Config::config_path();
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(path.parent().unwrap());
    }
}
