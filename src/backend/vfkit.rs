use crate::backend::Backend;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct VfkitBackend {
    pub workspace_dir: PathBuf,
}

impl VfkitBackend {
    pub fn new(workspace_dir: impl AsRef<Path>) -> Self {
        Self {
            workspace_dir: workspace_dir.as_ref().to_path_buf(),
        }
    }

    fn vm_dir(&self, vm_name: &str) -> PathBuf {
        self.workspace_dir.join(vm_name)
    }

    fn disk_path(&self, vm_name: &str) -> PathBuf {
        self.vm_dir(vm_name).join("disk.raw")
    }

    fn efi_vars_path(&self, vm_name: &str) -> PathBuf {
        self.vm_dir(vm_name).join("efi_vars.fd")
    }
}

impl Backend for VfkitBackend {
    fn create(&self, vm_name: &str, image_path: &Path) -> Result<()> {
        let dir = self.vm_dir(vm_name);
        if dir.exists() {
            return Err(anyhow!("VM {} already exists", vm_name));
        }
        std::fs::create_dir_all(&dir)?;

        // Use APFS clonefile to fork the golden image
        crate::clone::clone_file(image_path, &self.disk_path(vm_name))?;
        Ok(())
    }

    fn delete(&self, vm_name: &str) -> Result<()> {
        let dir = self.vm_dir(vm_name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    fn start(&self, vm_name: &str) -> Result<()> {
        let disk = self.disk_path(vm_name);
        let efi_vars = self.efi_vars_path(vm_name);
        if !disk.exists() {
            return Err(anyhow!("VM {} does not exist", vm_name));
        }

        // We run vfkit via script to allocate a PTY
        // We use --nested as required by the guide, and gvisor-tap-vsock for network (mac=00:11:22:33:44:55 for now)
        let vfkit_args = format!(
            "vfkit --cpus 4 --memory 4096 --nested --bootloader efi,variable-store={},create --device \"virtio-blk,path={}\" --device virtio-serial,stdio --device virtio-net,nat,mac=00:11:22:33:44:55 --device virtio-rng",
            efi_vars.display(),
            disk.display()
        );

        let log_path = self.vm_dir(vm_name).join("serial.log");
        let mut child = Command::new("script")
            .arg("-q")
            .arg(log_path.as_os_str())
            .arg("sh")
            .arg("-c")
            .arg(&vfkit_args)
            .spawn()?;

        // Wait briefly to ensure it doesn't crash immediately
        std::thread::sleep(std::time::Duration::from_secs(2));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(anyhow!("vfkit exited immediately with status {}", status));
        }

        Ok(())
    }

    fn stop(&self, vm_name: &str) -> Result<()> {
        // Just pkill vfkit for now, ideally we'd send ACPI shutdown
        let disk = self.disk_path(vm_name);
        let status = Command::new("pkill")
            .arg("-f")
            .arg(format!("vfkit.*{}", disk.display()))
            .status()?;

        if !status.success() {
            println!(
                "Warning: failed to stop VM {} (maybe it wasn't running?)",
                vm_name
            );
        }
        Ok(())
    }

    fn ssh(&self, _vm_name: &str, _command: Option<&[&str]>) -> Result<()> {
        // For vfkit, SSH requires resolving the guest IP from the MAC or using vsock.
        // The implementation guide notes: "Discover the NAT gateway; don't hardcode 10.0.2.2... the SSH path under vfkit requires a host->guest:22 forward via gvisor-tap-vsock"
        // For now, this is a placeholder.
        println!("vfkit ssh is not fully implemented yet");
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        let mut vms = Vec::new();
        if !self.workspace_dir.exists() {
            return Ok(vms);
        }
        for entry in std::fs::read_dir(&self.workspace_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && path.join("disk.raw").exists()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                vms.push(name.to_string());
            }
        }
        Ok(vms)
    }

    fn fork(&self, source_vm_name: &str, target_vm_name: &str) -> Result<()> {
        let source_disk = self.disk_path(source_vm_name);
        if !source_disk.exists() {
            return Err(anyhow!("Source VM does not exist"));
        }

        let target_dir = self.vm_dir(target_vm_name);
        if target_dir.exists() {
            return Err(anyhow!("Target VM already exists"));
        }
        std::fs::create_dir_all(&target_dir)?;

        // Find vfkit process for the source VM
        let pgrep_out = Command::new("pgrep")
            .arg("-f")
            .arg(format!("vfkit.*{}", source_disk.display()))
            .output()?;

        let pid_str = String::from_utf8_lossy(&pgrep_out.stdout)
            .trim()
            .to_string();
        let is_running = !pid_str.is_empty();

        if is_running {
            // Suspend the VM
            Command::new("kill").arg("-STOP").arg(&pid_str).status()?;
            std::thread::sleep(std::time::Duration::from_millis(500)); // give it a moment to pause
        }

        // Perform the clone
        let clone_result = (|| -> Result<()> {
            crate::clone::clone_file(&source_disk, &self.disk_path(target_vm_name))?;
            let source_efi = self.efi_vars_path(source_vm_name);
            if source_efi.exists() {
                std::fs::copy(&source_efi, self.efi_vars_path(target_vm_name))?;
            }
            Ok(())
        })();

        if is_running {
            // Resume the VM
            Command::new("kill").arg("-CONT").arg(&pid_str).status()?;
        }

        clone_result?;

        // Target VM is created but not started. The caller can start it.
        Ok(())
    }
}
