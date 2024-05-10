use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    io::{self, Read},
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
    str::FromStr,
    time::{self, Duration, SystemTime},
};

use tokio::time::sleep;

use crate::{
    state::{self, CommandInfo},
    CmdState,
};
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

async fn cmd_capture_output(
    mut c_stdout: ChildStdout,
    mut c_stderr: ChildStderr,
    cmd_state: CmdState,
    cmd_hash: HashId,
    mut cmd_info: CommandInfo,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // buffer to write ouput to
    let mut c_output = [0u8; 1024];

    loop {
        println!("Atempting to read..");
        match c_stdout.read(&mut c_output) {
            Ok(0) => {
                println!("Nothing to read...");
                cmd_info.state = "Completed".to_string();
                // check if stderr has a anything
                match c_stderr.read(&mut c_output) {
                    Ok(n) => {
                        cmd_info.state = "Failed".to_string();
                        println!("Reading: {} bytes", n);
                        cmd_info.output.append(&mut c_output[0..n].into());
                    }
                    Err(_) => {
                        println!("Failed writting to buffer");
                    }
                }
                break;
            }
            Ok(n) => {
                println!("Reading: {} bytes", n);
                cmd_info.output.append(&mut c_output[0..n].into());
            }
            Err(_) => {
                println!("Failed writting to buffer");
            }
        }
        // get a mutex lock
        let mut cmd_state = cmd_state.lock().unwrap();
        cmd_state.insert(cmd_hash, cmd_info.clone());

        std::thread::sleep(Duration::new(5, 0));
    }

    // lets insert output again at the end here
    let mut cmd_state = cmd_state.lock().unwrap();
    cmd_state.insert(cmd_hash, cmd_info.clone());
    Ok(())
}

async fn watch_cmd(mut c: Child, cmd_state: CmdState, script: String, cmd_hash: HashId) {
    println!("Watching cmd now....");
    let c_stdout = c.stdout.take().unwrap();
    let c_stderr = c.stderr.take().unwrap();
    // recording start time, which is used to decide on timeout for the command
    let start_time = SystemTime::now();

    // Initiate command info
    let cmd_info = state::CommandInfo::new(
        script,
        Vec::new(),
        "in-progress".to_string(),
        SystemTime::now(),
        false,
    );

    tokio::spawn(async move {
        cmd_capture_output(c_stdout, c_stderr, cmd_state, cmd_hash, cmd_info).await
    });

    while c.try_wait().unwrap().take().is_none() {
        println!(
            "Wating for command to exit...:{:?}",
            SystemTime::now().duration_since(start_time).unwrap()
        );
        // timeout and kill the command if continues to run long
        if SystemTime::now().duration_since(start_time).unwrap() > Duration::new(50, 0) {
            match c.kill() {
                Ok(_) => {
                    println!("Killing it....");
                }
                Err(err) => {
                    println!("Cloudn't kill it, might have completed already..: {}", err);
                }
            }
            break;
        }
        sleep(Duration::new(10, 0)).await;
    }
}

pub async fn init(
    shell: String,
    script: String,
    cmd_state: CmdState,
) -> Result<HashId, Box<dyn Error>> {
    let supported_shell = match SupportedShells::from_str(&shell.to_lowercase()[..]) {
        Ok(supported_shell) => supported_shell,
        Err(_) => panic!("Unsupported shell"),
    };
    let mut hasher = DefaultHasher::new();
    let new_command = Cmd::new(supported_shell, script);
    new_command.hash(&mut hasher);

    let c_proc = new_command.runit()?;
    let cmd_hasher = hasher.finish();

    tokio::spawn(async move {
        watch_cmd(
            c_proc,
            cmd_state.clone(),
            new_command.script.clone(),
            cmd_hasher,
        )
        .await
    });
    Ok(hasher.finish())
}
