use crate::stage::Stage;
use anyhow::Result;
use std::path::Path;

pub struct FetchInstallerStage {
    pub kernel_path: std::path::PathBuf,
    pub initrd_path: std::path::PathBuf,
}

impl Default for FetchInstallerStage {
    fn default() -> Self {
        Self {
            kernel_path: std::path::PathBuf::from("osdev-vm-state/installer/vmlinux"),
            initrd_path: std::path::PathBuf::from("osdev-vm-state/installer/initrd.gz"),
        }
    }
}

impl Stage for FetchInstallerStage {
    fn name(&self) -> &'static str {
        "fetch-installer"
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        println!(
            "Fetching Debian 13 (Trixie) installer (simplification with 4K netboot kernel)..."
        );

        let base_url = "https://deb.debian.org/debian/dists/trixie/main/installer-arm64/current/images/netboot/debian-installer/arm64/";

        let installer_dir = state_dir.join("installer");
        effects.fs_create_dir_all(&installer_dir).await?;

        println!("Downloading installer linux kernel...");
        let vmlinuz_url = format!("{}linux", base_url);
        let vmlinuz_bytes = effects.http_get(&vmlinuz_url).await?;

        // Save directly, arm64 netboot kernel is an uncompressed Image
        effects.fs_write(&self.kernel_path, &vmlinuz_bytes).await?;

        println!("Downloading initrd.gz...");
        let initrd_url = format!("{}initrd.gz", base_url);
        let initrd_bytes = effects.http_get(&initrd_url).await?;
        effects.fs_write(&self.initrd_path, &initrd_bytes).await?;

        println!("Successfully fetched Trixie 4K installer!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::MockEffects;

    #[tokio::test]
    async fn test_fetch_installer() -> Result<()> {
        let mock = MockEffects::new();
        let state_dir = Path::new("/state");
        let stage = FetchInstallerStage {
            kernel_path: state_dir.join("vmlinux"),
            initrd_path: state_dir.join("initrd.gz"),
        };

        // Mock HTTP responses
        let base_url = "https://deb.debian.org/debian/dists/trixie/main/installer-arm64/current/images/netboot/debian-installer/arm64/";
        {
            let mut http = mock.http_responses.lock().await;
            http.insert(format!("{}linux", base_url), b"mock kernel".to_vec());
            http.insert(format!("{}initrd.gz", base_url), b"mock initrd".to_vec());
        }

        stage.execute(state_dir, &mock).await?;

        // Verify files
        let files = mock.files.lock().await;
        let kernel_content = files
            .get(&state_dir.join("vmlinux"))
            .expect("kernel missing");
        assert_eq!(kernel_content, b"mock kernel");

        let initrd_content = files
            .get(&state_dir.join("initrd.gz"))
            .expect("initrd missing");
        assert_eq!(initrd_content, b"mock initrd");

        Ok(())
    }
}
