# Implementation Notes

This document captures the key technical learnings and workarounds discovered during the implementation of the `mac-osdev-vm-builder`, which diverge from or expand upon the original `implementation-guide.md`.

## 1. Resolution of Open Issue 1: Lima Fork Implementation
- **New Learning:** The architecture guide hypothesized that Lima fork might fall back to stop -> clone -> start if live suspend isn't available. We discovered that `limactl` identifies instances entirely by directory structure. We can fork a Lima VM instantaneously by using `cp -cR ~/.lima/<source_vm> ~/.lima/<target_vm>`. This natively utilizes APFS cloning for the large 100GB disk, completely bypassing the need for `limactl create` (which would have slowly copied and re-converted the disk).

## 2. Suspending `vfkit` VMs for Forking
- **New Learning:** The guide states we should suspend the VM for a consistent `clonefile` fork. However, `vfkit` does not expose a native CLI command or API to suspend/resume VMs gracefully.
- **Solution:** We successfully suspend running `vfkit` VMs at the OS level by sending `SIGSTOP` (`kill -STOP <pid>`) to the underlying hypervisor process, executing our `clonefile`, and then resuming execution with `SIGCONT` (`kill -CONT <pid>`).

## 3. Package and Tool Consolidation
- **Divergence from Guide:** While implicit in some iterations, based on user preference, the library and CLI crates have been consolidated into a single root Rust package named `mac-osdev-vm-builder`. This structure simplifies dependency management while maintaining the clean separation between library internals and the CLI parsing binary.
