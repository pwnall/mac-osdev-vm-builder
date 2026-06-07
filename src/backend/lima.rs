use crate::backend::Backend;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LimaBackend {
    pub workspace_dir: PathBuf,
}

impl LimaBackend {
    pub fn new(workspace_dir: impl AsRef<Path>) -> Self {
        Self {
            workspace_dir: workspace_dir.as_ref().to_path_buf(),
        }
    }
}

impl Backend for LimaBackend {
    fn create(&self, vm_name: &str, image_path: &Path) -> Result<()> {
        let yaml_path = self.workspace_dir.join(format!("{}.yaml", vm_name));

        let yaml_content = generate_lima_yaml(image_path);
        std::fs::write(&yaml_path, yaml_content)?;

        // Delete if exists, then create
        let _ = Command::new("limactl")
            .arg("delete")
            .arg("-f")
            .arg(vm_name)
            .status();
        let status = Command::new("limactl")
            .arg("create")
            .arg("--name")
            .arg(vm_name)
            .arg(&yaml_path)
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to create lima VM"));
        }
        Ok(())
    }

    fn delete(&self, vm_name: &str) -> Result<()> {
        let status = Command::new("limactl")
            .arg("delete")
            .arg("-f")
            .arg(vm_name)
            .status()?;
        if !status.success() {
            return Err(anyhow!("Failed to delete lima VM"));
        }

        let yaml_path = self.workspace_dir.join(format!("{}.yaml", vm_name));
        if yaml_path.exists() {
            std::fs::remove_file(yaml_path)?;
        }
        Ok(())
    }

    fn start(&self, vm_name: &str) -> Result<()> {
        let status = Command::new("limactl").arg("start").arg(vm_name).status()?;
        if !status.success() {
            return Err(anyhow!("Failed to start lima VM"));
        }
        Ok(())
    }

    fn stop(&self, vm_name: &str) -> Result<()> {
        let status = Command::new("limactl").arg("stop").arg(vm_name).status()?;
        if !status.success() {
            return Err(anyhow!("Failed to stop lima VM"));
        }
        Ok(())
    }

    fn ssh(&self, vm_name: &str, command: Option<&[&str]>) -> Result<()> {
        let mut cmd = Command::new("limactl");
        cmd.arg("shell").arg(vm_name);

        if let Some(c) = command {
            cmd.args(c);
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("limactl shell failed"));
        }
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        let output = Command::new("limactl")
            .arg("ls")
            .arg("--format")
            .arg("{{.Name}}")
            .output()?;
        let mut vms = Vec::new();
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let name = line.trim();
                if !name.is_empty() {
                    vms.push(name.to_string());
                }
            }
        }
        Ok(vms)
    }

    fn fork(&self, source_vm_name: &str, target_vm_name: &str) -> Result<()> {
        let status_out = Command::new("limactl")
            .arg("ls")
            .arg("--format")
            .arg("{{.Status}}")
            .arg(source_vm_name)
            .output()?;
        let status_str = String::from_utf8_lossy(&status_out.stdout)
            .trim()
            .to_string();
        if status_str.is_empty() {
            return Err(anyhow!("Source VM not found"));
        }
        let is_running = status_str == "Running";

        if is_running {
            Command::new("limactl")
                .arg("stop")
                .arg(source_vm_name)
                .status()?;
        }

        let dir_out = Command::new("limactl")
            .arg("ls")
            .arg("--format")
            .arg("{{.Dir}}")
            .arg(source_vm_name)
            .output()?;
        let dir_str = String::from_utf8_lossy(&dir_out.stdout).trim().to_string();
        if dir_str.is_empty() {
            return Err(anyhow!("Could not find directory for source lima VM"));
        }

        let source_dir = PathBuf::from(dir_str);
        if !source_dir.exists() {
            return Err(anyhow!("Source dir not found at {}", source_dir.display()));
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let target_dir = PathBuf::from(&home).join(".lima").join(target_vm_name);
        if target_dir.exists() {
            return Err(anyhow!("Target VM already exists"));
        }

        let status = Command::new("cp")
            .arg("-cR")
            .arg(&source_dir)
            .arg(&target_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow!("Failed to clone VM directory"));
        }

        if is_running {
            Command::new("limactl")
                .arg("start")
                .arg(source_vm_name)
                .status()?;
        }

        Ok(())
    }
}

pub fn generate_lima_yaml(image_path: &Path) -> String {
    format!(
        r#"
vmType: "vz"
images:
- location: "{}"
cpus: 4
memory: "4GiB"
nestedVirtualization: true
"#,
        image_path.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lima_yaml() {
        let image_path = Path::new("/mock/image.raw");
        let yaml = generate_lima_yaml(image_path);

        assert!(yaml.contains("vmType: \"vz\""));
        assert!(yaml.contains("- location: \"/mock/image.raw\""));
        assert!(yaml.contains("cpus: 4"));
        assert!(yaml.contains("memory: \"4GiB\""));
        assert!(yaml.contains("nestedVirtualization: true"));
    }
}
