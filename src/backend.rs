use crate::{
    state::{self, CommandInfo},
    CmdState,
};
use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    io::{self},
    process::Stdio,
    time::{self, Duration, SystemTime},
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, ChildStderr, ChildStdout, Command},
    time::sleep,
};
type HashId = u64;

#[derive(Debug)]
struct Cmd {
    cmd: String,
    is_shell: bool,
}

impl Hash for Cmd {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cmd.hash(state);
        time::SystemTime::now().hash(state);
    }
}

impl Cmd {
    fn new(cmd: String, is_shell: bool) -> Self {
        Self { cmd, is_shell }
    }
    fn runit_with_args(&self, args: String) -> io::Result<Child> {
        Command::new(&self.cmd)
            .arg(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
    fn runit(&self) -> io::Result<Child> {
        Command::new(&self.cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
    fn runit_with_shell(&self, args: String) -> io::Result<Child> {
        Command::new(&self.cmd)
            .arg("-c")
            .arg(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
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
        match c_stdout.read(&mut c_output).await {
            Ok(0) => {
                println!("Nothing to read...");
                cmd_info.state = "Completed".to_string();
                // check if stderr has a anything
                match c_stderr.read(&mut c_output).await {
                    Ok(0) => (),
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
            match c.kill().await {
                Ok(_) => {
                    println!("Killing it....");
                }
                Err(err) => {
                    println!("Cloudn't kill it, might have completed already..: {}", err);
                }
            }
            break;
        }
        sleep(Duration::from_secs(10)).await;
    }
}

pub async fn init(
    cmd: String,
    args: Option<String>,
    is_shell: bool,
    cmd_state: CmdState,
) -> Result<HashId, Box<dyn Error>> {
    let mut hasher = DefaultHasher::new();
    let new_command = Cmd::new(cmd, is_shell);
    new_command.hash(&mut hasher);
    let c_proc = match args {
        Some(args) => {
            if new_command.is_shell {
                new_command.runit_with_shell(args.clone())?
            } else {
                new_command.runit_with_args(args.clone())?
            }
        }
        None => new_command.runit()?,
    };
    let cmd_hasher = hasher.finish();

    tokio::spawn(async move { watch_cmd(c_proc, cmd_state, new_command.cmd, cmd_hasher).await });
    Ok(hasher.finish())
}
