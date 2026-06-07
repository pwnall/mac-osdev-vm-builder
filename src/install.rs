use crate::stage::Stage;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;

pub struct InstallStage {
    pub disk_path: std::path::PathBuf,
    pub kernel_path: std::path::PathBuf,
    pub initrd_path: std::path::PathBuf,
}

impl Stage for InstallStage {
    fn name(&self) -> &'static str {
        "install"
    }

    async fn execute(&self, state_dir: &Path, effects: &dyn crate::effects::Effects) -> Result<()> {
        let preseed_path = state_dir.join("preseed.cfg");
        let preseed_content = generate_preseed();
        effects
            .fs_write(&preseed_path, preseed_content.as_bytes())
            .await?;

        let initrd_abs = effects
            .fs_canonicalize(&self.initrd_path)
            .await
            .unwrap_or_else(|_| self.initrd_path.clone());
        let initrd_with_preseed = state_dir.join("initrd_with_preseed.gz");

        // Inject preseed.cfg into initrd using appended compressed cpio
        println!("Injecting preseed.cfg into initrd...");
        let script = format!(
            "cd {} && echo preseed.cfg | cpio -H newc -o | gzip -c > preseed.cpio.gz && cat {} preseed.cpio.gz > initrd_with_preseed.gz",
            state_dir.display(),
            initrd_abs.display()
        );
        let cpio_status = effects.run_command("sh", &["-c", &script]).await?;
        if !cpio_status.status.success() {
            return Err(anyhow!("Failed to inject preseed.cfg into initrd"));
        }

        let kernel_abs = effects
            .fs_canonicalize(&self.kernel_path)
            .await
            .unwrap_or_else(|_| self.kernel_path.clone());
        let disk_abs = effects
            .fs_canonicalize(&self.disk_path)
            .await
            .unwrap_or_else(|_| self.disk_path.clone());

        let cmdline = "auto=true priority=critical DEBIAN_FRONTEND=text console=tty0 console=ttyS0 console=ttyAMA0 console=hvc0 earlycon debug";

        println!("Running vfkit for installation phase...");
        let log_path_abs = effects
            .fs_canonicalize(&state_dir.join("install_serial.log"))
            .await
            .unwrap_or_else(|_| state_dir.join("install_serial.log"));

        let vfkit_args = format!(
            "vfkit --cpus 2 --memory 2048 --bootloader \"linux,kernel={},initrd={},cmdline={}\" --device \"virtio-blk,path={}\" --device virtio-serial,stdio --device virtio-net,nat,mac=00:11:22:33:44:55 --device virtio-rng",
            kernel_abs.display(),
            initrd_with_preseed.display(),
            cmdline,
            disk_abs.display()
        );

        let mut child = Command::new("script")
            .arg("-q")
            .arg(log_path_abs.as_os_str())
            .arg("sh")
            .arg("-c")
            .arg(&vfkit_args)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;

        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow::anyhow!("vfkit install phase failed."));
        }

        println!("Installation phase completed.");
        Ok(())
    }
}

pub fn generate_preseed() -> String {
    r#"
# Debian Installer Preseed
d-i debian-installer/locale string en_US
d-i keyboard-configuration/xkb-keymap select us

d-i netcfg/choose_interface select auto
d-i netcfg/get_hostname string fuchsia-vm
d-i netcfg/get_domain string local

d-i mirror/country string manual
d-i mirror/http/hostname string deb.debian.org
d-i mirror/http/directory string /debian
d-i mirror/http/proxy string

d-i passwd/root-login boolean false
d-i passwd/user-fullname string Fuchsia Developer
d-i passwd/username string fuchsia
d-i passwd/user-password string fuchsia
d-i passwd/user-password-again string fuchsia

d-i clock-setup/utc boolean true
d-i time/zone string UTC

d-i partman-auto/method string regular
d-i partman-auto/expert_recipe string \
      custom-efi-root :: \
              512 512 512 fat32 \
                      $iflabel{ gpt } \
                      $reusemethod{ } \
                      method{ efi } \
                      format{ } \
              . \
              500 10000 -1 ext4 \
                      $primary{ } \
                      method{ format } \
                      format{ } \
                      use_filesystem{ } \
                      filesystem{ ext4 } \
                      mountpoint{ / } \
              . \
              1024 1024 1024 linux-swap \
                      method{ swap } \
                      format{ } \
              .
d-i partman-auto/choose_recipe select custom-efi-root
d-i partman-partitioning/choose_label string gpt
d-i partman-partitioning/default_label string gpt
d-i partman-efi/non_efi_system boolean true
d-i partman/choose_partition select finish
d-i partman/confirm boolean true
d-i partman/confirm_nooverwrite boolean true

d-i pkgsel/include string openssh-server sudo curl git build-essential python3 grub-efi-arm64 cloud-init sshfs uidmap qemu-system-arm qemu-system-aarch64 unzip file ca-certificates
d-i pkgsel/upgrade select full-upgrade

d-i grub-installer/only_debian boolean true
d-i grub-installer/force-efi-extra-removable boolean true

d-i base-installer/kernel/image string linux-image-arm64-16k
d-i debian-installer/add-kernel-opts string console=hvc0
d-i finish-install/reboot_in_progress note

# Power off on exit instead of reboot
d-i debian-installer/exit/poweroff boolean true

# Sentinel file to signal completion (for test observability)
d-i preseed/late_command string \
    mkdir -p /target/boot/efi ; \
    in-target grub-install --target=arm64-efi --removable --no-nvram /dev/vda ; \
    in-target update-grub ; \
    mkdir -p /target/etc/cloud/cloud.cfg.d ; \
    echo "datasource_list: [ NoCloud, None ]" > /target/etc/cloud/cloud.cfg.d/99-lima.cfg ; \
    echo 'KERNEL=="kvm", GROUP="kvm", MODE="0666"' > /target/etc/udev/rules.d/99-kvm.rules ; \
    echo "INSTALL_DONE" > /target/root/install_done.txt

"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_preseed() {
        let preseed = generate_preseed();
        assert!(preseed.contains("d-i netcfg/get_hostname string fuchsia-vm"));
        assert!(preseed.contains("d-i passwd/username string fuchsia"));
        assert!(preseed.contains("custom-efi-root"));
        assert!(preseed.contains("linux-image-arm64-16k"));
        assert!(preseed.contains("console=hvc0"));
    }
}
