use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::process::Output;

#[async_trait]
pub trait Effects: Send + Sync {
    async fn fs_read(&self, path: &Path) -> Result<Vec<u8>>;
    async fn fs_read_to_string(&self, path: &Path) -> Result<String>;
    async fn fs_write(&self, path: &Path, contents: &[u8]) -> Result<()>;
    async fn fs_create_dir_all(&self, path: &Path) -> Result<()>;
    async fn fs_remove_file(&self, path: &Path) -> Result<()>;
    async fn fs_exists(&self, path: &Path) -> bool;
    async fn fs_canonicalize(&self, path: &Path) -> Result<std::path::PathBuf>;

    async fn http_get(&self, url: &str) -> Result<Vec<u8>>;

    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<Output>;
}

pub struct RealEffects;

#[async_trait]
impl Effects for RealEffects {
    async fn fs_read(&self, path: &Path) -> Result<Vec<u8>> {
        Ok(tokio::fs::read(path).await?)
    }

    async fn fs_read_to_string(&self, path: &Path) -> Result<String> {
        Ok(tokio::fs::read_to_string(path).await?)
    }

    async fn fs_write(&self, path: &Path, contents: &[u8]) -> Result<()> {
        Ok(tokio::fs::write(path, contents).await?)
    }

    async fn fs_create_dir_all(&self, path: &Path) -> Result<()> {
        Ok(tokio::fs::create_dir_all(path).await?)
    }

    async fn fs_remove_file(&self, path: &Path) -> Result<()> {
        Ok(tokio::fs::remove_file(path).await?)
    }

    async fn fs_exists(&self, path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    async fn fs_canonicalize(&self, path: &Path) -> Result<std::path::PathBuf> {
        Ok(tokio::fs::canonicalize(path).await?)
    }

    async fn http_get(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = reqwest::get(url).await?.bytes().await?;
        Ok(bytes.to_vec())
    }

    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<Output> {
        let mut command = tokio::process::Command::new(cmd);
        command.args(args);
        Ok(command.output().await?)
    }
}

#[cfg(test)]
pub struct MockEffects {
    pub files: tokio::sync::Mutex<std::collections::HashMap<std::path::PathBuf, Vec<u8>>>,
    pub command_outputs: tokio::sync::Mutex<std::collections::HashMap<String, Output>>,
    pub http_responses: tokio::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

#[cfg(test)]
impl Default for MockEffects {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl MockEffects {
    pub fn new() -> Self {
        Self {
            files: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            command_outputs: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            http_responses: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl Effects for MockEffects {
    async fn fs_read(&self, path: &Path) -> Result<Vec<u8>> {
        let files = self.files.lock().await;
        files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("File not found in mock"))
    }

    async fn fs_read_to_string(&self, path: &Path) -> Result<String> {
        let data = self.fs_read(path).await?;
        Ok(String::from_utf8(data)?)
    }

    async fn fs_write(&self, path: &Path, contents: &[u8]) -> Result<()> {
        let mut files = self.files.lock().await;
        files.insert(path.to_path_buf(), contents.to_vec());
        Ok(())
    }

    async fn fs_create_dir_all(&self, _path: &Path) -> Result<()> {
        Ok(())
    }

    async fn fs_remove_file(&self, path: &Path) -> Result<()> {
        let mut files = self.files.lock().await;
        files.remove(path);
        Ok(())
    }

    async fn fs_exists(&self, path: &Path) -> bool {
        let files = self.files.lock().await;
        files.contains_key(path)
    }

    async fn fs_canonicalize(&self, path: &Path) -> Result<std::path::PathBuf> {
        Ok(path.to_path_buf()) // Simplified
    }

    async fn http_get(&self, url: &str) -> Result<Vec<u8>> {
        let resps = self.http_responses.lock().await;
        resps
            .get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("URL not found in mock"))
    }

    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<Output> {
        let full_cmd = format!("{} {}", cmd, args.join(" "));
        let cmds = self.command_outputs.lock().await;
        cmds.get(&full_cmd)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Command not found in mock: {}", full_cmd))
    }
}
