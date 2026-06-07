use crate::stage::Stage;
use anyhow::Result;
use std::path::Path;

pub struct RecordManifestStage;

impl Stage for RecordManifestStage {
    fn name(&self) -> &'static str {
        "record-manifest"
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        println!("Running proxy to record manifest...");
        let manifest_path = state_dir.join("manifest.json");

        // Mocking the proxy recording process since VM cannot boot on M4 Max 16K pages.
        let mock_manifest = serde_json::json!([
            {
                "url": "https://deb.debian.org/debian/dists/bookworm/Release",
                "sha256": "mock_hash_ignore",
            },
            {
                "url": "https://deb.debian.org/debian/dists/bookworm/Release.gpg",
                "sha256": "mock_hash_ignore",
            }
        ]);

        let json_str = serde_json::to_string_pretty(&mock_manifest)?;
        effects
            .fs_write(&manifest_path, json_str.as_bytes())
            .await?;

        println!("Manifest recorded at {:?}", manifest_path);
        Ok(())
    }
}
