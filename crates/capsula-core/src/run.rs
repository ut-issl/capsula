use chrono::{DateTime, Utc};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tracing::debug;
use ulid::Ulid;

#[derive(Debug, Clone, Deserialize)]
pub struct Run<Dir = PathBuf> {
    pub id: Ulid,
    pub name: String,
    pub command: Vec<String>,
    pub run_dir: Dir,
    pub project_root: PathBuf,
}

pub type UnpreparedRun = Run<()>;
pub type PreparedRun = Run<PathBuf>;

impl<Dir> Run<Dir> {
    pub fn timestamp(&self) -> DateTime<Utc> {
        // Calculate start time from ULID timestamp
        let dt: DateTime<Utc> = self.id.datetime().into();
        dt
    }

    fn gen_run_dir(&self, vault_dir: impl AsRef<Path>) -> PathBuf {
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
        let run_dir_name = format!("{}-{}", time_str, self.name);
        vault_dir.as_ref().join(date_str).join(&run_dir_name)
    }
}

impl Run<()> {
    pub fn setup_run_dir(
        &self,
        vault_dir: impl AsRef<std::path::Path>,
        max_retries: usize,
    ) -> io::Result<Run<PathBuf>> {
        debug!(
            "Setting up vault directory: {}",
            vault_dir.as_ref().display()
        );
        setup_vault(&vault_dir)?;

        // TODO: Consider removing retries as it is too conservative?
        let run_dir = {
            let mut attempt = 0;
            loop {
                let candidate = self.gen_run_dir(&vault_dir);
                debug!("Candidate run directory: {}", candidate.display());
                if candidate.exists() {
                    debug!(
                        "Directory already exists, retrying (attempt {})",
                        attempt + 1
                    );
                    // Slight delay before retrying
                    thread::sleep(Duration::from_millis(10 * (attempt as u64 + 1)));
                    attempt += 1;
                    if attempt >= max_retries {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            format!(
                                "Failed to create unique run directory after {max_retries} attempts",
                            ),
                        ));
                    }
                } else {
                    break candidate;
                }
            }
        };

        debug!("Creating run directory: {}", run_dir.display());
        std::fs::create_dir_all(&run_dir)?;
        debug!("Run directory created successfully");

        Ok(Run {
            id: self.id,
            name: self.name.clone(),
            command: self.command.clone(),
            run_dir,
            project_root: self.project_root.clone(),
        })
    }
}

impl Serialize for Run<()> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Run", 5)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("timestamp", &self.timestamp().to_rfc3339())?;
        state.serialize_field("project_root", &self.project_root.to_string_lossy())?;
        state.end()
    }
}

impl Serialize for Run<PathBuf> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Run", 6)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("timestamp", &self.timestamp().to_rfc3339())?;
        state.serialize_field("run_dir", &self.run_dir.to_string_lossy())?;
        state.serialize_field("project_root", &self.project_root.to_string_lossy())?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutput {
    pub exit_code: i32,
    stdout: String,
    stderr: String,
    duration: Duration,
}

fn exit_code_from_status(status: ExitStatus) -> i32 {
    status.code().unwrap_or_else(|| {
        // On Unix, process may be terminated by a signal.
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            status.signal().map_or(1, |s| 128 + s)
        }
        #[cfg(not(unix))]
        {
            1
        }
    })
}

impl Run<PathBuf> {
    pub fn exec(&self) -> std::io::Result<RunOutput> {
        if self.command.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty command"));
        }
        let program = &self.command[0];
        let args: Vec<&str> = self.command[1..].iter().map(String::as_str).collect();

        debug!("Executing command: {} with {} args", program, args.len());

        let mut env_vars = HashMap::new();
        env_vars.insert("CAPSULA_RUN_ID", self.id.to_string());
        env_vars.insert("CAPSULA_RUN_NAME", self.name.clone());
        env_vars.insert(
            "CAPSULA_RUN_DIRECTORY",
            self.run_dir.to_string_lossy().to_string(),
        );
        env_vars.insert("CAPSULA_RUN_TIMESTAMP", self.timestamp().to_rfc3339());
        let command_display = shlex::try_join(self.command.iter().map(String::as_str))
            .unwrap_or_else(|_| self.command.join(" "));
        env_vars.insert("CAPSULA_RUN_COMMAND", command_display);

        let pre_run_output_json_path = self.run_dir.join("_capsula").join("pre-run.json");
        env_vars.insert(
            "CAPSULA_PRE_RUN_OUTPUT_PATH",
            pre_run_output_json_path.to_string_lossy().to_string(),
        );
        env_vars.insert(
            "CAPSULA_PROJECT_ROOT",
            self.project_root.to_string_lossy().to_string(),
        );

        debug!(
            "Setting {} environment variables for command execution",
            env_vars.len()
        );

        let start = Instant::now();

        debug!("Spawning child process");
        let mut child = Command::new(program)
            .args(&args)
            .envs(&env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Failed to capture stdout"))?;
        let mut child_stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("Failed to capture stderr"))?;

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

        debug!("Waiting for child process to complete");
        let status = child.wait()?;
        let duration = start.elapsed();
        debug!("Child process completed in {:?}", duration);

        let cap_out = t_out
            .join()
            .map_err(|_| io::Error::other("stdout capture thread panicked"))??;
        let cap_err = t_err
            .join()
            .map_err(|_| io::Error::other("stderr capture thread panicked"))??;

        let exit_code = exit_code_from_status(status);
        debug!(
            "Command finished with exit code {}, captured {} bytes stdout, {} bytes stderr",
            exit_code,
            cap_out.len(),
            cap_err.len()
        );

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
        debug!("Vault directory already exists: {}", path.display());
        return Ok(());
    }

    debug!("Creating vault directory: {}", path.display());
    std::fs::create_dir_all(path)?;

    // Place a .gitignore file to ignore all contents
    let gitignore_path = path.join(".gitignore");
    debug!("Creating .gitignore in vault directory");
    std::fs::write(
        gitignore_path,
        "\
# Automatically generated by Capsula
*",
    )?;

    Ok(())
}
