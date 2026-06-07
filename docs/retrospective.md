# Building the ARM64 OS-Development VM Builder: A Retrospective

## Why this document exists

This is the companion to the implementation plan. The plan states *what* to build;
this document records *how* that design was arrived at — the research that grounded
it, a cross-check against a second researcher's findings, the empirical
course-corrections from two implementation attempts, and the lessons worth carrying
into similar projects. It is written to be honest about the wrong turns as well as
the right ones, because the wrong turns are where most of the transferable learning
lives.

## Where we started

The original goal was narrow and concrete: a CLI tool that runs on an Apple Silicon
Mac and produces a clean-room Debian ARM64 VM for Fuchsia work — specifically, for
enabling ARM64-Linux *host* build tooling, with success defined as building a
`minimal.arm64` image and running it under QEMU. A handful of decisions anchored
everything that followed, and each came from a hard constraint rather than taste:

- **Substrate.** A native arm64 Debian guest on Apple's Virtualization.framework
  (the `vz` backend) runs near-native and keeps QEMU — and its GPL — off the host.
- **Don't fight Apple's entitlement model.** Rather than embedding
  Virtualization.framework directly (which drags in the
  `com.apple.security.virtualization` codesigning and entitlement burden), the tool
  orchestrates the pre-signed `vfkit` binary. This was the single most important
  architectural concession, and it was explicitly sanctioned by the requirement that
  permits external tools where they avoid entitlements.
- **Permissive licensing as a first-class constraint.** This ruled out UTM
  (GPL/QEMU), OrbStack (proprietary), Multipass (GPL-3 + QEMU), and Tart (Fair Source
  with a CPU-core cap), and it later steered crate choices too.
- **Reproducible, clean-room install.** Direct-kernel-boot the official Debian
  netboot installer through `VZLinuxBootLoader` (no ISO repack, so no `xorriso`/GPLv3
  and no fragile UEFI re-authoring); pin a `snapshot.debian.org` timestamp; cache
  through an in-process record/replay mirror; verify the Debian signing chain in
  process.

The shape of the tool — a non-deterministic "resolve the latest pins" stage feeding
a deterministic, cacheable, resettable pipeline that ends in a blessed golden image —
was there from early on, and it survived every later revision intact. That is itself
a signal: the parts grounded in first principles (reproducibility, provenance,
licensing) were stable, while the parts grounded in assumptions about the hardware
were the ones that later needed correcting.

## What the research established

A good deal of the plan rests on facts that are easy to get wrong from memory, so
they were checked against primary sources rather than assumed:

- **Apple's nested-virtualization constraints.** Nested virtualization on
  Virtualization.framework is available only on M3-or-newer silicon, on macOS 15+,
  and only for Linux guests. It is off by default and must be requested per launch.
  This is *why* the host preflight matters and why macOS 15 became a hard floor.
- **A preflight that avoids the entitlement-bound API.** User-space `sysctl` values
  (`hw.machine`, `machdep.cpu.brand_string`, `kern.hv_support`, and
  `kern.osrelease` ≥ 24 for macOS 15) give a clean capability check without invoking
  the Virtualization.framework API just to probe support.
- **Licensing of every candidate tool and crate.** `vfkit` and `lima` are Apache-2.0
  and already signed; `rpgp` is MIT/Apache while `sequoia-openpgp` is LGPL; Tart's
  Fair Source license caps CPU cores. These checks did real work: they eliminated
  options that would otherwise have been tempting.
- **Debian's install internals.** The netboot kernel/initrd path, preseed
  directives, EFI-removable GRUB path, and `console=hvc0` for serial were all
  verified, as was the existence and behavior of `snapshot.debian.org` timestamped
  archives.

The research also surfaced the one area that would haunt the project: Apple Silicon
is 16 KB-page-native, KVM's Stage-2 page granule is gated by the underlying hardware,
and the interaction of those two facts under *nested* virtualization was genuinely
uncertain from the documentation alone.

## The cross-check: a second research pass

Partway through, a second research document (produced with Gemini) was brought in for
comparison. Fact-checking it was instructive in both directions.

Most of it was correct and independently corroborated the plan: the `sysctl`
preflight, the explicit `nestedVirtualization: true` for Lima, the `debian-packaging`
crate for parsing repository metadata, the choice of rPGP over LGPL Sequoia, the
netboot-plus-removable-EFI install approach, APFS `clonefile` for snapshots, the
case-sensitivity hazard of sharing a source tree over virtio-fs, and SSH forwarding
via gvisor-tap-vsock. Where two independent passes agree on a non-obvious detail,
confidence rightly goes up.

It also contained real errors that were caught and corrected: the `10.0.2.2` proxy
gateway it specified is a QEMU/slirp convention, not what vfkit's gvisor-tap-vsock
uses, and a `fx qemu` GIC-version flag it cited could not be verified. Those were
removed or flagged.

And then there was the claim that mattered most: that nested virtualization would
*require* a 16 KB-page guest kernel, and that a 4 KB kernel under `--nested` would
panic the hypervisor. At the time, the evidence I could find cut against the strong
version of that claim — 4 KB Linux guests demonstrably expose a working `/dev/kvm`
under Lima on Apple Silicon — so the plan recorded the 16 KB requirement as an
*empirical risk to validate* rather than a settled fact, and added a smoke test as
the gate. That hedge turned out to be the right *method* attached to the wrong *bet*.

## What the implementation taught us

Two rounds of implementation reports converted speculation into ground truth, and
they are the most valuable artifacts in the whole process.

**Round one settled the page-size question — against my skepticism.** On M4-class
hardware, a 4 KB kernel booted with `--nested` does panic the Apple hypervisor;
nested KVM requires a 16 KB *installed* kernel. The Gemini document had been right in
its conclusion. It also resolved a loose end: Debian Trixie ships a pre-compiled
`linux-image-arm64-16k`, so no custom kernel build is needed — which retired my
earlier doubt that such a package even existed. The same round delivered three
gotchas no amount of reading would have produced: `vfkit` crashes with "inappropriate
ioctl for device" unless its `virtio-serial`/stdio is given a real PTY; `--nested`
must be *off* during installation because high-I/O `partman` formatting panics with
nesting on; and the kernel decompression step needed care. The honest takeaway is
that the smoke-test gate did its job — it would have caught the page-size issue — but
I should have weighted a specific, falsifiable, load-bearing claim more heavily and
prioritized testing it first instead of arguing it down.

**Round two rewarded a cheap experiment and corrected one of my own instructions.**
The plan had carried a fairly elaborate installer: boot the 16 KB kernel, inject its
modules into the netboot initrd via cpio concatenation, and pass
`anna/no_kernel_modules=true` to suppress the resulting installer prompt. I had
flagged a hypothesis — since `--nested` is off during install anyway, the installer
could just run the *stock 4 KB* netboot kernel and only install the 16 KB kernel to
the target. The A/B test confirmed it, and three pieces of machinery evaporated:
no initrd surgery, no `anna` override, and — because Debian's arm64 netboot `linux`
is *already* an uncompressed `Image`, unlike x86's gzip `vmlinuz` — no decompression
step at all. That last point corrected an instruction I had written (and that the
Gemini document had also assumed). Round two also surfaced the final provisioning
detail: the dev user cannot open `/dev/kvm` by default, so a udev rule in the install
`late_command` is needed for unprivileged nested QEMU.

The pattern across both rounds is worth naming: the design was *more complex than the
constraints required*, and contact with reality simplified it.

## Generalizing: Android as a mirror of Fuchsia

Late in the process the question arose of whether the tool was Fuchsia-specific or a
general OS-development workbench. Checking the current AOSP documentation rather than
assuming proved the point precisely. Android's published build requirements — a
Debian-based Linux host, 64 GB RAM, ~400 GB disk, source via `repo`+git — line up
with the tool's host assumptions, and Android's virtual device, Cuttlefish, runs the
OS under crosvm using `/dev/kvm`, which is exactly the nested-virtualization the VM
already provides. Cuttlefish explicitly supports ARM64 hosts.

The honest caveat is what made Android a *genuine* parallel rather than a stretch:
building AOSP *on* an arm64 host is officially unsupported (Google states the build
requires x86-64; the prebuilt host toolchains are x86-64 and Soong's `linux_arm64` is
a cross-compile target, not a host), with arm64 self-hosting only community
work-in-progress. That is the same shape as the Fuchsia task, whose entire purpose is
to *enable* arm64 host build tooling that does not fully exist yet. So both use cases
exercise the same VM in the same way: arm64 target, arm64 emulator under nested KVM,
and arm64-host-build as the frontier. The tool's OS-agnostic core stayed put; only
four parameters vary per OS — source fetch, build prerequisites, emulator, and
verification target.

## How the requirements grew

The specification roughly tripled over the conversation, and the plan absorbed each
change without structural upheaval, which is the best evidence that the architecture
was sound. The notable additions: host resource assumptions (64 GB / 500 GB) that
turned a "feasibility risk" into a budgeting exercise; a full VM-lifecycle surface
(create/list/delete, start/stop/ssh, fork) required across *both* backends; a fork
defined precisely as suspend-clone-resume, which resolved an open quiesce question by
fiat; a feature×tool integration-test matrix; and finally a code-quality tier
(clippy/rustfmt, unit tests, testability seams) and an explicit permissive-crate
licensing rule. None of these forced a redesign — they slotted into the existing
sections.

## Doing better next time

- **Test the load-bearing, falsifiable claim first.** The page-size question was
  cheap to test and expensive to be wrong about. When a single fact gates the whole
  design and can be settled with a smoke test, settle it before reasoning around it.
  A correct *method* (build the gate) does not excuse a poorly-weighted *bet*.
- **Judge claims by evidence, not provenance.** The second researcher's document was
  mostly right, including its scariest claim, and also contained verifiable errors.
  Neither "it's from another model" nor "it sounds authoritative" is signal; only
  per-claim verification is.
- **Assume the design is more complex than the constraints demand.** The installer
  carried three mechanisms that a single isolated experiment removed. Before building
  the elaborate path, ask which complexity the constraints actually force versus
  which is inherited assumption — and isolate the variable to find out.
- **Separate what you verify from what you trust.** Scoping verification to Debian
  resources (signing chain, pinned snapshot) while explicitly trusting git/`repo`/CIPD
  for OS source kept the verification story tractable instead of boil-the-ocean.
- **Keep a living "resolved / open" ledger.** Tracking each decision and each
  unknown, and running a consistency scan after edits, is what let the document stay
  coherent across many revisions and renumberings rather than accumulating
  contradictions.
- **Ground present-state facts in primary sources, every time.** Tooling licenses,
  hardware nesting support, distro package availability, and another OS's build
  requirements all change; each was worth a fresh check rather than a recalled
  answer.

## Future work

- **Close the one open issue.** Confirm whether Lima exposes live suspend/resume for
  `fork`, or whether the lima-backend fork must fall back to stop-clone-start. Small,
  but it is the last unverified behavior.
- **Build out the Android profile for real.** Implement the AOSP use-case parameters
  end to end (the `repo` provisioning set, a Cuttlefish run path, an
  `aosp_cf_arm64_only_phone` verification) so the second use case is exercised, not
  just analyzed.
- **A declarative use-case profile format.** The four per-OS parameters (source
  fetch, prerequisites, emulator, verification target) are begging to become a small
  manifest, so adding a third OS is data, not code. This is the natural payoff of the
  generalization.
- **Persistent, content-addressed source caches.** Keep the in-guest Fuchsia/AOSP
  checkouts and tool caches in a shared volume that CoW forks inherit, to stop
  re-hitting Google on every iteration; a content-addressed store would make this
  reproducible too.
- **An x86-64 guest variant for the AOSP build host.** Since AOSP's host build is
  officially x86-64-only, a sibling pipeline that produces an x86-64 build VM (with
  the same packaging/verification machinery) would let the tool serve the *supported*
  Android build path, not only the arm64 frontier — and would test how OS-agnostic
  the core really is.
- **CI on Apple Silicon runners.** Run the integration suite (and periodically the
  full `minimal.arm64` acceptance build) on hosted M-series runners so regressions in
  the install/boot/nested path are caught automatically.
- **Tighter security posture for the golden image.** Replace the permissive
  `/dev/kvm` `MODE=0666` udev rule with `kvm`-group membership, sign and record an
  SBOM for blessed images, and treat the pin lockfile as an auditable supply-chain
  artifact.
- **Full VM state save/restore.** Beyond fork, Virtualization.framework can persist
  and restore VM state; exposing snapshot/restore would make long-running build VMs
  resumable across host reboots.
- **Upstream contributions.** The work this tool supports — arm64 host build tooling
  for Fuchsia, and arm64 self-hosting for AOSP — is valuable to the respective
  upstreams, and a reproducible clean-room VM is a strong substrate for landing and
  bisecting that work.
