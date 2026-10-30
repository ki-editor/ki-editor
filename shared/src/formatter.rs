use std::{
    io::{Read, Write},
    process::{Child, ExitStatus},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use crate::language::ProcessCommand;

const FORMATTER_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Formatter {
    process_command: ProcessCommand,
    timeout: Duration,
}

impl From<ProcessCommand> for Formatter {
    fn from(value: ProcessCommand) -> Self {
        Self {
            process_command: value,
            timeout: FORMATTER_TIMEOUT,
        }
    }
}

impl Formatter {
    pub fn format(&self, content: &str) -> anyhow::Result<String> {
        // Run the command with the args,
        // pass in the content using stdin,
        // get the output from the stdout

        let mut child = self.process_command.spawn()?;
        let deadline = Instant::now() + self.timeout;

        let stdin = child.stdin.take().ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to open stdin for the command: {:?}",
                self.process_command
            )
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdout for the command"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stderr for the command"))?;

        // Every pipe is handled on its own thread, so that a formatter that stops
        // reading its input or writing its output can never block us past the deadline.
        let stdin = spawn_in_background({
            let content = content.to_string();
            move || {
                let mut stdin = stdin;
                stdin.write_all(content.as_bytes())
            }
        });
        let stdout = spawn_in_background(move || read_to_end(stdout));
        let stderr = spawn_in_background(move || read_to_end(stderr));

        let Some(status) = wait_until(&mut child, deadline)? else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow::anyhow!(
                "The formatter `{}` did not finish within {:?} and was killed",
                self.process_command,
                self.timeout
            ));
        };

        // The pipes may still be held open by processes spawned by the formatter,
        // so the deadline applies to them too.
        receive_until(&stdin, deadline)?;
        let stdout = receive_until(&stdout, deadline)?;
        let stderr = receive_until(&stderr, deadline)?;

        if !status.success() {
            let stdout =
                String::from_utf8(stdout.clone()).unwrap_or_else(|_| format!("{stdout:?}"));
            let stderr =
                String::from_utf8(stderr.clone()).unwrap_or_else(|_| format!("{stderr:?}"));
            Err(anyhow::anyhow!(
                "[[STDERR]]:\n{stderr}\n\n[[STDOUT]]:\n{stdout}"
            ))
        } else {
            Ok(String::from_utf8(stdout)?)
        }
    }
}

fn read_to_end(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).map(|_| buffer)
}

fn spawn_in_background<T: Send + 'static>(
    f: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> Receiver<std::io::Result<T>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(f());
    });
    receiver
}

fn receive_until<T>(
    receiver: &Receiver<std::io::Result<T>>,
    deadline: Instant,
) -> anyhow::Result<T> {
    receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| anyhow::anyhow!("Timed out communicating with the formatter"))?
        .map_err(Into::into)
}

/// Returns `None` if the child is still running once `deadline` is reached.
fn wait_until(child: &mut Child, deadline: Instant) -> anyhow::Result<Option<ExitStatus>> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod test_formatter {
    use std::time::{Duration, Instant};

    use super::Formatter;
    use crate::language::ProcessCommand;

    fn formatter(script: &str, timeout: Duration) -> Formatter {
        Formatter {
            process_command: ProcessCommand::new("sh", &["-c".to_string(), script.to_string()]),
            timeout,
        }
    }

    #[test]
    fn formats_through_stdin_and_stdout() {
        let output = formatter("tr a-z A-Z", Duration::from_secs(10))
            .format("hello")
            .unwrap();
        assert_eq!(output, "HELLO");
    }

    #[test]
    fn handles_output_larger_than_pipe_buffer() {
        let content = "x".repeat(1024 * 1024);
        let output = formatter("cat", Duration::from_secs(10))
            .format(&content)
            .unwrap();
        assert_eq!(output, content);
    }

    #[test]
    fn hanging_formatter_is_killed_after_timeout() {
        let started_at = Instant::now();
        let error = formatter("cat > /dev/null; sleep 60", Duration::from_millis(200))
            .format("hello")
            .unwrap_err();
        assert!(started_at.elapsed() < Duration::from_secs(5));
        assert!(
            error.to_string().contains("did not finish within"),
            "{error}"
        );
    }

    #[test]
    fn formatter_ignoring_its_stdin_is_killed_after_timeout() {
        let content = "x".repeat(1024 * 1024);
        let error = formatter("sleep 60", Duration::from_millis(200))
            .format(&content)
            .unwrap_err();
        assert!(
            error.to_string().contains("did not finish within"),
            "{error}"
        );
    }

    #[test]
    fn failing_formatter_reports_stderr() {
        let error = formatter("echo oops >&2; exit 1", Duration::from_secs(10))
            .format("hello")
            .unwrap_err();
        assert!(error.to_string().contains("oops"), "{error}");
    }
}
