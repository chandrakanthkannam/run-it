use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::{self},
    process::{Child, Command, Stdio},
    str::FromStr,
    time::{self, SystemTime},
};

use crate::{state, CmdState};
type HashId = u64;

#[derive(Debug)]
enum SupportedShells {
    Bash,
    PowerShell,
}

impl FromStr for SupportedShells {
    type Err = SupportedShellsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bash" => Ok(Self::Bash),
            "powershell" => Ok(Self::PowerShell),
            _ => Err(SupportedShellsError {}),
        }
    }
}

struct SupportedShellsError {}

#[derive(Debug)]
struct Cmd {
    shell: SupportedShells,
    script: String,
}

impl Hash for Cmd {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.script.hash(state);
        time::SystemTime::now().hash(state);
    }
}

impl Cmd {
    fn new(shell: SupportedShells, script: String) -> Self {
        Self { shell, script }
    }

    fn runit(&self) -> io::Result<Child> {
        match &self.shell {
            SupportedShells::Bash => Command::new("bash")
                .arg("-c")
                .arg(&self.script)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn(),
            SupportedShells::PowerShell => Command::new("powershell")
                .arg(&self.script)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn(),
        }
    }
}

pub async fn init(shell: String, script: String, cmd_state: CmdState) -> HashId {
    let supported_shell = match SupportedShells::from_str(&shell.to_lowercase()[..]) {
        Ok(supported_shell) => supported_shell,
        Err(_) => panic!("Unsupported shell"),
    };
    let mut hasher = DefaultHasher::new();
    let new_command = Cmd::new(supported_shell, script);
    new_command.hash(&mut hasher);

    match new_command.runit() {
        Ok(mut ch) => match ch.wait() {
            Ok(ex) => {
                let mut cmd_state = cmd_state.lock().unwrap();
                if ex.success() {
                    let cmd_info = state::CommandInfo::new(
                        new_command.script,
                        ch.wait_with_output().unwrap().stdout,
                        true,
                        SystemTime::now(),
                    );
                    cmd_state.insert(hasher.finish(), cmd_info);
                    hasher.finish()
                } else {
                    let cmd_info = state::CommandInfo::new(
                        new_command.script,
                        ch.wait_with_output().unwrap().stderr,
                        false,
                        SystemTime::now(),
                    );
                    cmd_state.insert(hasher.finish(), cmd_info);
                    hasher.finish()
                }
            }
            Err(_) => {
                let mut cmd_state = cmd_state.lock().unwrap();
                let cmd_info = state::CommandInfo::new(
                    new_command.script,
                    Vec::from("Script failed to start"),
                    false,
                    SystemTime::now(),
                );
                cmd_state.insert(hasher.finish(), cmd_info);
                hasher.finish()
            }
        },
        Err(_) => {
            let mut cmd_state = cmd_state.lock().unwrap();
            let cmd_info = state::CommandInfo::new(
                new_command.script,
                Vec::from("Child process failed to spwan"),
                false,
                SystemTime::now(),
            );
            cmd_state.insert(hasher.finish(), cmd_info);
            hasher.finish()
        }
    }
}
