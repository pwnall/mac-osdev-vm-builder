pub mod backend;
pub mod check_host;
pub mod clone;
pub mod create_disk;
pub mod effects;
pub mod fetch_installer;
pub mod install;
pub mod materialize_mirror;
pub mod provision_dev;
pub mod record_manifest;
pub mod resolve;
pub mod stage;
pub mod verify;

use crate::stage::Stage;
use anyhow::Result;
use std::path::Path;

pub struct Runner<'a> {
    state_dir: std::path::PathBuf,
    effects: &'a dyn crate::effects::Effects,
}

impl<'a> Runner<'a> {
    pub fn new(state_dir: impl AsRef<Path>, effects: &'a dyn crate::effects::Effects) -> Self {
        Self {
            state_dir: state_dir.as_ref().to_path_buf(),
            effects,
        }
    }

    pub async fn run_stage<S: Stage>(&self, stage: S) -> Result<()> {
        if !self.effects.fs_exists(&self.state_dir).await {
            self.effects.fs_create_dir_all(&self.state_dir).await?;
        }

        println!("Running stage: {}", stage.name());
        if stage.is_completed(&self.state_dir, self.effects).await? {
            println!("Stage {} is already completed, skipping.", stage.name());
            return Ok(());
        }

        stage.execute(&self.state_dir, self.effects).await?;
        stage.mark_completed(&self.state_dir, self.effects).await?;
        println!("Stage {} completed successfully.", stage.name());

        Ok(())
    }

    pub async fn reset_stage<S: Stage>(&self, stage: S) -> Result<()> {
        println!("Resetting stage: {}", stage.name());
        stage.reset(&self.state_dir, self.effects).await?;
        Ok(())
    }
}
