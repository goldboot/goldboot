use crate::{
	build::BuildWorker,
	cache::{MediaCache, MediaFormat},
	qemu::QemuArgs,
	templates::*,
};
use log::info;
use serde::{Deserialize, Serialize};
use simple_error::bail;
use std::{
	error::Error,
	io::{BufRead, BufReader},
};
use validator::Validate;

const DEFAULT_MIRROR: &str = "https://mirrors.edge.kernel.org/archlinux";

const MIRRORLIST: &'static [&'static str] = &[
	"https://geo.mirror.pkgbuild.com/",
	"https://mirror.rackspace.com/archlinux/",
	"https://mirrors.edge.kernel.org/archlinux/",
];

#[derive(rust_embed::RustEmbed)]
#[folder = "res/Arch/"]
struct Resources;

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct ArchTemplate {
	pub id: TemplateId,

	#[validate(length(max = 64))]
	pub root_password: String,

	pub mirrorlist: Vec<String>,

	/// The installation media
	pub iso: IsoContainer,

	#[serde(flatten)]
	pub general: GeneralContainer,

	//pub luks: LuksContainer,
	#[serde(flatten)]
	pub provisioners: ProvisionersContainer,
}

impl ArchTemplate {
	pub fn format_mirrorlist(&self) -> String {
		self.mirrorlist
			.iter()
			.map(|s| format!("Server = {}", s))
			.collect::<Vec<String>>()
			.join("\n")
	}
}

/// Fetch the latest iso URL and its SHA1 hash
fn fetch_latest_iso() -> Result<(String, String), Box<dyn Error>> {
	let rs = reqwest::blocking::get(format!("{DEFAULT_MIRROR}/iso/latest/sha1sums.txt"))?;
	if rs.status().is_success() {
		for line in BufReader::new(rs).lines().filter_map(|result| result.ok()) {
			if line.ends_with(".iso") {
				let split: Vec<&str> = line.split_whitespace().collect();
				if let [hash, filename] = split[..] {
					return Ok((
						format!("{DEFAULT_MIRROR}/iso/latest/{filename}"),
						format!("sha1:{hash}"),
					));
				}
			}
		}
	}
	bail!("Failed to request latest ISO");
}

impl Default for ArchTemplate {
	fn default() -> Self {
		let (iso_url, iso_checksum) = fetch_latest_iso().unwrap_or((
			format!("{DEFAULT_MIRROR}/iso/latest/archlinux-2022.03.01-x86_64.iso"),
			String::from("none"),
		));
		Self {
			root_password: String::from("root"),
			mirrorlist: vec![format!("{DEFAULT_MIRROR}/$repo/os/$arch",)],
			iso: IsoContainer {
				url: iso_url,
				checksum: iso_checksum,
			},
			general: GeneralContainer {
				base: TemplateBase::ArchLinux,
				storage_size: String::from("10 GiB"),
				..Default::default()
			},
			provisioners: ProvisionersContainer::default(),
		}
	}
}

impl Template for ArchTemplate {
	fn build(&self, context: &BuildWorker) -> Result<(), Box<dyn Error>> {
		info!("Starting {} build", console::style("ArchLinux").blue());

		let mut qemuargs = QemuArgs::new(&context);

		qemuargs.drive.push(format!(
			"file={},if=virtio,cache=writeback,discard=ignore,format=qcow2",
			context.image_path
		));
		qemuargs.drive.push(format!(
			"file={},media=cdrom",
			MediaCache::get(self.iso.url.clone(), &self.iso.checksum, MediaFormat::Iso)?
		));

		// Start VM
		let mut qemu = qemuargs.start_process()?;

		// Send boot command
		#[rustfmt::skip]
		qemu.vnc.boot_command(vec![
			// Initial wait
			wait!(30),
			// Wait for login
			wait_screen_rect!("5b3ca88689e9d671903b3040889c7fa1cb5f244a", 100, 0, 1024, 400),
			// Configure root password
			enter!("passwd"), enter!(self.root_password), enter!(self.root_password),
			// Configure SSH
			enter!("echo 'AcceptEnv *' >>/etc/ssh/sshd_config"),
			enter!("echo 'PermitRootLogin yes' >>/etc/ssh/sshd_config"),
			// Start sshd
			enter!("systemctl restart sshd"),
		])?;

		// Wait for SSH
		let mut ssh = qemu.ssh_wait(context.ssh_port, "root", &self.root_password)?;

		// Run install script
		if let Some(resource) = Resources::get("install.sh") {
			info!("Running base installation");
			match ssh.upload_exec(
				resource.data.to_vec(),
				vec![
					("GB_MIRRORLIST", &self.format_mirrorlist()),
					("GB_ROOT_PASSWORD", &self.root_password),
				],
			) {
				Ok(0) => debug!("Installation completed successfully"),
				_ => bail!("Installation failed"),
			}
		}

		// Run provisioners
		self.provisioners.run(&mut ssh)?;

		// Shutdown
		ssh.shutdown("poweroff")?;
		qemu.shutdown_wait()?;
		Ok(())
	}
}

impl Promptable for ArchTemplate {
	fn prompt(
		&mut self,
		config: &BuildConfig,
		theme: &dialoguer::theme::ColorfulTheme,
	) -> Result<(), Box<dyn Error>> {
		// Prompt mirror list
		{
			let template_index = dialoguer::Select::with_theme(theme)
				.with_prompt("Choose a mirror site")
				.default(0)
				.items(&MIRRORLIST)
				.interact()?;

			self.mirrorlist = vec![MIRRORLIST[template_index].to_string()];
		}

		// Prompt provisioners
		self.provisioners.prompt(config, theme)?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_fetch_latest_iso() -> Result<(), Box<dyn Error>> {
		fetch_latest_iso()?;
		Ok(())
	}
}
