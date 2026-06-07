use crate::stage::Stage;
use anyhow::{Result, anyhow};
use std::path::Path;

pub struct CreateDiskStage {
    pub size_gb: u64,
}

impl Default for CreateDiskStage {
    fn default() -> Self {
        Self { size_gb: 64 }
    }
}

impl Stage for CreateDiskStage {
    fn name(&self) -> &'static str {
        "create-disk"
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        let disk_path = state_dir.join("disk.raw");

        println!(
            "Creating sparse raw disk of {} GB at {:?}",
            self.size_gb, disk_path
        );

        // create sparse file using mkfile
        // mkfile -n <size> <path> creates a sparse file on macOS
        let size_str = format!("{}g", self.size_gb);
        let status = effects
            .run_command("mkfile", &["-n", &size_str, disk_path.to_str().unwrap()])
            .await?;

        if !status.status.success() {
            return Err(anyhow!("Failed to create disk using mkfile."));
        }

        println!("Disk created successfully.");
        Ok(())
    }
}
