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

        // Get the stderr
        if !output.stderr.is_empty() {
            Err(anyhow::anyhow!(
                "Failed to format the content: {:#?}",
                String::from_utf8(output.stderr.clone())
                    .unwrap_or_else(|_| format!("{:?}", output.stderr))
            ))
        } else {
            Ok(String::from_utf8(output.stdout)?)
        }
    }
}
