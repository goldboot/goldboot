use crate::{config::Config, profiles};
use goldboot_templates::*;
use simple_error::bail;
use std::{env, error::Error, fs, path::Path};

/// Choose some arbitrary disk and get its size in bytes. The user will likely change it
/// in the config later.
fn guess_disk_size() -> u64 {
    if cfg!(target_os = "unix") {
        // TODO
    }
    return 64000000000;
}

/// Choose some arbitrary memory size in megabytes which is less than the amount of available free memory on the system.
fn guess_memory_size() -> u64 {
    return 2048;
}

pub fn init(
    templates: &Vec<String>,
    name: &Option<String>,
    memory: &Option<String>,
    disk: &Option<String>,
) -> Result<(), Box<dyn Error>> {
    let config_path = Path::new("goldboot.json");

    if config_path.exists() {
        bail!("This directory has already been initialized. Delete goldboot.json to reinitialize.");
    }

    if templates.len() == 0 {
        bail!("Specify at least one template with --template");
    }

    // Create a new config to be filled in according to the given arguments
    let mut config = Config::default();

    if let Some(name) = name {
        config.name = name.to_string();
    } else {
        // Set name equal to directory name
        if let Some(name) = env::current_dir()?.file_name() {
            config.name = name.to_str().unwrap().to_string();
        }
    }

    // Generate QEMU flags for this hardware
    //config.qemuargs = generate_qemuargs()?;

    // Set current platform
    config.arch = if cfg!(target_arch = "x86_64") {
        Some("x86_64".into())
    } else if cfg!(target_arch = "aarch64") {
        Some("aarch64".into())
    } else {
        bail!("Unsupported platform");
    };

    // Set an arbitrary disk size unless given a value
    config.disk_size = if let Some(disk_size) = disk {
        disk_size.to_string()
    } else {
        format!("{}b", guess_disk_size())
    };

    // Set an arbitrary memory size unless given a value
    config.memory = if let Some(memory_size) = memory {
        memory_size.to_string()
    } else {
        format!("{}", guess_memory_size())
    };

    // Run template-specific initialization
    templates = templates
        .iter()
        .map(|t| #[rustfmt::skip]
            match t.to_lowercase() {
                "alpine"         => serde_json::to_value(AlpineTemplate::default()),
                "archlinux"      => serde_json::to_value(ArchLinuxTemplate::default()),
                "debian"         => serde_json::to_value(DebianTemplate::default()),
                "goldbootusb"    => serde_json::to_value(GoldbootUsbTemplate::default()),
                "macos"          => serde_json::to_value(MacOsTemplate::default()),
                "popos"          => serde_json::to_value(PopOsTemplate::default()),
                "steamdeck"      => serde_json::to_value(SteamDeckTemplate::default()),
                "steamos"        => serde_json::to_value(SteamOsTemplate::default()),
                "ubuntudesktop"  => serde_json::to_value(UbuntuDesktopTemplate::default()),
                "ubuntuserver"   => serde_json::to_value(UbuntuServerTemplate::default()),
                "windows10"      => serde_json::to_value(Windows10Template::default()),
                "windows11"      => serde_json::to_value(Windows11Template::default()),
                "windows7"       => serde_json::to_value(Windows7Template::default()),
                _                => bail!("Unknown template"),
            })
        .collect();

    if templates.len() == 1 {
        config.template = Some(templates.first()?);
    } else {
        config.templates = Some(templates);
    }

    // Finally write out the config
    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}
