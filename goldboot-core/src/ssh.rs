use log::{debug, info};
use std::{
	error::Error,
	io::{Cursor, Read},
	net::TcpStream,
	path::Path,
};

/// Represents an SSH session to a running VM.
pub struct SshConnection {
	pub username: String,
	pub password: String,
	pub session: ssh2::Session,
}

impl SshConnection {
	pub fn new(port: u16, username: &str, password: &str) -> Result<SshConnection, Box<dyn Error>> {
		debug!("Trying SSH: {}@localhost:{}", username, port);

		let mut session = ssh2::Session::new()?;
		session.set_tcp_stream(TcpStream::connect(format!("127.0.0.1:{port}"))?);

		session.handshake()?;
		session.userauth_password(username, password)?;

		info!("Established SSH connection");
		Ok(SshConnection {
			username: username.to_string(),
			password: password.to_string(),
			session,
		})
	}

	/// Send the shutdown command to the VM.
	pub fn shutdown(&self, command: &str) -> Result<(), Box<dyn Error>> {
		info!("Sending shutdown command");
		let mut channel = self.session.channel_session()?;
		channel.exec(command)?;
		Ok(())
	}

	pub fn upload_exec(&self, source: Vec<u8>, env: Vec<(&str, &str)>) -> Result<(), Box<dyn Error>> {
		self.upload(source, "/tmp/tmp.script")?;
		self.exec_env("/tmp/tmp.script", env)?;
		self.exec("rm -f /tmp/tmp.script")?;
		Ok(())
	}

	pub fn upload(&self, source: Vec<u8>, dest: &str) -> Result<(), Box<dyn Error>> {
		let mut channel = self.session.scp_send(Path::new(dest), 0o700, source.len().try_into()?, None)?;
		std::io::copy(&mut Cursor::new(source), &mut channel)?;

		channel.send_eof()?;
		channel.wait_eof()?;
		channel.close()?;
		channel.wait_close()?;

		Ok(())
	}

	/// Run a command on the VM with the given environment.
	pub fn exec_env(&self, cmdline: &str, env: Vec<(&str, &str)>) -> Result<i32, Box<dyn Error>> {
		debug!("Executing command: '{}'", cmdline);

		let mut channel = self.session.channel_session()?;

		// Set environment
		for (var, val) in env {
			channel.setenv(&var, &val)?;
		}

		channel.exec(cmdline)?;

		let mut output = String::new();
		channel.read_to_string(&mut output)?;
		// TODO print

		channel.wait_close()?;
		let exit = channel.exit_status()?;
		debug!("Exit code: {}", exit);
		Ok(exit)
	}

	/// Run a command on the VM.
	pub fn exec(&self, cmdline: &str) -> Result<i32, Box<dyn Error>> {
		debug!("Executing command: '{}'", cmdline);

		let mut channel = self.session.channel_session()?;
		channel.exec(cmdline)?;

		let mut output = String::new();
		channel.read_to_string(&mut output)?;
		// TODO print

		channel.wait_close()?;
		let exit = channel.exit_status()?;
		debug!("Exit code: {}", exit);
		Ok(exit)
	}
}
