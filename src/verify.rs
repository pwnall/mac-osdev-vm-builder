use crate::stage::Stage;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;

pub struct VerifyStage {
    pub backend_type: String,
    pub disk_path: std::path::PathBuf,
}

impl Stage for VerifyStage {
    fn name(&self) -> &'static str {
        "verify"
    }

    async fn execute(
        &self,
        _state_dir: &Path,
        _effects: &dyn crate::effects::Effects,
    ) -> Result<()> {
        println!("Running integration tests...");

        let abs_disk_path = if self.disk_path.is_absolute() {
            self.disk_path.clone()
        } else {
            std::env::current_dir()?.join(&self.disk_path)
        };

        // Pass the disk path and backend to the integration tests via environment variables
        let status = Command::new("cargo")
            .arg("test")
            .arg("--test")
            .arg("integration_tests")
            .env("VM_DISK_PATH", &abs_disk_path)
            .env("VM_BACKEND", &self.backend_type)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Integration tests failed"));
        }

        println!("All integration tests passed.");
        Ok(())
    }
}
