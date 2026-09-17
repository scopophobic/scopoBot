use anyhow::{Context, Result};
use companion_core::PowerMode;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub position: Option<[f32; 2]>,
    pub scale: u32,
    pub click_through: bool,
    pub power: PowerMode,
    pub character: Option<PathBuf>,
    pub weather_city: Option<super::weather::City>,
    pub sensors_enabled: bool,
    pub first_run: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            position: None,
            scale: 2,
            click_through: false,
            power: PowerMode::Auto,
            character: None,
            weather_city: None,
            sensors_enabled: true,
            first_run: true,
        }
    }
}
impl Settings {
    pub fn path() -> Result<PathBuf> {
        if let Some(path) = std::env::var_os("SCOPOBOT_CONFIG_DIR") {
            return Ok(PathBuf::from(path).join("settings.json"));
        }
        let dir = directories::ProjectDirs::from("dev", "Scopobot", "Scopobot")
            .context("No local settings directory")?;
        Ok(dir.config_dir().join("settings.json"))
    }
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
        let had_scene = saved.get("room").and_then(|v| v.as_bool()) == Some(true);
        let mut value: Self = serde_json::from_value(saved)?;
        if had_scene {
            value.scale = 2;
        }
        value.scale = value.scale.clamp(2, 8);
        if value
            .position
            .is_some_and(|p| !p.iter().all(|v| v.is_finite()))
        {
            value.position = None;
        }
        Ok(value)
    }
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        fs::create_dir_all(path.parent().unwrap())?;
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        // Replace atomically so interruption never destroys the previous settings.
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows::{
                core::PCWSTR,
                Win32::Storage::FileSystem::{
                    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
                },
            };
            let source: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let destination: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                MoveFileExW(
                    PCWSTR(source.as_ptr()),
                    PCWSTR(destination.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )?;
            }
        }
        #[cfg(not(target_os = "windows"))]
        fs::rename(temp, path)?;
        Ok(())
    }
}
