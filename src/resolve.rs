use crate::stage::Stage;
use anyhow::{Result, anyhow};
use std::path::Path;

pub struct ResolveStage;

impl Stage for ResolveStage {
    fn name(&self) -> &'static str {
        "resolve"
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        println!("Running host preflight checks...");

        let arch = run_sysctl("hw.machine", effects).await?;
        if arch != "arm64" {
            return Err(anyhow!("Host must be arm64, found {}", arch));
        }

        let brand = run_sysctl("machdep.cpu.brand_string", effects).await?;
        if !brand.contains("Apple M") {
            return Err(anyhow!("Host must be Apple Silicon, found {}", brand));
        }

        // M3 or newer (M3, M4, etc.)
        let is_m3_plus = brand.contains("M3")
            || brand.contains("M4")
            || brand.contains("M5")
            || brand.contains("M6");
        if !is_m3_plus {
            return Err(anyhow!(
                "Nested virtualization requires M3 or newer, found {}",
                brand
            ));
        }

        let hv_support = run_sysctl("kern.hv_support", effects).await?;
        if hv_support != "1" {
            return Err(anyhow!(
                "Hypervisor support (kern.hv_support) is not enabled"
            ));
        }

        let osrelease = run_sysctl("kern.osrelease", effects).await?;
        let major_version: u32 = osrelease
            .split('.')
            .next()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        if major_version < 24 {
            return Err(anyhow!(
                "Nested virtualization requires macOS 15 (Darwin 24) or newer, found Darwin {}",
                major_version
            ));
        }

        println!("Preflight checks passed! Host supports nested virtualization.");

        // For now, hardcode the pins to Bookworm for the rest of the pipeline
        let pins_path = state_dir.join("pins.json");
        let pins = serde_json::json!({
            "suite": "bookworm",
            "snapshot_timestamp": "20240101T000000Z", // Mock timestamp for now
        });
        effects
            .fs_write(&pins_path, serde_json::to_string_pretty(&pins)?.as_bytes())
            .await?;

        Ok(())
    }
}

async fn run_sysctl(key: &str, effects: &dyn crate::effects::Effects) -> Result<String> {
    let output = effects.run_command("sysctl", &["-n", key]).await?;
    if !output.status.success() {
        return Err(anyhow!("Failed to read sysctl key {}", key));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::MockEffects;
    use std::os::unix::process::ExitStatusExt;

    #[tokio::test]
    async fn test_resolve_success() -> Result<()> {
        let mock = MockEffects::new();

        // Mock sysctl outputs for success
        let mut cmds = mock.command_outputs.lock().await;
        let success_status = std::process::ExitStatus::from_raw(0);

        cmds.insert(
            "sysctl -n hw.machine".to_string(),
            std::process::Output {
                status: success_status,
                stdout: b"arm64\n".to_vec(),
                stderr: vec![],
            },
        );
        cmds.insert(
            "sysctl -n machdep.cpu.brand_string".to_string(),
            std::process::Output {
                status: success_status,
                stdout: b"Apple M3 Max\n".to_vec(),
                stderr: vec![],
            },
        );
        cmds.insert(
            "sysctl -n kern.hv_support".to_string(),
            std::process::Output {
                status: success_status,
                stdout: b"1\n".to_vec(),
                stderr: vec![],
            },
        );
        cmds.insert(
            "sysctl -n kern.osrelease".to_string(),
            std::process::Output {
                status: success_status,
                stdout: b"24.0.0\n".to_vec(),
                stderr: vec![],
            },
        );
        drop(cmds);

        let stage = ResolveStage;
        let state_dir = Path::new("/state");
        stage.execute(state_dir, &mock).await?;

        // Check if pins.json was written
        let files = mock.files.lock().await;
        let pins_content = files
            .get(&state_dir.join("pins.json"))
            .expect("pins.json missing");
        let pins_str = std::str::from_utf8(pins_content)?;
        assert!(pins_str.contains("bookworm"));

        Ok(())
    }

    #[tokio::test]
    async fn test_resolve_fails_intel() -> Result<()> {
        let mock = MockEffects::new();
        let mut cmds = mock.command_outputs.lock().await;
        let success_status = std::process::ExitStatus::from_raw(0);
        cmds.insert(
            "sysctl -n hw.machine".to_string(),
            std::process::Output {
                status: success_status,
                stdout: b"x86_64\n".to_vec(),
                stderr: vec![],
            },
        );
        drop(cmds);

        let stage = ResolveStage;
        let res = stage.execute(Path::new("/state"), &mock).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Host must be arm64"));
        Ok(())
    }
}
