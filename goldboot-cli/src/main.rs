#![feature(derive_default_enum)]

use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::{env, error::Error, path::PathBuf};

pub mod build;
pub mod cache;
pub mod image;
pub mod init;
pub mod make_usb;
pub mod registry;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CommandLine {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build a new image
    Build {
        /// Save a screenshot to ./debug after each boot command
        #[clap(long, takes_value = false)]
        record: bool,

        /// Insert a breakpoint after each boot command
        #[clap(long, takes_value = false)]
        debug: bool,
    },

    /// Manage local images
    Image {
        #[clap(subcommand)]
        command: ImageCommands,
    },

    /// Initialize the current directory
    Init {
        /// The image name
        #[clap(long)]
        name: Option<String>,

        /// A base profile which can be found with --list-profiles
        #[clap(long)]
        profile: Vec<String>,

        /// The amount of memory the image can access
        #[clap(long)]
        memory: Option<String>,

        /// The amount of storage the image can access
        #[clap(long)]
        disk: Option<String>,

        /// List available profiles and exit
        #[clap(long, takes_value = false)]
        list_profiles: bool,
    },

    /// Create a bootable USB drive
    MakeUsb {
        /// The disk to erase and make bootable
        disk: String,

        /// Do not check for confirmation
        #[clap(long, takes_value = false)]
        confirm: bool,

        /// A local image to include on the boot USB
        #[clap(long)]
        include: Vec<String>,
    },

    /// Manage image registries
    Registry {
        #[clap(subcommand)]
        command: RegistryCommands,
    },
}

#[derive(Subcommand, Debug)]
enum RegistryCommands {
    /// Upload a local image to a remote registry
    Push { url: String },

    /// Download an image from a remote registry
    Pull { url: String },
}

#[derive(Subcommand, Debug)]
enum ImageCommands {
    /// List local images
    List {},

    Info {
        image: String,
    },

    /// Write image to a disk
    Write {
        /// The selected image
        #[clap(long)]
        image: String,

        /// The disk to overwrite
        #[clap(long)]
        disk: String,

        /// Do not check for confirmation
        #[clap(long, takes_value = false)]
        confirm: bool,
    },

    /// Run an existing image
    Run {
        image: String,
    },
}

/// Return the image library path for the current platform.
pub fn image_library_path() -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from("/var/lib/goldboot/images")
    } else {
        panic!("Unsupported platform");
    }
}

/// A simple cache for storing images that are not stored in the Packer cache.
/// Most images here need some kind of transformation before they are bootable.
pub fn image_cache_lookup(key: &str) -> PathBuf {
    // Hash the key to get the filename
    let hash = hex::encode(Sha256::new().chain_update(&key).finalize());

    if cfg!(target_os = "linux") {
        PathBuf::from("/var/lib/goldboot/cache").join(hash)
    } else {
        panic!("Unsupported platform");
    }
}

/// Get the QEMU system binary for the current platform
pub fn current_qemu_binary() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "qemu-system-x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "qemu-system-aarch64"
    } else {
        panic!("Unsupported platform");
    }
}

/// Determine whether builds should be headless or not for debugging.
pub fn build_headless_debug() -> bool {
    if env::var("CI").is_ok() {
        return true;
    }
    if env::var("GOLDBOOT_DEBUG").is_ok() {
        return false;
    }
    return true;
}

pub fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line first
    let cl = CommandLine::parse();

    // Configure logging
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Dispatch command
    match &cl.command {
        Commands::Build { record, debug } => crate::build::build(*record, *debug),
        Commands::Registry { command } => match &command {
            RegistryCommands::Push { url } => crate::registry::push(),
            RegistryCommands::Pull { url } => crate::registry::pull(),
        },
        Commands::Init {
            name,
            profile,
            memory,
            disk,
            list_profiles,
        } => {
            if *list_profiles {
                profile::list_profiles()
            } else {
                crate::init::init(profile, name, memory, disk)
            }
        }
        Commands::MakeUsb {
            disk,
            confirm,
            include,
        } => crate::make_usb::make_usb(),
        Commands::Image { command } => match &command {
            ImageCommands::List {} => crate::image::list(),
            ImageCommands::Info { image } => crate::image::info(image),
            ImageCommands::Run { image } => crate::image::run(image),
            ImageCommands::Write {
                image,
                disk,
                confirm,
            } => crate::image::write(image, disk),
        },
    }
}