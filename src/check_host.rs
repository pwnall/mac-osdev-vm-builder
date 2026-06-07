use crate::stage::Stage;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;

pub struct CheckHostStage;

impl Stage for CheckHostStage {
    fn name(&self) -> &'static str {
        "check-host"
    }

    async fn execute(
        &self,
        _state_dir: &Path,
        _effects: &dyn crate::effects::Effects,
    ) -> Result<()> {
        // Check for macOS ARM64
        let uname_m = Command::new("uname").arg("-m").output()?;
        let arch = String::from_utf8_lossy(&uname_m.stdout).trim().to_string();
        if arch != "arm64" {
            return Err(anyhow!(
                "Host architecture is {}, but arm64 is required.",
                arch
            ));
        }

        let uname_s = Command::new("uname").arg("-s").output()?;
        let os = String::from_utf8_lossy(&uname_s.stdout).trim().to_string();
        if os != "Darwin" {
            return Err(anyhow!(
                "Host OS is {}, but Darwin (macOS) is required.",
                os
            ));
        }

        // Check for nested virtualization support
        // On Apple Silicon, M3 or newer supports nested virtualization.
        // We can check this by looking at machdep.cpu.brand_string
        let sysctl = Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()?;
        if sysctl.status.success() {
            let brand = String::from_utf8_lossy(&sysctl.stdout).trim().to_string();
            // E.g., "Apple M4 Max"
            if brand.contains("Apple M1") || brand.contains("Apple M2") {
                return Err(anyhow!(
                    "Nested virtualization is not supported on this host. (M3 or newer is required, found {})",
                    brand
                ));
            } else if !brand.contains("Apple M") {
                // Not an M-series Mac? Just warn or fail, but we already checked arch=arm64.
                println!(
                    "Warning: Could not definitively determine M-series generation from '{}'",
                    brand
                );
            }
        } else {
            return Err(anyhow!("Failed to check CPU brand string."));
        }

        println!("Host check passed: macOS ARM64 with nested virtualization support.");
        Ok(())
    }
}
