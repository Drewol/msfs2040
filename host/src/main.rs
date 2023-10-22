mod device;

use std::{
    collections::HashMap,
    default,
    sync::{atomic::AtomicBool, mpsc::channel, Arc},
    thread,
    time::Duration,
};

use common::DeviceResult;
use device::Device;
use rusb::Language;
use slint::{Model, Weak};

slint::include_modules!();

#[derive(Debug)]
struct DataStruct {
    throttle: f64,
}

fn sim_thread(close_signal: Arc<AtomicBool>, handle: Weak<SimConnectApp>) -> anyhow::Result<()> {
    let mut device = Device::new(0xc0aa, 0xcafe);
    let (_, dev_name) = device.open()?;

    let mut devices: HashMap<String, Device> = [(dev_name, device)].into();

    while !close_signal.load(std::sync::atomic::Ordering::Relaxed) {
        handle.upgrade_in_event_loop(|x| {
            x.set_devices(slint::VecModel::from_slice(&[DeviceConnection {
                name: "Throttle".into(),
                warning: "".into(),
                status: ConnectionStatus::Connected,
            }]));
            x.set_sim_connect_status(ConnectionStatus::Disconnected);
        });

        let mut conn = simconnect::SimConnector::new();
        while !conn.connect("rp2040 host") {
            if close_signal.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }
            thread::sleep(Duration::from_secs(1))
        }

        handle.upgrade_in_event_loop(|x| x.set_sim_connect_status(ConnectionStatus::Connected));

        conn.add_data_definition(
            0,
            "GENERAL ENG THROTTLE LEVER POSITION:1",
            "Percent",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
        ); // Assign a sim variable to a client defined id

        let (throttle_tx, throttle_rx) = channel();

        handle.upgrade_in_event_loop(move |x| {
            x.on_throttle_changed(move |value| {
                throttle_tx.send(value as f64);
            })
        });

        conn.request_data_on_sim_object(
            0,
            0,
            0,
            simconnect::SIMCONNECT_PERIOD_SIMCONNECT_PERIOD_SIM_FRAME,
            0,
            0,
            0,
            0,
        );

        while !close_signal.load(std::sync::atomic::Ordering::Relaxed) {
            while let Ok(mut v) = throttle_rx.try_recv() {
                println!("{v}");
                unsafe {
                    conn.set_data_on_sim_object(0, 0, 0, 1, 8, &mut v as *mut f64 as _);
                }
            }

            match conn.get_next_message() {
                Ok(simconnect::DispatchResult::SimobjectData(data)) => unsafe {
                    if let 0 = data.dwDefineID {
                        let sim_data: DataStruct =
                            std::ptr::read_unaligned(std::ptr::addr_of!(data.dwData) as *const _);
                        let device_result = devices.get("Throttle").map(|d| {
                            d.command(
                                common::Command::Throttle(sim_data.throttle as _),
                                Duration::from_millis(500),
                            )
                        });

                        let (state, warning) = match device_result {
                            Some(Ok(v)) => (ConnectionStatus::Connected, "".to_string()),
                            Some(Err(v)) => (ConnectionStatus::Warning, format!("{}", v)),
                            None => {
                                if let Some(dev) = devices.get_mut("Throttle") {
                                    dev.open();
                                }
                                (ConnectionStatus::Disconnected, "".to_string())
                            }
                        };

                        handle.upgrade_in_event_loop(move |ui| {
                            let a = ui.get_devices();
                            a.set_row_data(
                                0,
                                DeviceConnection {
                                    name: "Throttle".into(),
                                    status: state,
                                    warning: warning.into(),
                                },
                            );

                            // ui.set_devices(vec![DeviceConnection {
                            //     name: "Throttle".into(),
                            //     status: state,
                            //     warning: warning.into(),
                            // }]);
                            ui.set_throttle(sim_data.throttle as _)
                        });
                    }
                },
                _ => thread::sleep(Duration::from_millis(16)),
            }
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut app = SimConnectApp::new()?;

    let handle = app.as_weak();

    let close_signal = Arc::new(AtomicBool::new(false));

    let sim_thread = {
        let close_signal = close_signal.clone();
        thread::spawn(move || sim_thread(close_signal, handle))
    };

    app.run()?;
    close_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    sim_thread.join();

    Ok(())
}
