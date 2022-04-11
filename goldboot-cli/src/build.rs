use crate::commands::image::ImageMetadata;
use crate::config::Config;
use colored::*;
use log::{debug, info};
use simple_error::bail;
use std::time::Instant;
use std::{error::Error, fs};

#[rustfmt::skip]
fn print_banner() {
    println!("⬜{}⬜", "⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜");
    println!("⬜{}⬜", "　　　　　　　　⬛　　　⬛　⬛　　　　　　　　　　　⬛　".truecolor(200, 171, 55));
    println!("⬜{}⬜", "　　　　　　　　⬛　　　⬛　⬛　　　　　　　　　　　⬛⬛".truecolor(200, 171, 55));
    println!("⬜{}⬜", "⬛⬛⬛　⬛⬛⬛　⬛　⬛⬛⬛　⬛⬛⬛　⬛⬛⬛　⬛⬛⬛　⬛　".truecolor(200, 171, 55));
    println!("⬜{}⬜", "⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　⬛　".truecolor(200, 171, 55));
    println!("⬜{}⬜", "⬛⬛⬛　⬛⬛⬛　⬛　⬛⬛⬛　⬛⬛⬛　⬛⬛⬛　⬛⬛⬛　⬛⬛".truecolor(200, 171, 55));
    println!("⬜{}⬜", "　　⬛　　　　　　　　　　　　　　　　　　　　　　　　　".truecolor(200, 171, 55));
    println!("⬜{}⬜", "⬛⬛⬛　　　　　　　　　　　　　　　　　　　　　　　　　".truecolor(200, 171, 55));
    println!("⬜{}⬜", "⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜");
}

pub fn build(record: bool, debug: bool) -> Result<(), Box<dyn Error>> {
    print_banner();

    let start_time = Instant::now();
    let context = BuildContext::new(Config::load()?, record, debug);

    // Prepare to build profiles
    let profiles = config.get_profiles();
    let profiles_len = profiles.len();
    if profiles_len == 0 {
        bail!("At least one base profile must be specified");
    }

    // Create an initial image that will be attached as storage to each VM
    let image_path = tmp.path().join("image.gb");
    debug!("Allocating new {} image: {}", config.disk_size, image_path);
    goldboot_image::Qcow2::create(
        &image_path,
        config.disk_size_bytes(),
        serde_json::to_vec(&config)?,
    )?;

    // Create partitions if we're multi booting
    if profiles.len() > 1 {
        // TODO
    }

    // Build each profile
    for profile in profiles {
        profile.build(&config, &image_path)?;
    }

    // Install bootloader if we're multi booting
    if profiles_len > 1 {
        // TODO
    }

    // Attempt to reduce the size of image
    crate::qemu::compact_qcow2(&image_path)?;

    info!("Build completed in: {:?}", start_time.elapsed());

    // Create new image metadata
    let metadata = ImageMetadata::new(config.clone())?;
    metadata.write()?;

    // Move the image to the library
    fs::rename(image_path, metadata.path_qcow2())?;

    Ok(())
}
