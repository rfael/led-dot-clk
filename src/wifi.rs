use embassy_executor::{SpawnError, Spawner};
use embassy_net::{Runner, Stack, StackResources};
use embassy_time::{Duration, Ticker, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::rng::Rng;
use esp_radio::wifi::{self, Interface, Interfaces, WifiController, sta::StationConfig};
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
    stack: Stack<'static>,
}

impl WifiInterface {
    pub async fn init(
        spawner: &Spawner,
        rng: Rng,
        interfaces: Interfaces<'static>,
        controller: WifiController<'static>,
        config: &'static Config,
    ) -> WifiResult<Self> {
        let wifi_interface = interfaces.station;
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
        runner: Runner<'static, Interface<'static>>,
        controller: WifiController<'static>,
    ) -> WifiResult<()> {
        spawner.spawn(connection(controller, self.config.wifi()))?;
        spawner.spawn(net_task(runner))?;

        let mut ticker = Ticker::every(Duration::from_millis(500));
        while !self.stack.is_link_up() {
            log::info!("Wait for WiFi interface link up");
            ticker.next().await;
        }

        log::info!("Waiting to get IP address...");
        loop {
            if let Some(config) = self.stack.config_v4() {
                log::info!("Got IP: {}", config.address);
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
    const MAX_CONN_FAIL: usize = 5;
    log::debug!("Start connection task");

    loop {
        let station_config = StationConfig::default()
            .with_ssid(config.ssid())
            .with_password(config.password().into());
        let station_config = wifi::Config::Station(station_config);

        if let Err(err) = controller.set_config(&station_config) {
            log::error!("Settin WiFi conifg failed: {err}");
            Timer::after(config.reconnect_timeout()).await;
            continue;
        }

        log::info!("Wifi configured and started!");

        let mut conn_fails = 0;
        loop {
            log::info!("About to connect...");
            match controller.connect_async().await {
                Ok(info) => {
                    log::info!("Connected to {info:?}");
                    match controller.wait_for_disconnect_async().await {
                        Ok(info) => log::info!("Disconnected: {info:?}"),
                        Err(err) => {
                            log::error!("Waiting for disconnect failed: {err:?}");
                            conn_fails += 1;
                        }
                    }
                }
                Err(err) => {
                    log::error!("Failed to connect to wifi: {err:?}");
                    conn_fails += 1;
                }
            }

            if conn_fails >= MAX_CONN_FAIL {
                log::error!("Connection failed {conn_fails}/{MAX_CONN_FAIL}: reconfigure WiFi");
                break;
            }

            Timer::after(config.reconnect_timeout()).await
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}
