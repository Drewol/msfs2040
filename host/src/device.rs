use std::time::Duration;

use anyhow::{anyhow, bail};
use common::{Command, DeviceResult};
use rusb::{DeviceHandle, GlobalContext};

pub struct Device {
    vid: u16,
    pid: u16,
    device: Option<DeviceHandle<GlobalContext>>,
}

impl Device {
    pub fn new(vid: u16, pid: u16) -> Self {
        Self {
            vid,
            pid,
            device: None,
        }
    }

    pub fn open(&mut self) -> anyhow::Result<(String, String)> {
        self.device = rusb::open_device_with_vid_pid(self.vid, self.pid);
        let Some(device) = self.device.as_ref() else {
            bail!("Failed to open device");
        };

        let desc = device.device().device_descriptor()?;
        let manufacturer = device.read_manufacturer_string_ascii(&desc)?;
        let product = device.read_product_string_ascii(&desc)?;

        Ok((manufacturer, product))
    }

    pub fn command(&self, command: Command, timeout: Duration) -> anyhow::Result<Command> {
        let Some(device) = self.device.as_ref() else {
            bail!("Device not open")
        };

        let mut buf = [0u8; 64];
        postcard::to_slice(&command, &mut buf)?;
        println!("---> {:?}", command);
        device.write_bulk(1, &buf, timeout)?;
        device.read_bulk(129, &mut buf, timeout)?;
        let reply: DeviceResult = postcard::from_bytes(&buf)?;
        println!("<--- {:?}", reply);

        match reply {
            DeviceResult::Ok(v) => Ok(v),
            DeviceResult::Err(e) => Err(anyhow!("{:?}", e)),
        }
    }
}
