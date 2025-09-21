use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use shlex::try_join;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use ulid::Ulid;

#[derive(Debug, Clone, Deserialize)]
pub struct Run<Dir = PathBuf> {
    pub id: Ulid,
    pub name: String,
    pub command: Vec<String>,
    pub run_dir: Dir,
}

pub type UnpreparedRun = Run<()>;
pub type PreparedRun = Run<PathBuf>;

impl<Dir> Run<Dir> {
    pub fn timestamp(&self) -> DateTime<Utc> {
        // Calculate start time from ULID timestamp
        let dt: DateTime<Utc> = self.id.datetime().into();
        dt
    }

    pub fn run_dir(&self, vault_dir: impl AsRef<Path>) -> PathBuf {
        let timestamp = self.timestamp();
        let date_str = timestamp.format("%Y-%m-%d").to_string();
        let time_str = timestamp.format("%H%M%S").to_string();

        // Prefix the run directory with time because
        // folders are sorted in natural order, not in lexicographical order,
        // For example, on macOS Finder, the order is:
        // 1. 01K5K478KNQ2ZXZG68MWM1Z9X6
        // 2. 01K5K4571FGKBFTTRJCG1J3DCZ
        // which is not the correct chronological order.
        // By adding time prefix, it will be sorted correctly.
        let run_dir_name = format!("{}-{}--{}", time_str, self.name, self.id.to_string());
        vault_dir.as_ref().join(date_str).join(&run_dir_name)
    }
}

impl Run<()> {
    pub fn setup_run_dir(
        &self,
        vault_dir: impl AsRef<std::path::Path>,
    ) -> io::Result<Run<PathBuf>> {
        setup_vault(&vault_dir)?;
        let run_dir = self.run_dir(&vault_dir);
        std::fs::create_dir_all(&run_dir)?;
        Ok(Run {
            id: self.id,
            name: self.name.clone(),
            command: self.command.clone(),
            run_dir,
        })
    }
}

impl Serialize for Run<()> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Run", 4)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("timestamp", &self.timestamp().to_rfc3339())?;
        state.end()
    }
}

impl Serialize for Run<PathBuf> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Run", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("timestamp", &self.timestamp().to_rfc3339())?;
        state.serialize_field("run_dir", &self.run_dir.to_string_lossy())?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

fn exit_code_from_status(status: ExitStatus) -> i32 {
    match status.code() {
        Some(c) => c,
        None => {
            // On Unix, process may be terminated by a signal.
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                status.signal().map(|s| 128 + s).unwrap_or(1)
            }
            #[cfg(not(unix))]
            {
                1
            }
        }
    }
}

impl Run<PathBuf> {
    pub fn exec(&self) -> std::io::Result<RunOutput> {
        if self.command.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty command"));
        }
        let program = &self.command[0];
        let args: Vec<&str> = self.command[1..].iter().map(|s| s.as_str()).collect();

        let mut env_vars = HashMap::new();
        env_vars.insert("CAPSULA_RUN_ID", self.id.to_string());
        env_vars.insert("CAPSULA_RUN_NAME", self.name.clone());
        env_vars.insert(
            "CAPSULA_RUN_DIRECTORY",
            self.run_dir.to_string_lossy().to_string(),
        );
        env_vars.insert("CAPSULA_RUN_TIMESTAMP", self.timestamp().to_rfc3339());
        if let Ok(cmd_str) = try_join(self.command.iter().map(|s| s.as_str())) {
            env_vars.insert("CAPSULA_RUN_COMMAND", cmd_str);
        }

        let start = Instant::now();

        let mut child = Command::new(program)
            .args(&args)
            .envs(&env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut child_stdout = child.stdout.take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to capture stdout"))?;
        let mut child_stderr = child.stderr.take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to capture stderr"))?;

        let t_out = thread::spawn(move || -> io::Result<Vec<u8>> {
            let mut cap = Vec::with_capacity(8 * 1024);
            let mut buf = [0u8; 8192];
            let mut console = io::stdout().lock();

            loop {
                let n = child_stdout.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                console.write_all(&buf[..n])?;
                cap.extend_from_slice(&buf[..n]);
            }
            console.flush()?;
            Ok(cap)
        });

        let t_err = thread::spawn(move || -> io::Result<Vec<u8>> {
            let mut cap = Vec::with_capacity(8 * 1024);
            let mut buf = [0u8; 8192];
            let mut console = io::stderr().lock();

            loop {
                let n = child_stderr.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                console.write_all(&buf[..n])?;
                cap.extend_from_slice(&buf[..n]);
            }
            console.flush()?;
            Ok(cap)
        });

        let status = child.wait()?;
        let duration = start.elapsed();
        let cap_out = t_out.join()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "stdout capture thread panicked"))??;
        let cap_err = t_err.join()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "stderr capture thread panicked"))??;

        let exit_code = exit_code_from_status(status);

        Ok(RunOutput {
            exit_code,
            stdout: String::from_utf8_lossy(&cap_out).to_string(),
            stderr: String::from_utf8_lossy(&cap_err).to_string(),
            duration,
        })
    }
}

fn setup_vault(path: impl AsRef<std::path::Path>) -> io::Result<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(path)?;

    // Place a .gitignore file to ignore all contents
    let gitignore_path = path.join(".gitignore");
    std::fs::write(
        gitignore_path,
        "\
# Automatically generated by Capsula
*",
    )?;

    Ok(())
}
