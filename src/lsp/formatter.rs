use std::io::Write;

use super::language::Language;

pub struct Formatter {
    command: String,
    args: Vec<String>,
}

impl Formatter {
    pub fn new(command: &str, args: &[&str]) -> Self {
        Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn from_language(language: &Language) -> Option<Self> {
        match language {
            Language::Rust => Formatter::new("rustfmt", &[]).into(),
            Language::Typescript => Formatter::new("prettierd", &[".ts"]).into(),
            Language::TypescriptReact => Formatter::new("prettierd", &[".tsx"]).into(),
            Language::JavaScript => Formatter::new("prettierd", &[".js"]).into(),
            Language::JavaScriptReact => Formatter::new("prettierd", &[".jsx"]).into(),
            Language::Markdown => Formatter::new("prettierd", &[".md"]).into(),
        }
    }

    pub fn format(&self, content: &str) -> anyhow::Result<String> {
        // Run the command with the args,
        // pass in the content using stdin,
        // get the output from the stdout

        let mut command = std::process::Command::new(&self.command);
        command.args(&self.args);

        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.as_mut().ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to open stdin for the command: {}",
                self.command.clone()
            )
        })?;

        // Read from stdout

        stdin.write_all(content.as_bytes())?;

        let output = child.wait_with_output()?;

        Ok(String::from_utf8(output.stdout)?)
    }
}

#[cfg(test)]
mod test_formatter {
    use crate::lsp::language::Language;

    use super::*;

    fn run_test(language: &Language, content: &str, expected: &str) {
        let formatter = Formatter::from_language(language).unwrap();
        let formatted = formatter.format(content).unwrap();
        assert_eq!(formatted, expected);
    }

    #[test]
    fn rust() {
        run_test(&Language::Rust, "fn main(){}", "fn main() {}\n");
    }

    #[test]
    fn typescript() {
        run_test(&Language::Typescript, "let x:Int=1", "let x: Int = 1;\n");
    }

    #[test]
    fn typescript_react() {
        run_test(
            &Language::TypescriptReact,
            "let x:Int=<x >1</ x>",
            "let x: Int = <x>1</x>;\n",
        );
    }

    #[test]
    fn javascript() {
        run_test(&Language::JavaScript, "let x=1", "let x = 1;\n");
    }

    #[test]
    fn javascript_react() {
        run_test(
            &Language::JavaScriptReact,
            "let x=<x >1</ x>",
            "let x = <x>1</x>;\n",
        );
    }
}
