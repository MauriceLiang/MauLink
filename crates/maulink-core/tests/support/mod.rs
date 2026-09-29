#![cfg(unix)]
#![allow(dead_code)]

use std::{
    fs,
    net::TcpListener,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub struct OpenSshFixture {
    directory: tempfile::TempDir,
    process: Option<Child>,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub private_key: PathBuf,
    host_key_a: PathBuf,
    host_key_b: PathBuf,
}

impl OpenSshFixture {
    pub fn start() -> Self {
        let directory = tempfile::tempdir().expect("OpenSSH fixture directory");
        let private_key = directory.path().join("client_ed25519");
        let encrypted_private_key = directory.path().join("client_encrypted_ed25519");
        let host_key_a = directory.path().join("host_a_ed25519");
        let host_key_b = directory.path().join("host_b_ed25519");
        generate_key(&private_key, "");
        generate_key(&encrypted_private_key, "fixture-passphrase");
        generate_key(&host_key_a, "");
        generate_key(&host_key_b, "");

        let mut authorized_keys =
            fs::read_to_string(private_key.with_extension("pub")).expect("read fixture public key");
        authorized_keys.push_str(
            &fs::read_to_string(encrypted_private_key.with_extension("pub"))
                .expect("read encrypted fixture public key"),
        );
        fs::write(directory.path().join("authorized_keys"), authorized_keys)
            .expect("write fixture authorized_keys");

        let username = std::env::var("USER").expect("USER must identify the fixture account");
        assert!(
            !username.is_empty()
                && username
                    .chars()
                    .all(|character| !character.is_whitespace() && !character.is_control()),
            "fixture username must be safe for sshd_config"
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("reserve fixture port");
        let port = listener.local_addr().expect("fixture address").port();
        drop(listener);

        let mut fixture = Self {
            directory,
            process: None,
            host: "127.0.0.1".to_owned(),
            port,
            username,
            private_key,
            host_key_a,
            host_key_b,
        };
        let initial_host_key = fixture.host_key_a.clone();
        fixture.launch(&initial_host_key);
        fixture
    }

    pub fn rotate_host_key(&mut self) {
        self.stop();
        let rotated_host_key = self.host_key_b.clone();
        self.launch(&rotated_host_key);
    }

    pub fn restart_with_rotated_host_key(&mut self) {
        self.stop();
        let rotated_host_key = self.host_key_b.clone();
        self.launch(&rotated_host_key);
    }

    pub fn freeze(&self) {
        signal_process_group(self.process_id(), "-STOP");
    }

    pub fn stop(&mut self) {
        let Some(mut process) = self.process.take() else {
            return;
        };
        signal_process_group(process.id(), "-TERM");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match process.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                Ok(None) => {
                    signal_process_group(process.id(), "-KILL");
                    let _ = process.wait();
                    return;
                }
                Err(_) => return,
            }
        }
    }

    pub fn kill_frozen(&mut self) {
        let Some(mut process) = self.process.take() else {
            return;
        };
        signal_process_group(process.id(), "-KILL");
        let _ = process.wait();
    }

    fn launch(&mut self, host_key: &Path) {
        assert!(self.process.is_none(), "fixture already running");
        let config_path = self.directory.path().join("sshd_config");
        let pid_path = self.directory.path().join("sshd.pid");
        let authorized_keys = self.directory.path().join("authorized_keys");
        let log_path = self.directory.path().join("sshd.log");
        let config = format!(
            "Port {port}\n\
             ListenAddress 127.0.0.1\n\
             HostKey {host_key}\n\
             PidFile {pid_path}\n\
             AuthorizedKeysFile {authorized_keys}\n\
             PasswordAuthentication yes\n\
             KbdInteractiveAuthentication no\n\
             PubkeyAuthentication yes\n\
             AllowTcpForwarding yes\n\
             PermitRootLogin prohibit-password\n\
             StrictModes no\n\
             UsePAM no\n\
             AllowUsers {username}\n\
             Subsystem sftp internal-sftp\n\
             LogLevel ERROR\n",
            port = self.port,
            host_key = host_key.display(),
            pid_path = pid_path.display(),
            authorized_keys = authorized_keys.display(),
            username = self.username,
        );
        fs::write(&config_path, config).expect("write sshd_config");
        let validation = Command::new(sshd_path())
            .args(["-t", "-f"])
            .arg(&config_path)
            .status()
            .expect("validate sshd_config");
        assert!(validation.success(), "invalid fixture sshd_config");

        let log = fs::File::create(&log_path).expect("create fixture log");
        let mut command = Command::new(sshd_path());
        command
            .args(["-D", "-e", "-ddd", "-f"])
            .arg(&config_path)
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(log));
        let mut process = command.spawn().expect("start OpenSSH fixture");
        wait_until_listening(&mut process, &log_path);
        self.process = Some(process);
    }

    fn process_id(&self) -> u32 {
        self.process.as_ref().expect("fixture is running").id()
    }
}

impl Drop for OpenSshFixture {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            signal_process_group(process.id(), "-CONT");
            signal_process_group(process.id(), "-KILL");
            let _ = process.wait();
        }
    }
}

fn generate_key(path: &Path, passphrase: &str) {
    let status = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", passphrase, "-f"])
        .arg(path)
        .status()
        .expect("run ssh-keygen");
    assert!(status.success(), "ssh-keygen failed for {}", path.display());
}

fn sshd_path() -> &'static str {
    if Path::new("/usr/sbin/sshd").exists() {
        "/usr/sbin/sshd"
    } else {
        "sshd"
    }
}

fn signal_process_group(process_id: u32, signal: &str) {
    let process_group = format!("-{process_id}");
    let _ = Command::new("kill")
        .args([signal, &process_group])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn wait_until_listening(process: &mut Child, log_path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if fs::read_to_string(log_path).is_ok_and(|log| log.contains("Server listening")) {
            return;
        }
        assert!(
            process.try_wait().expect("inspect sshd process").is_none(),
            "OpenSSH fixture exited before listening"
        );
        assert!(Instant::now() < deadline, "OpenSSH fixture did not listen");
        thread::sleep(Duration::from_millis(20));
    }
}
