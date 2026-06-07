use crate::stage::Stage;
use anyhow::{Result, anyhow};
use std::path::Path;

unsafe extern "C" {
    fn clonefile(src: *const libc::c_char, dst: *const libc::c_char, flags: u32) -> libc::c_int;
}

pub fn clone_file(src: &Path, dst: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c_src = std::ffi::CString::new(src.as_os_str().as_bytes())?;
    let c_dst = std::ffi::CString::new(dst.as_os_str().as_bytes())?;

    let res = unsafe { clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) };

    if res != 0 {
        return Err(anyhow::anyhow!(
            "clonefile failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

pub struct BlessStage {
    pub disk_path: std::path::PathBuf,
}

impl Stage for BlessStage {
    fn name(&self) -> &'static str {
        "bless"
    }

    async fn execute(
        &self,
        state_dir: &Path,
        _effects: &dyn crate::effects::Effects,
    ) -> Result<()> {
        let golden_path = state_dir.join("golden.raw");
        println!("Blessing current disk as golden image: {:?}", golden_path);

        if golden_path.exists() {
            tokio::fs::remove_file(&golden_path).await?;
        }

        use std::os::unix::ffi::OsStrExt;
        let c_src = std::ffi::CString::new(self.disk_path.as_os_str().as_bytes())?;
        let c_dst = std::ffi::CString::new(golden_path.as_os_str().as_bytes())?;

        let res = unsafe { clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) };

        if res != 0 {
            return Err(anyhow::anyhow!(
                "clonefile failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        println!("Successfully blessed image.");
        Ok(())
    }
}

pub struct CloneStage {
    pub target_path: std::path::PathBuf,
}

impl Stage for CloneStage {
    fn name(&self) -> &'static str {
        "clone"
    }

    async fn execute(
        &self,
        state_dir: &Path,
        _effects: &dyn crate::effects::Effects,
    ) -> Result<()> {
        let golden_path = state_dir.join("golden.raw");
        if !golden_path.exists() {
            return Err(anyhow!(
                "Golden image not found. Must run bless stage first."
            ));
        }

        if self.target_path.exists() {
            return Err(anyhow!(
                "Target path already exists: {:?}",
                self.target_path
            ));
        }

        println!("Cloning golden image to {:?}", self.target_path);

        use std::os::unix::ffi::OsStrExt;
        let c_src = std::ffi::CString::new(golden_path.as_os_str().as_bytes())?;
        let c_dst = std::ffi::CString::new(self.target_path.as_os_str().as_bytes())?;

        let res = unsafe { clonefile(c_src.as_ptr(), c_dst.as_ptr(), 0) };

        if res != 0 {
            return Err(anyhow::anyhow!(
                "clonefile failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        println!("Successfully cloned image to {:?}", self.target_path);
        Ok(())
    }
}
