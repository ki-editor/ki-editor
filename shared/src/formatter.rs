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
    pub fn command_string(&self) -> String {
        self.process_command.to_string()
    }
    pub fn format(&self, content: &str) -> anyhow::Result<String> {
        // Run the command with the args,
        // pass in the content using stdin,
        // get the output from the stdout

        let mut child = self.process_command.spawn()?;

        let stdin = child.stdin.as_mut().ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to open stdin for the command: {:?}",
                self.process_command
            )
        })?;

        stdin.write_all(content.as_bytes())?;

        // Read from stdout
        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stdout = String::from_utf8(output.stdout.clone())
                .unwrap_or_else(|_| format!("{:?}", output.stdout));
            let stderr = String::from_utf8(output.stderr.clone())
                .unwrap_or_else(|_| format!("{:?}", output.stderr));
            Err(anyhow::anyhow!(
                "Failed to format the content:\n[[STDOUT]]:\n\n{:#?}\n\n[[STDERR]]:\n\n{:#?}",
                stdout,
                stderr
            ))
        } else {
            Ok(String::from_utf8(output.stdout)?)
        }
    }
}
