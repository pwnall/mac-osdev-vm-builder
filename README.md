# ARM64 OS-Development VM Builder for macOS

This tool automates the creation of a clean-room Debian ARM64 virtual machine on
an Apple Silicon host for operating system development.

## Requirements

The tool assumes a macOS ARM64 (M3 or newer) host with nested virtualization
support.

The tool depends on external tools for the actual VM execution. These are
required to avoid complex Apple code-signing and entitlement requirements for
in-process Virtualization framework usage.

### Installing Dependencies

You must install the following tools before running this builder:

```sh
brew install vfkit lima
```

[vfkit](https://github.com/crc-org/vfkit) is recommended for automated, headless
execution. This tool uses `vfkit` to build the VM image.

[Lima](https://github.com/lima-vm/lima) is recommended for human developers to
interact with the resulting VM image.

## Usage

Download and build from source:

```sh
cargo install --git https://github.com/pwnall/mac-osdev-vm-builder
```

Alternatively, build a checked out version:

```sh
cargo build --release
```

See available commands:

```sh
mac-osdev-vm-builder --help
```

### Automated Build Pipeline

Run the complete sequence of stages (fetch installer, create disk, install,
validate, bless golden image):

```sh
mac-osdev-vm-builder package build
```

The tool uses on-demand caching. Stages are idempotent and will be skipped if
they have already been completed successfully.

### Ephemeral Clones

Once you have a blessed golden image, you can create extremely fast, cheap
clones using APFS `clonefile`:

```sh
mac-osdev-vm-builder vm create my-clone
```

Start the clone using the default `vfkit` backend:

```sh
mac-osdev-vm-builder vm start my-clone
```

Start the clone using a different backend (currently, Lima) using the:

```sh
mac-osdev-vm-builder vm start my-clone --backend lima
```

When you're finished, cleanly stop the VM:

```sh
mac-osdev-vm-builder vm stop my-clone
```

## Development

### Running the Tests

Make sure that the tests pass when making changes:

```sh
cargo test
```

The repository includes a comprehensive integration test suite that exercises
the full VM build pipeline and lifecycle management.

The test suite automatically generates a "golden" base image upon its first run,
caching it in the local `./test_state_integration` directory to speed up
subsequent test executions.

### Documentation

The `docs/` folder has Markdown documentation for the project.

Current artifacts:

* `requirements.md`: Human-written project objective and requirements
* `implementation-guide.md`: AI-written implementation design document, written
  based on Web research and implementation feedback
* `implementation-notes.md`: AI-written feedback from attempting to implement
  previous versions of the implementation guide
* `retrospective.md`: AI-written conclusions from the conversation between the
  project's human author and the main AI designer

### Architecture

The project is structured as a combined library and binary crate:

* **Library Crate (`src/lib.rs`)**: Contains the core logic and stage
  definitions for the build pipeline and VM management. Designed for reuse and
  testability.

* **Binary Crate (`src/main.rs`)**: Acts as a thin wrapper around the library,
  providing the CLI interface (using `clap`), parsing arguments, and formatting
  output strings for the user.

#### Class-Level Architecture

The core pipeline is built around a `Stage` trait that models idempotent,
resumable build steps.

* `Stage` (Trait): Defines the interface for a pipeline step, primarily the
  `execute(&mut self)` method.
* **Build Stages**: The VM build process is composed of sequential stages:
  - `ResolveStage`: Determines the appropriate Debian installer ISO URL.
  - `FetchInstallerStage`: Downloads the ISO.
  - `RecordManifestStage`: Records expected hashes.
  - `MaterializeMirrorStage`: Creates local mirrors if necessary.
  - `CreateDiskStage`: Allocates the virtual disk image.
  - `InstallStage`: Boots the VM with the installer and runs the automated
    Debian preseed installation.
  - `ProvisionDevStage`: Boots the installed VM to configure OS development
    prerequisites (e.g., nested KVM).
  - `VerifyStage`: Validates the configured image.
* **Side Effects (`src/effects.rs`)**: Abstracts file system and process
  execution for easier testing and mocking.
* **Virtual Machine Backends (`src/backend/`)**: Provides abstractions over the
  underlying hypervisor tools.
  - `VfkitBackend`: Manages VMs using the native `vfkit` tool.
  - `LimaBackend`: Manages VMs using `limactl`.
