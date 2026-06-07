use anyhow::Result;
use std::path::Path;

pub mod lima;
pub mod vfkit;

/// The VM Lifecycle Backend abstraction.
/// It provides a common interface for driving VMs using either `vfkit` (automation-friendly)
/// or `lima` (human-friendly).
pub trait Backend {
    /// Instantiate a VM from a golden image.
    ///
    /// For vfkit, this sets up the necessary launch configuration and a CoW clone of the disk.
    /// For lima, this generates a `lima.yaml` referencing the image and registers it with `limactl`.
    fn create(&self, vm_name: &str, image_path: &Path) -> Result<()>;

    /// Delete a VM and its associated disk image.
    fn delete(&self, vm_name: &str) -> Result<()>;

    /// Start a registered VM.
    ///
    /// Note: Both backends must ensure nested virtualization is enabled during launch.
    fn start(&self, vm_name: &str) -> Result<()>;

    /// Stop a running VM.
    fn stop(&self, vm_name: &str) -> Result<()>;

    /// Open an interactive SSH shell into the VM, or execute a command if provided.
    fn ssh(&self, vm_name: &str, command: Option<&[&str]>) -> Result<()>;

    /// List existing VMs.
    fn list(&self) -> Result<Vec<String>>;

    /// Fork an existing VM into a new independent VM.
    ///
    /// Suspends the running source VM, uses APFS CoW (`clonefile`) to clone its disk,
    /// and resumes both VMs.
    fn fork(&self, source_vm_name: &str, target_vm_name: &str) -> Result<()>;
}
