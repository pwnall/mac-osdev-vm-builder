# ARM64 OS-Development VM Builder for macOS

## Goal

A CLI tool that runs on an Apple Silicon host and manages clean-room ARM64 Linux
virtual machines for operating-system development workflows: building an OS from
source and running the result hardware-accelerated under an in-guest emulator or
virtual device.

The **primary, integration-tested use case is Fuchsia** — specifically, enabling
ARM64-Linux host build tooling, verified by building a `minimal.arm64` image and
running it under QEMU. The CLI tool produces a VM capable of performing that
verification, and the integration tests target it.

**Android (AOSP) is a second, example use case** (see *Use Cases*). The OS-agnostic
core — a reproducible, nested-virtualization-capable Debian VM plus the lifecycle
and packaging machinery — is shared; each OS differs only in four parameters: how
its source is obtained, which build prerequisites it needs, which emulator/virtual
device runs the built image under nested KVM, and what the verification target is.

---

## Requirements

### Host for produced VM and CLI tool

1. Run on a macOS ARM64 (Apple Silicon) host.
2. Check for nested virtualization support. (Apple M3 or newer)
3. Assume the host has 64 GB RAM or more.
4. Assume the host has 500 GB free on the SSD.

### Produced VM image

5. Based on the latest stable Debian Server ARM64.
6. Can perform the setup required for OS development. (Example: can perform the
   process for obtaining the Fuchsia code.)
7. Can take advantage of nested virtualization. OS development tooling inside the VM
   must run hardware-accelerated ARM64 VMs under QEMU.
8. Can be used by a tool friendly to AI agents. (Example: `vfkit`.)
9. Provides observability for AI agents. (Example: serial console logging enabled
   and capturable by an AI-friendly tool.)
10. Can be used by a tool friendly to human developers, which may differ from the
    automation tool. (Example: `lima` for humans, `vfkit` for automation.)

### CLI tool source code

11. One Rust package that uses the 2024 edition.
12. All functionality in one library crate, wrapped by one binary crate that
    implements CLI argument parsing and output.
13. One integration test for each VM image requirement.
14. One integration test for each CLI tool feature under each VM tool. (Example:
    check that the AI-friendly tool can be used to start a VM.)
15. README enumerating all required external tools with Homebrew installation
    instructions.
16. Verified with tools that promote Rust best practices. (Examples: `clippy`,
    `rustfmt`.)
17. Unit tests for all functions and methods that are testable.
18. Minor architectural accommodations for increasing unit-test coverage, without
    going overboard.

### CLI tool dependencies

19. Prefer Rust crates integrated into the tool over orchestrating external tools.
20. Do not use external tools whose licenses limit applicability. (Example: caps on
    concurrent CPUs or users, forbidding commercial usage.)
21. Use external tools when absolutely necessary to avoid Apple entitlements and
    code signing.
22. Prefer external tools with permissive licensing (Apache, MIT) over copyleft
    (LGPL, GPL, AGPL).
23. Use Rust crates with permissive licensing. Absolutely do not use crates with
    copyleft licenses, or crates whose licenses limit availability.
24. Prefer external tools with full automation support over friendlier GUIs that are
    less amenable to automation.

### CLI tool structure for VM packaging

25. VM packaging functionality structured as a sequence of stages.
26. The first stage determines the most up-to-date values for a minimal set of
    version / timestamp pins that can define the VM contents. (Example pin: timestamp
    for the Debian package repository snapshot.)
27. All following stages are deterministic / idempotent / repeatable. When a stage
    succeeds, its output is completely determined by its inputs.
28. Each stage is cacheable. The stage is skipped if its outputs already exist.
29. The tool supports resetting to a specific stage by removing the outputs of all
    following stages.

### CLI tool operation for VM image production

30. Minimize access to external servers while testing and iterating. Use on-demand
    caching to avoid downloading a resource multiple times.
31. Split each operation that uses on-demand caching across a record step and a
    replay step. (Example: the automated Debian installation process uses a package
    repository.)
32. Verify against the Debian signing chain and refuse to proceed on mismatch.

### CLI tool features for using VM images

33. VM management: create, list, and delete VMs using a VM image.
34. VM operation: start VM, stop VM, SSH into the VM.
35. VM image management: fork VM (suspend + fork the VM image + resume).
36. Functionality covers both the AI-friendly and the human-friendly tool.

*Scope note carried from earlier decisions: requirements 30–32 apply to **Debian
resources** (the image-production fetch). The OS source (e.g. Fuchsia or AOSP) and
any prebuilt-tool fetches are obtained inside the VM at usage time and are outside
the tool's caching/verification (their integrity rests on git, `repo`, and CIPD).*

---

## Use Cases

The tool is an OS-development workbench, not a Fuchsia-only utility. Everything in
*Architecture*, *VM Packaging Pipeline*, and *VM Lifecycle Commands* is OS-agnostic:
a pinned, signed, reproducible Debian Trixie ARM64 VM with a 16 KB kernel, nested
virtualization, serial observability, SSH, and CoW fork — managed identically across
vfkit and lima. A use case only customizes four parameters, all applied in
`provision-dev` and at run time, never in the VM substrate:

1. **Source fetch** — how the OS tree is obtained (inside the VM, outside the tool's
   Debian caching/verification scope; integrity rests on the OS's own VCS).
2. **Build prerequisites** — the apt package set installed before building.
3. **Emulator / virtual device** — what runs the built image hardware-accelerated
   under the guest's nested KVM.
4. **Verification target** — what "it works" means.

| Parameter | Fuchsia (primary, integration-tested) | Android / AOSP (example) |
|-----------|----------------------------------------|--------------------------|
| Source fetch | `jiri` bootstrap from `fuchsia.googlesource.com` | `repo init`/`repo sync` from `android.googlesource.com` (`android-latest-release`) |
| Build prereqs (apt) | `curl file unzip git python build-essential ca-certificates` | `git-core gnupg flex bison build-essential zip curl zlib1g-dev`, plus the `repo` launcher |
| Emulator under nested KVM | QEMU/FEMU (`ffx emu` / `fx qemu`) | Cuttlefish (`cvd`, crosvm) running an `aosp_cf_arm64_only_phone` image |
| Verification target | build `minimal.arm64`, boot under QEMU | build/obtain an `aosp_cf_arm64` image, boot under Cuttlefish |

**Why both fit the same VM.** Each OS targets ARM64 and runs hardware-accelerated
under an in-guest VMM that needs `/dev/kvm` — Fuchsia under QEMU, Android under
crosvm — which is exactly the nested-virtualization the VM provides (req 7). AOSP's
own requirements line up with the host assumptions: a Debian-based Linux host, 64 GB
RAM, ~400 GB disk (250 GB checkout + 150 GB build), `repo`+git. Cuttlefish explicitly
supports ARM64 hosts (`aosp_cf_arm64_only_phone` images, arm64 packages, `/dev/kvm`
on an arm64 machine).

**Honest caveat — the arm64-host-build parallel.** Both use cases share an in-progress
edge. Fuchsia's whole point here is *enabling* ARM64-Linux host build tooling, which
does not fully exist yet. Likewise, building AOSP *on* an arm64 host is **officially
unsupported** — Google states Android must be built on an x86-64 machine, since the
prebuilt host toolchains are x86-64 and Soong's `linux_arm64` is a cross-compile
*target*, not a host; arm64-host build support is community work-in-progress (e.g.
Linaro's "make AOSP self-hosting"). What *is* supported on an arm64 host today is
**running** arm64 Cuttlefish images under KVM. So Android exercises the same VM in
two honest ways: as a ready workbench for running/testing arm64 Android images under
nested KVM, and as a clean-room environment for the same kind of arm64-host-build
bootstrapping that motivates the Fuchsia case. The integration tests target Fuchsia;
Android is a documented example.

---

## Architecture and Tool Choices

### Virtualization substrate: Apple Virtualization.framework (vz)

The Fuchsia host port targets ARM64 Linux, so an ARM64 guest is the point.
ARM64-on-ARM64 through Apple's Virtualization.framework is near-native and avoids
QEMU on the host (a copyleft tool, per the dependency rules). Both launchers below
ride vz and EFI-boot the disk's own GRUB, so one image serves both.

### Nested virtualization (new, load-bearing)

The verification target — building `minimal.arm64` and running it
**hardware-accelerated under QEMU inside the guest** — requires nested
virtualization. Confirmed facts that constrain the host:

- Virtualization.framework exposes nested virtualization only on **M3/M4** chips,
  **macOS 15 (Sequoia) or later**, and **only for Linux guests**.
- It is a launch-time platform property (`isNestedVirtualizationEnabled` on
  `VZGenericPlatformConfiguration`), **disabled by default**, covered by the
  standard virtualization entitlement (no extra entitlement needed).
- **vfkit** supports it via a flag (added in vfkit; PR #327). **Lima** enables it
  by default on qualifying hardware.

Implications:

- The host preflight must verify chip (M3+) and macOS (15+) and that nested
  virtualization is supported, and fail early if not. This satisfies the host
  "check for nested virtualization support" requirement.
- Nested virtualization is enabled per launch (a launcher flag), but the **guest
  kernel must be 16 KB-page** for it to work on this hardware (see below). Both the
  automation launch (vfkit, explicit flag) and the human launch (lima, default-on)
  enable nesting; the image must carry the 16 KB kernel plus the in-guest QEMU/FEMU
  toolchain.
- macOS 15+ is now a hard requirement (previously only macOS 13+ for EFI boot).

**Preflight via `sysctl` (no entitlement needed).** Rather than calling the
entitlement-bound VZ API just to probe support, the preflight reads user-space
`sysctl` values as a pragmatic proxy: `hw.machine` = `arm64` (not under Rosetta),
`machdep.cpu.brand_string` matching `Apple M[3-9]`, `kern.hv_support` = 1, and
`kern.osrelease` ≥ `24.0.0` (Darwin 24 = macOS 15). No `sysctl` reports nested
virtualization directly, so "M3+ and macOS 15+" is the proxy; the definitive
`isNestedVirtualizationSupported` check happens implicitly when vfkit/Lima launch
with the flag set.

**Page-size / nested-KVM (confirmed: 16 KB *installed* kernel required).** Apple
Silicon is 16 KB-native, and on M4-class hardware nested virtualization requires the
**running guest kernel to use 16 KB pages**: a 16 KB kernel booted with `vfkit
--nested` works, while a 4 KB kernel under `--nested` panics the Apple hypervisor.
The installed VM therefore runs Debian Trixie's pre-compiled
**`linux-image-arm64-16k`** package (no custom kernel build). Crucially, only the
*installed* system needs 16 KB pages: because `--nested` is **off during the install
phase** (high disk-I/O `partman` formatting panics with it on), the **installer runs
the stock 4 KB netboot kernel as-is** and simply installs the 16 KB package to the
target disk via preseed. That keeps the fetch phase trivial — no initrd module
injection and no `anna/no_kernel_modules` workaround; the only kernel directive
left is the preseed line selecting `linux-image-arm64-16k` as the *target* kernel. Zircon's own 16 KB granule support is a
separate L2 concern handled by the Fuchsia build target.

### Automation launcher: vfkit (Apache-2.0)

The CLI orchestrates the `vfkit` binary rather than embedding
Virtualization.framework. This is now **explicitly sanctioned** by the dependency
rule "use external tools when absolutely necessary to avoid Apple entitlements and
code signing": in-process VF would require code-signing the tool with
`com.apple.security.virtualization` (and `com.apple.vm.networking` for bridged
networking), and ad-hoc signatures are machine-local and brittle. Homebrew's
`vfkit` is already signed and now exposes nested virtualization. Everything else in
the loop stays Rust-native (see *Crate & Tool Ledger*).

### Human-developer launcher: Lima (Apache-2.0) — a co-equal backend

Lima provides mounts, port-forwarding, SSH, and cloud-init provisioning for the
interactive path. Under the revised requirements it is **a full dependency and a
co-equal backend**: requirement 36 says the VM lifecycle features (create/list/
delete/start/stop/SSH/fork) must work through both the AI-friendly tool (vfkit) and
the human-friendly tool (lima). The tool generates a tailored `lima.yaml` (`vmType:
vz`,
**`nestedVirtualization: true`**) and drives `limactl` for the full set of lifecycle
commands, not merely a boot check. Driving `limactl` is the same external-tool
orchestration as vfkit (Apache-2.0, Homebrew-installed, already signed), consistent
with the dependency rules. Both vfkit and lima are required Homebrew tools. See *VM
Lifecycle Commands* for the shared backend abstraction.

### Rejected launchers

| Tool | Reason |
|------|--------|
| UTM | GUI-first, weak headless automation; bundles QEMU (GPL). |
| OrbStack | Proprietary/closed. |
| Multipass | GPL-3.0, QEMU on macOS, Ubuntu-only. |
| Tart | Fair Source 100 (CPU-core cap) — disallowed by the "no CPU/user caps" rule, even though it supports nested virtualization. Revisit only if its announced permissive relicense lands. |
| QEMU (as host hypervisor) | GPL; avoided via vz. (QEMU still runs *inside* the guest — see below.) |

### QEMU layering (apparent-contradiction note)

QEMU is avoided as the **host** hypervisor (copyleft dependency rule) but is
**required inside the guest** for FEMU / `fx qemu` to run the built `minimal.arm64`
image. These are different layers: the host hypervisor is vz; the guest emulator is
QEMU, part of the Fuchsia/Debian environment. The copyleft-avoidance rules govern
the CLI tool's host-side dependencies, not software inside the Debian guest (which
is GPL-laden by nature).

### Image creation: clean-room install via direct kernel boot

Boot the official Debian netboot installer kernel + initrd via `VZLinuxBootLoader`
(which accepts a cmdline), avoiding ISO repacking and therefore `xorriso` (GPLv3)
and fragile UEFI re-authoring. Source the kernel/initrd directly from the mirror's
`installer-arm64/.../netboot/` path (same signed provenance, no ISO parsing). The
arm64 netboot `linux` is already an uncompressed `Image`, so it boots as-is — no
decompression needed. If a single signed-ISO provenance anchor is later required,
read the two files in-process with `hadris-iso` (MIT, `read` feature) instead.

Two-phase boot: install phase (`VZLinuxBootLoader` + stock 4 KB netboot kernel +
initrd + preseed cmdline + `console=hvc0`) → run phase (`VZEFIBootLoader` + installed
raw
disk; also how Lima boots it).

### Portable golden image contract

- **Boots under vfkit and Lima:** raw format, GPT, an ESP, GRUB-efi to the
  removable path (`\EFI\BOOT\BOOTAA64.EFI`), ext4 root, `console=hvc0`.
- **First-class Lima citizen:** cloud-init + `openssh-server` + `sudo` + `sshfs` +
  `uidmap` (systemd is default on Debian).
- **Robust across launchers:** `root=UUID=…` and UUID fstab; DHCP on the virtio NIC
  matched by driver; cloud-init `datasource_list: [NoCloud, None]`.

### Package access: in-process record/replay mirror

The caching mirror is a module in the library crate (not an external daemon),
satisfying both the single-package/crate-preference rules and the on-demand caching
+ record/replay requirement:

- **Record step:** one warm install proxies d-i's requests and writes a manifest /
  lockfile of `{url, sha256, fetched_at}` for every udeb, base, and target package
  plus the signed apt metadata (`InRelease`/`Release`/`Release.gpg`). It fetches
  against the **`snapshot.debian.org` timestamp pinned in stage 0**, so within a
  snapshot the metadata and packages are a consistent, immutable set and the record
  output is reproducible. (`snapshot.debian.org` serves timestamped archives at
  `/archive/debian/<YYYYMMDDTHHMMSSZ>/`; the pinned URL is what every later fetch
  uses.)
- **Replay step:** pre-download exactly the manifest, verify each hash, serve a
  static offline mirror, and fail closed on anything unlisted.

### Verification and provenance (Debian chain)

- **Trust anchor:** embed pinned Debian archive signing-key fingerprints (or a
  pinned `debian-archive-keyring`); verification against a runtime-fetched key would
  be circular.
- **Tool-side (kernel/initrd):** verify artifact hash → installer `SHA256SUMS` →
  signed `Release`/`InRelease` in process, using **`rpgp` (MIT/Apache)**, not
  `sequoia-openpgp` (LGPL — now explicitly disfavored).
- **Metadata parsing in-Rust:** parse `Release`/`Packages` to extract per-package
  SHA-256 checksums without shelling to `apt`/`dpkg`. A native crate such as
  `debian-packaging` (MIT/Apache — confirm current maintenance) covers the deb822
  formats; the format is simple enough to parse directly if the crate is unsuitable.
- **Guest-side (packages):** serve signed apt metadata intact so apt-secure inside
  d-i verifies independently. Refuse on any mismatch.
- **Scope (by decision):** record/replay caching and signing-chain verification
  cover **Debian resources only**. The Fuchsia source and CIPD prebuilts are
  obtained inside the VM, outside the tool's caching and verification. To keep that
  decision from reintroducing heavy external access on every iteration, persist the
  in-guest Fuchsia checkout and CIPD cache in the blessed image (or a persistent
  data volume) so `clonefile` clones inherit them and avoid re-fetching from Google.

---

## VM Packaging Pipeline (requirements 25–32)

This is the staged image-production flow. The **first stage is the only
non-deterministic one**: it checks the host and resolves the latest Debian inputs
into a minimal set of pins (a lockfile). Every later stage consumes those pins and
is deterministic — its output is fixed by its inputs. Stages are cacheable: an
output is reused only when it exists **and** was produced from the current inputs
(cache key includes input hashes), so changing an input (or the pins) invalidates
that stage and everything downstream. A **reset to a stage** removes the outputs of
that stage and all following stages; resetting to stage 0 refreshes the pins to
newer Debian versions. The pipeline ends at a blessed golden image; runtime VM
operations live in *VM Lifecycle Commands*, and the pipeline reuses those same
commands rather than carrying its own VM-driving logic.

| # | Stage | Output / Effect |
|---|-------|-----------------|
| 0 | `resolve` *(not deterministic)* | Host preflight via `sysctl` (`arm64`, Apple M3+, `kern.hv_support`=1, macOS 15+) **and** resolve the minimal Debian pins: the suite (latest stable Debian, currently Trixie / 13), the current `snapshot.debian.org` timestamp, the netboot kernel/initrd, and the archive key fingerprints. The only stage that depends on host/upstream state; emits a lockfile all later stages consume. |
| 1 | `fetch-installer` | Against the pinned snapshot: download + verify the stock netboot kernel & initrd (hash → `SHA256SUMS` → signed `Release`, pinned key); record provenance. The arm64 netboot `linux` is **already an uncompressed `Image`** and boots as-is — no decompression, no initrd modification. (The 16 KB target kernel is installed by d-i from the mirror, captured during `record-manifest`.) |
| 2 | `record-manifest` *(record; rare)* | Warm install through the in-process proxy against the **pinned snapshot**; capture `{url, sha256, fetched_at}` for packages + signed metadata, including the `linux-image-arm64-16k` package the preseed selects. |
| 3 | `materialize-mirror` *(replay)* | Pre-download the manifest, verify hashes, build the static offline mirror with signed metadata intact. |
| 4 | `create-disk` | Sparse raw target disk, sized for a full Fuchsia build (see caveats). |
| 5 | `install` | Boot d-i via vfkit **with `--nested` OFF** (nested + high-I/O `partman` panics): linux bootloader with the **stock 4 KB netboot kernel** + initrd, allocated **PTY** for serial; preseed cmdline `console=hvc0`; preseed `base-installer/kernel/image=linux-image-arm64-16k` (installs the 16 KB kernel to the target); preseed `late_command` injects a `/dev/kvm` udev rule (below); apt → local mirror, fail-closed; capture serial; await completion sentinel; **power off**. |
| 6 | `provision-dev` | Via the lifecycle commands (`create`/`start`/`ssh`): install the selected use case's build prerequisites and in-guest emulator toolchain (Fuchsia + QEMU/FEMU by default; see *Use Cases*) so the VM is *capable* of the build+run verification. |
| 7 | `verify` | Run the integration-test suite (reqs 13–14: one test per VM image requirement 5–10, plus one per lifecycle feature × backend) against the candidate image through the VM management commands, **with `--nested` ON**, on both backends; all must pass. This is where boot/SSH and the L2 nested-KVM smoke test confirm the image before blessing. |
| 8 | `bless` | Promote the verified candidate to the golden VM image. |

Fuchsia prerequisites (the default use case, part of the verified Debian fetch):
`curl`, `file`, `unzip`, `git` (≥ 2.31), `python`, `build-essential`,
`ca-certificates`, plus QEMU/KVM userspace for in-guest FEMU. Other use cases swap
this set per *Use Cases* (e.g. AOSP adds `gnupg flex bison zip zlib1g-dev` and the
`repo` launcher, with Cuttlefish/crosvm as the emulator).

**Per-phase `--nested`:** off during `install` (nested + high-I/O `partman` panics),
on for `provision-dev`, `verify`, and all runtime use. The flag is a launch
parameter, so the same image is reused across phases.

**`/dev/kvm` access in the guest:** by default the dev user cannot open `/dev/kvm`,
so nested QEMU and the req-7 smoke test fail with "Permission denied." The install
`late_command` drops a udev rule — `SUBSYSTEM=="misc", KERNEL=="kvm", MODE="0666"` —
into the golden image so `qemu` runs without `sudo`. (Adding the user to the `kvm`
group is the tighter alternative if world-access is undesirable.)

---

## VM Lifecycle Commands (requirements 33–36)

These are the runtime commands that operate on a golden image, and **all must work
through both backends** (vfkit for the AI path, lima for the human path — req 36).
They are the single source of truth for VM driving: the packaging pipeline
(`provision-dev`, `verify`) and the integration tests call this same API rather than
re-implementing create/start/SSH. The library exposes one `Backend` abstraction with
two implementations so the CLI surface is backend-agnostic:

- **`create` / `list` / `delete`** (req 33): instantiate a VM from a golden image
  (vfkit: a launch spec + a CoW disk; lima: a generated `lima.yaml` referencing the
  image), enumerate existing VMs, and tear one down.
- **`start` / `stop` / `ssh`** (req 34): run/halt the VM and open a shell — over the
  gvisor-tap-vsock host→guest:22 forward for vfkit, and `limactl shell`/SSH for lima.
- **`fork`** (req 35): **suspend** the running VM, **`clonefile`** (APFS CoW) its
  current disk image into a new independent image, then **resume** both. Suspending
  before the clone gives a consistent disk (resolving the earlier quiesce question),
  and CoW makes the fork cheap — the permissive replacement for Tart's clones.

Backend notes: nested virtualization must be enabled per-launch (vfkit flag; lima
`nestedVirtualization: true`); SSH reachability differs (vsock forward vs. lima's
built-in); fork relies on the image being a single raw file so `clonefile` applies
uniformly; and the suspend/resume step uses each backend's pause/resume (vfkit's
VZ pause/resume; for lima, its stop/start if live suspend is unavailable).

**CLI surface.** A `--backend {vfkit|lima}` flag selects the backend for any
lifecycle command (default `vfkit` for the agent path). Packaging is driven by a
`package` subcommand group (run/resume stages, `reset <stage>`, `status`); the
lifecycle verbs (`create`, `list`, `delete`, `start`, `stop`, `ssh`, `fork`) are the
rest. Per req 12 the binary crate is a thin parsing/output shell; every capability is
a library function it calls — the same functions the pipeline and integration tests
use.

---

## Integration Tests (requirements 13–14)

Two test families, both written against the **VM management commands**
(`create`/`list`/`delete`/`start`/`stop`/`ssh`/`fork`) rather than bespoke
VM-driving — so the tests exercise the same code paths the tool ships. They run
against the candidate image and are invoked by the `verify` stage (and standalone
via `cargo test`).

**Per VM image requirement (req 13)** — one test each for reqs 5–10. Reqs 5–7 and 9
are backend-agnostic and run on the default backend:

| Req | Test (via the lifecycle commands) |
|-----|-----|
| 5 | `create`+`start`+`ssh`, then assert Debian Server **arm64** (`/etc/os-release`, `dpkg --print-architecture`). |
| 6 | Over `ssh`, assert the Fuchsia-obtaining prerequisites are present and the `jiri` bootstrap can start (prereqs + bootstrap init) — **not** a full multi-GB checkout. |
| 7 | Over `ssh`, run the L2 QEMU/KVM smoke test as the unprivileged dev user (the `/dev/kvm` udev rule makes this work without `sudo`): boot a tiny ARM64 image under `qemu -accel kvm` and confirm hardware acceleration (the req-7 capability, not a Fuchsia build). |
| 8 | Confirm the image is usable by the AI-friendly tool (vfkit) — covered by the req-14 matrix below. |
| 9 | During `start`, assert serial-console output is captured to a log. |
| 10 | Confirm the image is usable by the human-friendly tool (lima) — covered by the req-14 matrix below. |

**Per CLI feature × VM tool (req 14)** — one test for each lifecycle feature on each
backend, which is also what proves reqs 8, 10, and 33–36:

| Feature | vfkit | lima |
|---------|-------|------|
| create / list / delete | ✓ | ✓ |
| start / stop | ✓ | ✓ |
| ssh | ✓ | ✓ |
| fork (suspend + clone + resume) | ✓ | ✓ |

The full `minimal.arm64` build-and-run described in the Goal remains a manual /
nightly **acceptance** check (hours, tens of GB), not a routine integration test.

---

## Code Quality & Unit Tests (requirements 16–18)

Alongside the integration tests, the source meets standard Rust-hygiene bars:

- **Lint and format gates (req 16).** `cargo clippy` (warnings denied in CI) and
  `cargo fmt --check` run on every build; the README documents both.
- **Unit tests for testable logic (req 17).** The pure, I/O-free logic carries unit
  tests independent of any VM or network: pin/lockfile resolution given fetched
  metadata, `Release`/`Packages` parsing and checksum extraction, the
  signature-chain check (hash → `SHA256SUMS` → signed `Release`), preseed and
  `lima.yaml` generation, cache-key computation, and the stage-graph / reset logic.
- **Architecture for testability, in moderation (req 18).** Two seams make the above
  unit-testable without overbuilding: the existing `Backend` trait (vfkit/lima),
  which a mock backend can stand in for, and a thin trait over external effects
  (process execution, HTTP, filesystem) so record/replay and verification can be
  driven against fixtures. Real VM behavior stays in the integration tests (reqs
  13–14) — the unit layer covers determinism and parsing/verification, not booting.
  "Without going overboard" means no abstraction whose only purpose is a coverage
  number; the lib/bin split (req 12) already keeps the binary a thin shell so the
  library is testable directly.

---

## Automation Caveats

- **Nested virtualization needs a 16 KB-page *installed* kernel.** Enable nesting
  per launch (vfkit flag; Lima default-on on M3+/macOS 15); the installed VM must run
  Trixie's `linux-image-arm64-16k`, since a 4 KB kernel with `--nested` panics the
  Apple hypervisor on M4-class hardware. The *installer* runs the stock 4 KB netboot
  kernel (`--nested` is off then), so only the target kernel is 16 KB.
- **`--nested` off during install, on afterward.** With nesting on, high disk-I/O
  `partman` formatting triggers intermittent hypervisor panics; install with it off
  and enable it for `provision-dev`, `verify`, and runtime.
- **vfkit needs a PTY for serial.** A `virtio-serial` device bound to `stdio` fails
  with "inappropriate ioctl for device" when launched from a bare
  `std::process::Command`. Allocate a PTY (a Rust pty crate such as `portable-pty`,
  preferred per req 19; or wrap in `script -q <log> sh -c "vfkit …"`, which also
  captures the serial log).
- **The arm64 netboot kernel needs no decompression.** Unlike x86's gzip-compressed
  `vmlinuz`, Debian's arm64 netboot `linux` is already an uncompressed `Image` and
  boots directly under VZLinuxBootLoader — no `gunzip`/`flate2` step.
- **`/dev/kvm` permissions:** the dev user can't open `/dev/kvm` by default, breaking
  nested QEMU and the req-7 test with "Permission denied." The install `late_command`
  installs a udev rule (`SUBSYSTEM=="misc", KERNEL=="kvm", MODE="0666"`) so `qemu`
  runs without `sudo` (or add the user to the `kvm` group).
- **macOS 15+ / M3+ is mandatory** for the verification path. Preflight must
  enforce it.
- **The in-guest emulator is expected and GPL** — QEMU/FEMU (Fuchsia) or crosvm
  (Android Cuttlefish) lives inside the Debian guest, not among the CLI's host-side
  dependencies; the copyleft rules do not reach it.
- **Disk and RAM are now budgeted, not improvised.** The host is assumed to have
  ≥ 64 GB RAM and ≥ 500 GB free SSD (reqs 3–4), so allocate the VM generous cores
  and RAM for a full Fuchsia build plus a from-source host-clang bootstrap (ARM64
  host prebuilts do not exist yet — that is the project) plus nested QEMU. Budget
  the 500 GB across the golden image, its forks (CoW, so cheap), the in-guest
  Fuchsia checkout and build output, and the Debian package mirror. Size the raw
  disk for the full build (well into the hundreds of GB) within that envelope.
- **Install-success sentinel:** a preseed `late_command` writes a marker (tagged
  virtio-serial/vsock or localhost callback); do not scrape the console for "done."
- **Power off, not reboot:** preseed `d-i debian-installer/exit/poweroff boolean
  true`, then switch to the EFI run-phase.
- **Console:** `console=hvc0` on both the installer cmdline and the installed
  system (`d-i debian-installer/add-kernel-opts string console=hvc0`).
- **GRUB removable path:** `grub-installer/force-efi-extra-removable boolean true`
  so either launcher's (often empty) EFI NVRAM still boots the disk.
- **ESP** must be in the `partman-auto` recipe; **UUID** references in fstab and
  GRUB; **generic DHCP** on the virtio NIC.
- **Case-sensitivity:** keep the OS source tree (Fuchsia, AOSP, …) on the guest
  ext4; never share it from APFS over virtio-fs.
- **Headless observability (agent path):** capture serial to a timestamped log;
  emit machine-readable status; hard timeouts; the SSH path under vfkit requires a
  host→guest:22 forward via gvisor-tap-vsock (Lima provides SSH for free).
- **Discover the NAT gateway; don't hardcode `10.0.2.2`.** That address is the
  QEMU/slirp convention; vfkit's gvisor-tap-vsock uses a different subnet. The
  preseed proxy/mirror URL must use the launcher's actual gateway, resolved at run
  time.
- **Determinism:** the stage-0 snapshot pin plus the recorded manifest are what make
  the record/replay stages satisfy "output determined by inputs."

---

## Crate & Tool Ledger

Every Rust crate is permissively licensed (MIT/Apache) with no copyleft and no
availability-limiting terms (req 23); every external tool is permissive too, with
the entitlement-driven exceptions called out below (reqs 20–22, 24). The one place
this actively steered a choice was OpenPGP: `rpgp` (MIT/Apache) over
`sequoia-openpgp` (LGPL).

### Used (all permissive)

| Component | Role | License |
|-----------|------|---------|
| vfkit | AI-path VM backend (orchestrated; sanctioned to avoid entitlements) | Apache-2.0 |
| Lima | Human-path VM backend; co-equal lifecycle backend (driven via `limactl`) | Apache-2.0 |
| `hyper`/`axum` + `reqwest` + `tokio` | In-process record/replay mirror | MIT / MIT-Apache |
| `portable-pty` | Allocate a PTY for vfkit's `virtio-serial`/stdio + serial capture | MIT |
| `sha2` | Hashing / provenance | MIT/Apache |
| `rpgp` | OpenPGP verification | MIT/Apache |
| `debian-packaging` | Parse `Release`/`Packages`, extract checksums (verify maintenance) | MIT/Apache |
| `hadris-iso` (`read`) | ISO read, only if ISO-anchored | MIT |
| `clonefile` via `std`/`libc` | CoW disk clones | std/permissive |

Required external tools (Homebrew): **vfkit** (AI-path backend) and **lima**
(human-path backend). Both are Apache-2.0 and pre-signed, carrying the entitlement
burden so the Rust binary does not. Both are driven for the full VM lifecycle
(req 36).

### Avoided

- **Copyleft:** QEMU on host (GPL), `xorriso` (GPLv3), apt-cacher-ng (AGPLv3),
  Squid (GPL), Multipass (GPL-3), `sequoia-openpgp` (LGPL).
- **License caps / proprietary:** Tart (Fair Source 100 — a CPU-core cap, exactly
  the kind of usage-limiting license req 20 forbids alongside non-commercial terms),
  OrbStack (proprietary). Terraform/Packer (BUSL) if a cloud path is ever added →
  OpenTofu (MPL-2.0).
- **Unavoidable / not a CLI dependency:** git, base Debian userland, Linux kernel,
  and **QEMU inside the guest** (FEMU) are GPL but intrinsic to the Debian/Fuchsia
  environment, not host-side choices.

---

## Resolved Decisions

- **Caching/verification scope is Debian-only.** Record/replay caching and
  Debian-signing-chain verification apply only to Debian resources (image
  production). The OS source (Fuchsia via `jiri`, AOSP via `repo` — both
  Google-hosted) and any prebuilt-tool fetches are obtained inside the VM at usage
  time, outside the tool's caching and verification; their integrity rests on git,
  `repo`, and CIPD. To avoid re-hitting Google on every iteration, persist the
  in-guest checkout and tool caches in the golden image or a persistent volume so
  forks inherit them.
- **Determinism via a non-deterministic first stage.** Stage 0 resolves a minimal
  pin set (notably a `snapshot.debian.org` timestamp); it alone is non-deterministic
  (reqs 26–27). Every later stage consumes the pins and is deterministic.
- **Lima and vfkit are co-equal, required backends.** Behind one `Backend`
  abstraction, the full VM lifecycle (create/list/delete/start/stop/SSH/fork,
  reqs 33–36) is exercised through both. Both are required Homebrew tools.
- **Reset removes downstream outputs** (req 29); resetting to stage 0 refreshes pins.
- **Resource feasibility is now an assumption, not a risk.** Reqs 3–4 assume ≥ 64 GB
  RAM and ≥ 500 GB free SSD, which is enough for a full Fuchsia build, a from-source
  host-clang bootstrap, nested QEMU, the golden image, CoW forks, and the Debian
  mirror. The earlier laptop-resource worry is retired; what remains is *budgeting*
  the 500 GB and allocating the VM generous cores/RAM.
- **Integration-test scope is bounded by reqs 13–14** — one test per VM image
  requirement (5–10) plus one per lifecycle feature × backend, each kept light (the
  req-7 test is the L2 KVM smoke test, not a Fuchsia build). The full `minimal.arm64`
  build-and-run stays a manual/nightly acceptance check, so the earlier "tests can't
  cover the full build" tension is resolved by scoping.
- **Page-size is settled, and the installer stays simple.** Nested virtualization on
  M4-class hardware requires a 16 KB *installed* kernel (a 4 KB kernel with
  `--nested` panics), so the VM runs Trixie's pre-compiled `linux-image-arm64-16k`.
  Because `--nested` is off during install, the installer runs the **stock 4 KB
  netboot kernel as-is** and just installs the 16 KB package to the target — the A/B
  test confirmed this, so there is no initrd module injection, no `anna` override,
  and no kernel decompression.
- **`/dev/kvm` is made accessible** to the dev user via a udev rule
  (`MODE="0666"`) dropped in the install `late_command`, so nested QEMU and the req-7
  test run without `sudo`.
- **Fork semantics are defined** (req 35): suspend the VM, `clonefile` its image,
  resume — suspending before the clone gives a consistent disk, which settles the
  earlier quiesce question.

## Open Issues

1. **Lima suspend/resume capability for `fork` (minor).** Req 35's fork is
   suspend + clone + resume; vfkit (VZ) exposes pause/resume directly, but Lima may
   only offer stop/start. If Lima lacks live suspend, the lima-backend `fork` falls
   back to stop → clone → start, which is correct but not live. Confirm during
   implementation; not a blocker.
