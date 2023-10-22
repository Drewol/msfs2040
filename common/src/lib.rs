#![no_std]
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Ping(u32),
    Pong(u32),
    Throttle(f32),
    AutoPilot(bool),
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeviceError {
    NotSupported,
    UnknownCommand,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeviceResult {
    Ok(Command),
    Err(DeviceError),
}

impl From<core::result::Result<Command, DeviceError>> for DeviceResult {
    fn from(value: core::result::Result<Command, DeviceError>) -> Self {
        match value {
            Ok(v) => DeviceResult::Ok(v),
            Err(e) => DeviceResult::Err(e),
        }
    }
}

impl From<DeviceResult> for core::result::Result<Command, DeviceError> {
    fn from(val: DeviceResult) -> Self {
        match val {
            DeviceResult::Ok(v) => Ok(v),
            DeviceResult::Err(e) => Err(e),
        }
    }
}

pub type Result = core::result::Result<Command, DeviceError>;
