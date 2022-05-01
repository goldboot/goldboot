use crate::BuildConfig;
use goldboot_image::{levels::ClusterDescriptor, GoldbootImage};
use log::debug;
use sha1::Digest;
use sha2::Sha256;
use std::{
	error::Error,
	fs::File,
	io::{BufReader, Seek, SeekFrom, Write},
	path::{Path, PathBuf},
	process::Command,
};

/// Represents the local image library.
///
/// Depending on the platform, the directory will be located at:
///     - /var/lib/goldboot/images (linux)
///
/// Images are named according to their SHA256 hash (ID) and have a file extension
/// of ".gb".
pub struct ImageLibrary;

/// Return the image library path for the current platform.
fn library_path() -> PathBuf {
	let path = if cfg!(target_os = "linux") {
		PathBuf::from("/var/lib/goldboot/images")
	} else {
		panic!("Unsupported platform");
	};

	std::fs::create_dir_all(&path).unwrap();
	path
}

/// Represents a local goldboot image.
pub struct ImageMetadata {
	/// The image's ID (SHA256 hash)
	pub id: String,

	/// The size in bytes of the image file itself
	pub size: u64,

	/// Whether the image can be downloaded by anonymous users
	pub public: bool,

	/// The config that was used during the build
	pub config: BuildConfig,

	/// The library path to the image
	pub path: String,
}

impl ImageLibrary {
	/// Add an image to the library. The image will be hashed and copied to the
	/// library with the appropriate name.
	pub fn add(image_path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
		let mut hasher = Sha256::new();
		std::io::copy(&mut File::open(&image_path)?, &mut hasher)?;
		let hash = hex::encode(hasher.finalize());

		std::fs::copy(&image_path, library_path().join(format!("{hash}.gb")))?;
		Ok(())
	}

	/// Load images present in the local image library.
	pub fn load() -> Result<Vec<ImageMetadata>, Box<dyn Error>> {
		let mut images = Vec::new();

		for p in library_path().read_dir()? {
			let path = p?.path();

			if let Some(ext) = path.extension() {
				if ext == "gb" {
					let image = GoldbootImage::open(&path)?;

					images.push(ImageMetadata {
						id: path.file_stem().unwrap().to_str().unwrap().to_string(),
						size: std::fs::metadata(&path)?.len(),
						path: path.to_string_lossy().to_string(),
						config: BuildConfig::from_image(&image)?,
						public: false,
					});
				}
			}
		}

		Ok(images)
	}

	/// Find images in the library by name.
	pub fn find_by_name(image_name: &str) -> Result<Vec<ImageMetadata>, Box<dyn Error>> {
		Ok(ImageLibrary::load()?
			.into_iter()
			.filter(|metadata| metadata.config.name == image_name)
			.collect())
	}

	/// Find images in the library by ID.
	pub fn find_by_id(image_id: &str) -> Result<ImageMetadata, Box<dyn Error>> {
		Ok(ImageLibrary::load()?
			.into_iter()
			.find(|metadata| metadata.id == image_id)
			.ok_or("Image not found")?)
	}

	/// Remove an image from the library by ID.
	pub fn delete(image_id: &str) -> Result<(), Box<dyn Error>> {
		todo!();
	}
}

pub fn write(image: &ImageMetadata, disk_name: &str) -> Result<(), Box<dyn Error>> {
	// TODO backup option

	// Verify sizes are compatible
	//if image.size != disk.total_space() {
	//    bail!("The requested disk size is not equal to the image size");
	//}

	// Check if mounted
	// TODO

	// Update EFI vars
	// TODO

	let mut f = File::open("foo.txt").unwrap();

	let qcow2 = goldboot_image::GoldbootImage::open(&image.path)?;
	let mut file = BufReader::new(File::open(&image.path)?);

	let mut offset = 0u64;
	let mut buffer = [0u8, 1 << qcow2.header.cluster_bits];

	for l1_entry in qcow2.l1_table {
		if l1_entry.l2_offset() != 0 {
			if let Some(l2_table) = l1_entry.read_l2(&mut file, qcow2.header.cluster_bits) {
				for l2_entry in l2_table {
					match &l2_entry.cluster_descriptor {
						ClusterDescriptor::Standard(cluster) => {
							if cluster.host_cluster_offset != 0 {
								debug!("Uncompressed cluster: {:?}", cluster);
								l2_entry.read_contents(&mut file, &mut buffer).unwrap();
								f.seek(SeekFrom::Start(offset)).unwrap();
								f.write_all(&buffer).unwrap();
							}
						}
						ClusterDescriptor::Compressed(cluster) => {
							debug!("Compressed cluster: {:?}", cluster);
						}
					}
					offset += 1 << qcow2.header.cluster_bits;
				}
			}
		} else {
			offset += u64::pow(1 << qcow2.header.cluster_bits, 2) / 8;
		}
	}
	Ok(())
}

pub fn run(image: &ImageMetadata) -> Result<(), Box<dyn Error>> {
	Command::new("qemu-system-x86_64")
		.args([
			"-display",
			"gtk",
			"-machine",
			"type=pc,accel=kvm",
			"-m",
			"1000M",
			"-boot",
			"once=c",
			"-bios",
			"/usr/share/ovmf/x64/OVMF.fd",
			"-pflash",
			"/tmp/test.fd",
			"-drive",
			&format!(
				"file={},if=virtio,cache=writeback,discard=ignore,format=qcow2",
				&image.path
			),
			"-name",
			"cli",
		])
		.status()
		.unwrap();
	Ok(())
}
