use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Deserialize;

use crate::unwrap::PluginUnwrapError;

#[derive(Debug, thiserror::Error)]
pub enum PluginConfigError {
    #[error("Failed to parse manifest: {reason}")]
    InvalidManifest { reason: String },

    #[error("Failed to unwrap plugin")]
    InvalidCrab(#[from] PluginUnwrapError),

    #[error("Directory containing plugin has non UTF-8 name")]
    NonUtf8DirectoryName,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    #[default]
    Native,
    Wasm,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    display_name: String,
    #[serde(default)]
    plugin_type: PluginType,
}

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub(crate) animation_id: String,
    pub(crate) manifest: PluginManifest,
    pub(crate) path: PathBuf,
}

impl PluginConfig {
    pub fn new(path: PathBuf) -> Result<Self, PluginConfigError> {
        let manifest: PluginManifest =
            serde_json::from_slice(&std::fs::read(path.join("manifest.json")).map_err(|e| {
                PluginConfigError::InvalidManifest {
                    reason: format!("IO error: {}", e),
                }
            })?)
            .map_err(|e| PluginConfigError::InvalidManifest {
                reason: e.to_string(),
            })?;

        let animation_id = path
            .file_name()
            .unwrap()
            .to_str()
            .ok_or(PluginConfigError::NonUtf8DirectoryName)?
            .to_owned();

        Ok(Self {
            animation_id,
            manifest,
            path,
        })
    }

    pub fn animation_id(&self) -> &str {
        &self.animation_id
    }

    pub fn animation_name(&self) -> &str {
        &self.manifest.display_name
    }

    pub fn plugin_type(&self) -> PluginType {
        self.manifest.plugin_type
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn executable_path(&self) -> PathBuf {
        let executable_name = if self.manifest.plugin_type == PluginType::Wasm {
            "plugin.wasm"
        } else if cfg!(windows) {
            "plugin.exe"
        } else {
            "plugin"
        };

        self.path.join(executable_name)
    }

    pub fn is_executable(&self) -> bool {
        match self.manifest.plugin_type {
            PluginType::Native => Command::new(self.executable_path())
                .stdout(Stdio::null())
                .stdin(Stdio::null())
                .spawn()
                .is_ok(),
            PluginType::Wasm => self.executable_path().exists(),
        }
    }
}
