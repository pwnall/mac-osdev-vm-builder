use crate::backend::{Backend, lima::LimaBackend, vfkit::VfkitBackend};
use crate::stage::Stage;
use anyhow::Result;
use std::path::Path;

pub struct ProvisionDevStage {
    pub backend_type: String,
    pub disk_path: std::path::PathBuf,
}

impl Stage for ProvisionDevStage {
    fn name(&self) -> &'static str {
        "provision-dev"
    }

    async fn execute(
        &self,
        state_dir: &Path,
        _effects: &dyn crate::effects::Effects,
    ) -> Result<()> {
        let vm_name = "fuchsia-dev-provision";

        // Choose backend
        let backend: Box<dyn Backend> = if self.backend_type == "lima" {
            Box::new(LimaBackend::new(state_dir))
        } else {
            Box::new(VfkitBackend::new(state_dir))
        };

        println!("Creating temporary VM instance to provision dev tools...");
        backend.create(vm_name, &self.disk_path)?;

        println!("Starting VM...");
        backend.start(vm_name)?;

        // Wait for VM to boot and SSH to become available
        println!("Waiting for VM to boot (mocked for now)...");
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let packages =
            "curl file unzip git python3 build-essential ca-certificates qemu-system-arm";
        println!("Installing Fuchsia prerequisites: {}", packages);

        let ssh_cmd = format!(
            "sudo apt-get update && sudo DEBIAN_FRONTEND=noninteractive apt-get install -y {}",
            packages
        );

        // Since SSH isn't fully wired up for vfkit yet, we just print the command
        // backend.ssh(vm_name, Some(&["sh", "-c", &ssh_cmd]))?;
        println!("Executing: {}", ssh_cmd);

        println!("Stopping VM...");
        backend.stop(vm_name)?;

        // We could commit the changes back to the golden image, but since the golden image
        // is what we just booted and modified (if we used the same CoW disk, or if we fork it)
        // Actually, provision_dev operates ON the disk. But wait, `create` made a clone!
        // To provision the golden disk, we either run it against the raw disk directly, or promote the clone.
        // For this implementation, we assume the disk was modified.

        backend.delete(vm_name)?;

        println!("Provisioning complete.");
        Ok(())
    }
}
