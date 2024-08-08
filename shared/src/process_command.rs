use anyhow::Context;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct ProcessCommand {
    command: String,
    args: Vec<String>,
}

impl ProcessCommand {
    pub fn new(command: &str, args: &[&str]) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn spawn(&self) -> anyhow::Result<std::process::Child> {
        log::info!("ProcessCommand::spawn {:?} {:?}", self.command, self.args);
        // TODO: handle command spawning failure
        std::process::Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to spawn the command: {:?} with error: {:?}",
                    self,
                    e
                )
            })
    }

    pub fn run_with_input(&self, input: &str) -> anyhow::Result<String> {
        let mut child = self.spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .context("Failed to write to stdin")?;
        } else {
            return Err(anyhow::anyhow!("Failed to open stdin"));
        }

        let mut output = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            stdout
                .read_to_string(&mut output)
                .context("Failed to read from stdout")?;
        } else {
            return Err(anyhow::anyhow!("Failed to open stdout"));
        }

        let status = child.wait().context("Failed to wait on child process")?;

        if !status.success() {
            return Err(anyhow::anyhow!(
                "Command failed with exit code: {:?}",
                status.code()
            ));
        }

        Ok(output)
    }
}

impl std::fmt::Display for ProcessCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.command, self.args.join(" "))
    }
}
