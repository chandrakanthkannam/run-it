#[allow(dead_code)]
use std::time::SystemTime;

// #[derive(Debug)]
// pub enum CommandState {
//     Success,
//     Failed,
//     Init,
// }

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub script: String,
    pub output: Vec<u8>,
    pub state: String,
    pub time_stamp: SystemTime,
    pub timed_out: bool,
}

impl CommandInfo {
    pub fn new(
        script: String,
        output: Vec<u8>,
        state: String,
        time_stamp: SystemTime,
        timed_out: bool,
    ) -> Self {
        Self {
            script,
            output,
            state,
            time_stamp,
            timed_out,
        }
    }
}
