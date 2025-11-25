use std::collections::HashMap;
use std::process::{Child, Command, Stdio};

pub struct ProcessManager {
    running_processes: HashMap<String, Child>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            running_processes: HashMap::new(),
        }
    }

    pub fn toggle(&mut self, command: &'static str, args: &[String]) {
        let key = format!("{} {}", command, args.join(" "));
        if self.running_processes.contains_key(&key) {
            self.stop(&key);
        } else {
            self.start(&key, command, args);
        }
    }

    fn start(&mut self, key: &str, command: &'static str, args: &[String]) {
        match Command::new(command)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.running_processes.insert(key.to_string(), child);
            }
            Err(_e) => {}
        }
    }
    fn stop(&mut self, key: &str) {
        if let Some(mut child) = self.running_processes.remove(key) {
            let _ = child.kill();
        }
    }

    pub fn shutdown(&mut self) {
        let keys: Vec<String> = self.running_processes.keys().cloned().collect();
        for key in keys {
            self.stop(&key);
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
