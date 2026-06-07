# ARM64 OS-Development VM Builder for macOS

## Goal

CLI tool that runs on an Apple Silicon host and manages a clean-room virtual
machine supporting operating system development workflows on ARM64, such as
building an OS image from source and running an OS image under an emulator.

The primary development task, which is covered by integration tests, will be
developing ARM64-Linux host build tooling for Fuchsia. Verification entails
being able to build a `minimal.arm64` Fuchsia image and run it under QEMU. The
CLI tool will produce a VM capable of performing this verification.

Android (AOSP) development is analyzed as a secondary use case. Considering
both Fuchsia and Android highlights the aspects of the tool that may generalize
to other OS development tasks.

## Requirements

### Host for produced VM and CLI tool

1. Run on a macOS ARM64 (Apple Silicon) host.
2. Check for nested virtualization support. (Apple M3 or newer)
3. Assume the host has 64GB RAM or more.
4. Assume the host has 500GB free on the SSD.

### Produced VM image

5. Based on the latest stable Debian Server ARM64.
6. Can perform the setup required for OS development. Example: can perform
   the process for obtaining the Fuchsia code.
7. Can take advantage of nested virtualization. OS development tooling inside
   the VM must run hardware-accelerated ARM64 VMs under QEMU.
8. Can be used by a tool friendly to AI agents. (Example: `vfkit`.)
9. Provides observability for AI agents. (Example: serial console logging
   enabled and capturable by an AI-friendly tool.)
10. Can be used by a tool friendly to human developers, which may differ from
    the automation tool. (Example: `lima` for humans, `vfkit` for automation.)

### CLI tool source code

11. One Rust package that uses the 2024 edition.
12. All functionality in one library crate, wrapped by one binary crate that
    implements CLI argument parsing and output.
13. One integration test for each VM image requirement.
14. One integration test for each CLI tool feature under each VM tool.
    Example: check that the AI-friendly tool can be used to start a VM.
15. README enumerating all required external tools with Homebrew installation
    instructions.
16. Verified with tools that promote Rust best practices. Examples: `clippy`,
    `rustfmt`.
17. Unit tests for all functions and methods that are testable.
18. Minor architectural accomodations for increasing the unit tests coverage,
    without going overboard.

### CLI tool dependencies

19. Prefer Rust crates integrated into the tool over orchestrating external
    tools.
20. Do not use external tools whose licenses limit applicability. (Example: Caps
    on concurrent CPUs or users, forbidding commercial usage.)
21. Use external tools when absolutely necessary to avoid Apple entitlements and
    code signing.
22. Prefer external tools with permissive licensing (examples: Apache, MIT) over
    copyleft licensing (examples: LGPL, GPL, AGPL).
23. Use Rust crates with permissive licensing. Absolutely do not use crates
    with copyleft licenses, or crates whose licenses limit availability.
24. Prefer external tools with full automation support over friendlier GUIs that
    are less amenable to automation.

### CLI tool structure for VM packaging

25. VM packaging functionality structured as a sequence of stages.
26. The first stage determines the most up to date values for a minimal set of
    version / timestamp pins that can define the VM contents. (Example pin:
    timestamp for Debian package repository snapshot.)
27. All following stages are deterministic / idempotent / repeatable.
    When a stage succeeds, its output is completely determined by its inputs.
28. Each stage is cacheable. The stage is skipped if its outputs already exist.
29. The tool supports resetting to a specific stage by removing the outputs of
    all following stages.

### CLI tool operation for VM image production

30. Minimize access to external servers while testing and iterating. Use
    on-demand caching to avoid downloading a resource multiple times.
31. Split each operation that uses on-demand caching across a record step and a
    replay step. Example: The automated Debian installation process
    uses a package repository.
32. Verify against the Debian signing chain and refuse to proceed on mismatch.

### CLI tool features for using VM images

33. VM management: create, list, and delete VMs using a VM image.
34. VM operation: start VM, stop VM, SSH into the VM.
35. VM image management: fork VM (suspend + fork the VM image + resume).
36. Functionality covers both the AI-friendly and the human-friendly tool.

