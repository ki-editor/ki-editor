#[derive(Debug)]
pub struct ProcessCommand {
    pub command: String,
    pub args: Vec<String>,
}

impl ProcessCommand {
    pub fn new(command: &str, args: &[&str]) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }
}
