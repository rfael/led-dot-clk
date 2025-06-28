use embassy_executor::{SpawnError, Spawner};
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Ticker, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::rng::Rng;
use esp_println::println;
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, Interfaces, WifiController, WifiDevice, WifiEvent,
    WifiState,
};
use thiserror::Error;

use crate::{
    config::{Config, WiFiConfig},
    mk_static,
};

#[derive(Debug, Error)]
pub enum WifiError {
    #[error("Spawnn error: {0}")]
    SpawnError(#[from] SpawnError),
}

pub type WifiResult<T> = Result<T, WifiError>;

pub struct WifiInterface {
    config: &'static Config,
    // controller: WifiController<'static>,
    stack: Stack<'static>,
}

impl WifiInterface {
    pub async fn init(
        spawner: &Spawner,
        mut rng: Rng,
        interfaces: Interfaces<'static>,
        controller: WifiController<'static>,
        config: &'static Config,
    ) -> WifiResult<Self> {
        let wifi_interface = interfaces.sta;

        let net_config = embassy_net::Config::dhcpv4(Default::default());

        let seed = (rng.random() as u64) << 32 | rng.random() as u64;

        let (stack, runner) = embassy_net::new(
            wifi_interface,
            net_config,
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed,
        );

        let me = Self { config, stack };

        me.launch(spawner, runner, controller).await?;

        Ok(me)
    }

    async fn launch(
        &self,
        spawner: &Spawner,
        runner: Runner<'static, WifiDevice<'static>>,
        controller: WifiController<'static>,
    ) -> WifiResult<()> {
        spawner.spawn(connection(controller, self.config.wifi()))?;
        spawner.spawn(net_task(runner))?;

        let mut ticker = Ticker::every(Duration::from_millis(500));
        while !self.stack.is_link_up() {
            println!("Wait for WiFi interface link up");
            ticker.next().await;
        }

        println!("Waiting to get IP address...");
        loop {
            if let Some(config) = self.stack.config_v4() {
                println!("Got IP: {}", config.address);
                break;
            }
            ticker.next().await;
        }

        Ok(())
    }

    pub fn stack(&self) -> Stack<'static> {
        self.stack
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>, config: &'static WiFiConfig) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(config.reconnect_timeout()).await
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: config.ssid().into(),
                password: config.password().into(),
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
                Timer::after(config.reconnect_timeout()).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
