use embassy_executor::{SpawnError, Spawner};
use embassy_net::Runner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState,
};

use crate::impl_from_variant;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

const RECONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum WifiError {
    SpawnError(SpawnError),
}
impl_from_variant!(WifiError, SpawnError, SpawnError);

pub fn init_wifi_tasks(
    spawner: &Spawner,
    controller: WifiController<'static>,
    runner: Runner<'static, WifiDevice<'static>>,
) -> Result<(), WifiError> {
    spawner.spawn(connection(controller))?;
    spawner.spawn(net_task(runner))?;

    Ok(())
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(RECONNECT_TIMEOUT).await
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");

            println!("Scan");
            let result = controller.scan_n_async(10).await.unwrap();
            for ap in result {
                println!("{:?}", ap);
            }
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(RECONNECT_TIMEOUT).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
