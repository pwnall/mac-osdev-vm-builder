use crate::stage::Stage;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct ManifestEntry {
    url: String,
    #[allow(dead_code)]
    sha256: String,
}

pub struct MaterializeMirrorStage;

impl Stage for MaterializeMirrorStage {
    fn name(&self) -> &'static str {
        "materialize-mirror"
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        let manifest_path = state_dir.join("manifest.json");
        let mirror_dir = state_dir.join("mirror");

        if !effects.fs_exists(&manifest_path).await {
            return Err(anyhow::anyhow!(
                "manifest.json not found. Must run record-manifest first."
            ));
        }

        effects.fs_create_dir_all(&mirror_dir).await?;

        let manifest_content = effects.fs_read_to_string(&manifest_path).await?;
        let manifest: Vec<ManifestEntry> =
            serde_json::from_str(&manifest_content).context("Failed to parse manifest.json")?;

        for entry in manifest {
            let url_parsed = reqwest::Url::parse(&entry.url)?;
            let url_path = url_parsed.path().trim_start_matches('/');
            let file_path = mirror_dir.join(url_path);

            if let Some(parent) = file_path.parent() {
                effects.fs_create_dir_all(parent).await?;
            }

            println!("Materializing: {}", entry.url);
            let bytes = effects.http_get(&entry.url).await?;
            effects.fs_write(&file_path, &bytes).await?;
        }

        println!("Offline mirror materialized at {:?}", mirror_dir);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::MockEffects;

    #[tokio::test]
    async fn test_materialize_mirror() -> Result<()> {
        let mock = MockEffects::new();
        let state_dir = Path::new("/state");

        // Write mock manifest
        let manifest = serde_json::json!([
            {
                "url": "https://deb.debian.org/debian/dists/bookworm/Release",
                "sha256": "mock1"
            },
            {
                "url": "https://deb.debian.org/debian/dists/bookworm/Release.gpg",
                "sha256": "mock2"
            }
        ]);
        let manifest_bytes = serde_json::to_vec(&manifest)?;

        {
            let mut files = mock.files.lock().await;
            files.insert(state_dir.join("manifest.json"), manifest_bytes);
        }

        // Mock HTTP responses
        {
            let mut http = mock.http_responses.lock().await;
            http.insert(
                "https://deb.debian.org/debian/dists/bookworm/Release".to_string(),
                b"release content".to_vec(),
            );
            http.insert(
                "https://deb.debian.org/debian/dists/bookworm/Release.gpg".to_string(),
                b"gpg content".to_vec(),
            );
        }

        let stage = MaterializeMirrorStage;
        stage.execute(state_dir, &mock).await?;

        // Check if files were created
        let files = mock.files.lock().await;
        let release_path = state_dir.join("mirror/debian/dists/bookworm/Release");
        let release_content = files
            .get(&release_path)
            .expect("Release file not downloaded");
        assert_eq!(release_content, b"release content");

        let gpg_path = state_dir.join("mirror/debian/dists/bookworm/Release.gpg");
        let gpg_content = files
            .get(&gpg_path)
            .expect("Release.gpg file not downloaded");
        assert_eq!(gpg_content, b"gpg content");

        Ok(())
    }

    #[tokio::test]
    async fn test_materialize_mirror_missing_manifest() -> Result<()> {
        let mock = MockEffects::new();
        let state_dir = Path::new("/state");

        let stage = MaterializeMirrorStage;
        let res = stage.execute(state_dir, &mock).await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("manifest.json not found")
        );
        Ok(())
    }
}
