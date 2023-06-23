use std::io::Write;

use crate::language::ProcessCommand;

pub struct Formatter {
    process_command: ProcessCommand,
}

impl From<ProcessCommand> for Formatter {
    fn from(value: ProcessCommand) -> Self {
        Self {
            process_command: value,
        }
    }
}

impl Formatter {
    pub fn format(&self, content: &str) -> anyhow::Result<String> {
        // Run the command with the args,
        // pass in the content using stdin,
        // get the output from the stdout

        let mut command = std::process::Command::new(&self.process_command.command);
        command.args(&self.process_command.args);

        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.as_mut().ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to open stdin for the command: {:?}",
                self.process_command
            )
        })?;

        // Read from stdout

        stdin.write_all(content.as_bytes())?;

        let output = child.wait_with_output()?;

        Ok(String::from_utf8(output.stdout)?)
    }
}
