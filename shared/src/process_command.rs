use anyhow::Context;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct ProcessCommand {
    command: String,
    args: Vec<String>,
}

pub enum SpawnCommandResult {
    CommandNotFound { command_name: String },
    Spawned(anyhow::Result<std::process::Child>),
}

impl SpawnCommandResult {
    pub fn into_result(self) -> anyhow::Result<std::process::Child> {
        match self {
            SpawnCommandResult::CommandNotFound { command_name } => {
                Err(anyhow::anyhow!("Command '{command_name}' is not found"))
            }
            SpawnCommandResult::Spawned(result) => result,
        }
    }
}

impl ProcessCommand {
    pub fn new(command: &str, args: &[&str]) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn spawn(&self) -> SpawnCommandResult {
        log::info!("ProcessCommand::spawn {:?} {:?}", self.command, self.args);
        if which::which(&self.command).is_err() {
            log::info!("ProcessCommand::spawn: Failed to locate {:?}", self.command);
            SpawnCommandResult::CommandNotFound {
                command_name: self.command.clone(),
            }
        } else {
            SpawnCommandResult::Spawned(
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
                    }),
            )
        }
    }

    pub fn run_with_input(&self, input: &str) -> anyhow::Result<String> {
        let mut child = self.spawn().into_result()?;

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
            let stderr = child
                .stderr
                .take()
                .map(|mut stderr| -> anyhow::Result<_> {
                    let mut output = String::new();
                    stderr.read_to_string(&mut output)?;
                    Ok(output)
                })
                .unwrap_or(Ok("[No stderr]".to_string()))
                .unwrap_or("[Failed to obtain stderr]".to_string());
            return Err(anyhow::anyhow!(
                "Command failed with exit code: {}\n\nSTDERR =\n\n{}\n\nSTDOUT =\n\n{}",
                status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or("[Process terminated by signal]".to_string()),
                stderr,
                output,
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

#[cfg(test)]
mod test_process_command {
    use super::ProcessCommand;

    #[test]
    fn failed_command_includes_exit_code_and_stderr() {
        let err = ProcessCommand::new("bash", &["-c", "yo"])
            .run_with_input("hello")
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "
Command failed with exit code: 127

STDERR =

bash: yo: command not found


STDOUT =

"
            .trim_start()
        )
    }
}
