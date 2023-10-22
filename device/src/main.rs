#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
mod descriptor;

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

use common::{Command, DeviceResult};
use defmt::*;
use defmt_rtt as _;
use descriptor::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::gpio::{Input, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{bind_interrupts, pwm};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::Timer;
use embassy_usb::class::hid::{HidReader, HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler};

use usbd_hid::descriptor::{AsInputReport, KeyboardReport, SerializedDescriptor};
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    // Create the driver, from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0aa, 0xcafe);
    config.manufacturer = Some("Drewol");
    config.product = Some("Throttle");
    config.serial_number = Some("MSFS");

    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    // You can also add a Microsoft OS descriptor.
    // let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 256];
    let command_buffer = embassy_sync::channel::Channel::<NoopRawMutex, DeviceResult, 10>::new();

    let request_handler = MyRequestHandler {
        tx: command_buffer.sender(),
    };
    let mut device_handler = MyDeviceHandler::new();

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        // &mut msos_descriptor,
        &mut control_buf,
    );

    builder.handler(&mut device_handler);

    // Create classes on the builder.
    let config = embassy_usb::class::hid::Config {
        report_descriptor: DeviceData::desc(),
        request_handler: Some(&request_handler),
        poll_ms: 1,
        max_packet_size: 64,
    };
    let hid = HidReaderWriter::<_, 33, 65>::new(&mut builder, &mut state, config);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    let (reader, mut writer) = hid.split();

    // Do stuff with the class!
    let mut c: pwm::Config = Default::default();
    c.top = 0x8000;
    c.compare_b = 0;
    let mut led = pwm::Pwm::new_output_b(p.PWM_CH4, p.PIN_25, c.clone());

    let in_fut = async {
        loop {
            let mut report = DeviceData {
                output_buffer: [0; 32],
                input_buffer: [0; 32],
            };
            let command = command_buffer.receive().await;
            let response = match command {
                DeviceResult::Ok(Command::Ping(v)) => DeviceResult::Ok(Command::Pong(v)),
                DeviceResult::Ok(Command::Pong(v)) => DeviceResult::Ok(Command::Ping(v)),
                DeviceResult::Ok(Command::Throttle(v)) => {
                    c.compare_b = ((v / 100.0) * 0x8000 as f32) as _;
                    led.set_config(&c);
                    DeviceResult::Ok(Command::None)
                }
                DeviceResult::Ok(Command::AutoPilot(v)) => DeviceResult::Ok(Command::None),
                DeviceResult::Ok(Command::None) => DeviceResult::Ok(Command::None),
                e => e,
            };

            postcard::to_slice(&response, &mut report.output_buffer);

            match writer.write_serialize(&report).await {
                Ok(()) => {}
                Err(e) => {
                    cortex_m::peripheral::SCB::sys_reset();
                }
            };
        }
    };

    let out_fut = async {
        reader.run(true, &request_handler).await;
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, join(in_fut, out_fut)).await;
}

struct MyRequestHandler<'ch, const N: usize> {
    tx: Sender<'ch, NoopRawMutex, DeviceResult, N>,
}

impl<'a, const N: usize> RequestHandler for MyRequestHandler<'a, N> {
    fn get_report(&self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&self, id: ReportId, data: &[u8]) -> OutResponse {
        let Ok(message) = postcard::from_bytes::<Command>(&data) else {
            self.tx
                .try_send(DeviceResult::Err(common::DeviceError::UnknownCommand));

            return OutResponse::Accepted;
        };

        self.tx.try_send(DeviceResult::Ok(message));

        OutResponse::Accepted
    }

    fn set_idle_ms(&self, id: Option<ReportId>, dur: u32) {}

    fn get_idle_ms(&self, id: Option<ReportId>) -> Option<u32> {
        None
    }
}

struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
        } else {
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
        } else {
        }
    }
}
