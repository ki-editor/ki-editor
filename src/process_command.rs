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
}
