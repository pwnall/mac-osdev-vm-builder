use clap::{Parser, Subcommand};
use mac_osdev_vm_builder::Runner;
use mac_osdev_vm_builder::backend::{Backend, lima::LimaBackend, vfkit::VfkitBackend};
use mac_osdev_vm_builder::clone::BlessStage;
use mac_osdev_vm_builder::create_disk::CreateDiskStage;
use mac_osdev_vm_builder::fetch_installer::FetchInstallerStage;
use mac_osdev_vm_builder::install::InstallStage;
use mac_osdev_vm_builder::materialize_mirror::MaterializeMirrorStage;
use mac_osdev_vm_builder::provision_dev::ProvisionDevStage;
use mac_osdev_vm_builder::record_manifest::RecordManifestStage;
use mac_osdev_vm_builder::resolve::ResolveStage;
use mac_osdev_vm_builder::verify::VerifyStage;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "ARM64 OS-Development VM Builder for macOS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// State directory for the VM builder
    #[arg(short, long, default_value = "./osdev-vm-state", global = true)]
    state_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Package a new golden VM image
    #[command(subcommand)]
    Package(PackageCommands),

    /// Manage VMs using a golden image
    #[command(subcommand)]
    Vm(VmCommands),
}

#[derive(Subcommand)]
enum PackageCommands {
    /// Run the build pipeline
    Build {
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
}

#[derive(Subcommand)]
enum VmCommands {
    /// Create a new VM from the golden image
    Create {
        name: String,
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
    /// Delete a VM
    Delete {
        name: String,
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
    /// Start a VM
    Start {
        name: String,
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
    /// Stop a VM
    Stop {
        name: String,
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
    /// SSH into a VM
    Ssh {
        name: String,
        #[arg(long, default_value = "vfkit")]
        backend: String,
        /// Command to execute over SSH (optional)
        command: Vec<String>,
    },
    /// List existing VMs
    List {
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
    /// Fork an existing VM into a new independent VM
    Fork {
        source_name: String,
        target_name: String,
        #[arg(long, default_value = "vfkit")]
        backend: String,
    },
}

fn get_backend(backend_type: &str, state_dir: &std::path::Path) -> Box<dyn Backend> {
    if backend_type == "lima" {
        Box::new(LimaBackend::new(state_dir))
    } else {
        Box::new(VfkitBackend::new(state_dir))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let effects = mac_osdev_vm_builder::effects::RealEffects;
    let runner = Runner::new(&cli.state_dir, &effects);

    match cli.command {
        Commands::Package(PackageCommands::Build { backend }) => {
            runner.run_stage(ResolveStage).await?;
            let fetch_stage = FetchInstallerStage {
                kernel_path: cli.state_dir.join("installer/vmlinux"),
                initrd_path: cli.state_dir.join("installer/initrd.gz"),
            };
            runner.run_stage(fetch_stage).await?;
            runner.run_stage(RecordManifestStage).await?;
            runner.run_stage(MaterializeMirrorStage).await?;
            runner.run_stage(CreateDiskStage::default()).await?;

            let install_stage = InstallStage {
                disk_path: cli.state_dir.join("disk.raw"),
                kernel_path: cli.state_dir.join("installer/vmlinux"),
                initrd_path: cli.state_dir.join("installer/initrd.gz"),
            };
            runner.run_stage(install_stage).await?;

            let provision_stage = ProvisionDevStage {
                backend_type: backend.clone(),
                disk_path: cli.state_dir.join("disk.raw"),
            };
            runner.run_stage(provision_stage).await?;

            let verify_stage = VerifyStage {
                backend_type: backend,
                disk_path: cli.state_dir.join("disk.raw"),
            };
            runner.run_stage(verify_stage).await?;

            let bless_stage = BlessStage {
                disk_path: cli.state_dir.join("disk.raw"),
            };
            runner.run_stage(bless_stage).await?;

            println!("Full build pipeline completed successfully.");
        }
        Commands::Vm(vm_cmd) => match vm_cmd {
            VmCommands::Create { name, backend } => {
                let be = get_backend(&backend, &cli.state_dir);
                be.create(&name, &cli.state_dir.join("golden.raw"))?;
            }
            VmCommands::Delete { name, backend } => {
                let be = get_backend(&backend, &cli.state_dir);
                be.delete(&name)?;
            }
            VmCommands::Start { name, backend } => {
                let be = get_backend(&backend, &cli.state_dir);
                be.start(&name)?;
            }
            VmCommands::Stop { name, backend } => {
                let be = get_backend(&backend, &cli.state_dir);
                be.stop(&name)?;
            }
            VmCommands::Ssh {
                name,
                backend,
                command,
            } => {
                let be = get_backend(&backend, &cli.state_dir);
                let cmd_refs: Vec<&str> = command.iter().map(AsRef::as_ref).collect();
                be.ssh(
                    &name,
                    if cmd_refs.is_empty() {
                        None
                    } else {
                        Some(&cmd_refs)
                    },
                )?;
            }
            VmCommands::List { backend } => {
                let be = get_backend(&backend, &cli.state_dir);
                let vms = be.list()?;
                for vm in vms {
                    println!("{}", vm);
                }
            }
            VmCommands::Fork {
                source_name,
                target_name,
                backend,
            } => {
                let be = get_backend(&backend, &cli.state_dir);
                be.fork(&source_name, &target_name)?;
            }
        },
    }

    Ok(())
}
