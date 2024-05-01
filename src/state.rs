#[allow(dead_code)]
use std::time::SystemTime;

// pub enum CommandState {
//     Success,
//     Failed,
//     Init,
// }

#[derive(Debug)]
pub struct CommandInfo {
    pub script: String,
    pub output: Vec<u8>,
    pub success: bool,
    pub time_stamp: SystemTime,
}

impl CommandInfo {
    pub fn new(script: String, output: Vec<u8>, success: bool, time_stamp: SystemTime) -> Self {
        Self {
            script,
            output,
            success,
            time_stamp,
        }
    }
}
