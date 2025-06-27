use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{
    gpio::{Output, OutputConfig},
    peripherals::Peripherals,
    rng::Rng,
    spi::{
        self,
        master::{Config as SpiConfig, ConfigError as SpiConfigError, Spi},
    },
    time::Rate,
    timer::{systimer::SystemTimer, timg::TimerGroup},
    Async,
};
use esp_wifi::{
    wifi::{Interfaces, WifiController},
    EspWifiController,
};
use thiserror::Error;

use crate::{max7219_led_matrix::Max7219, mk_static};

#[derive(Debug, Error)]
pub enum BoardError {
    #[error("WiFi initialization failed")]
    WifiInitFail,
    #[error("SPI initialization error: {0}")]
    SpiConfigError(#[from] SpiConfigError),
}

pub type BoardResult<T> = Result<T, BoardError>;
pub type SharedDevice<P> = Mutex<CriticalSectionRawMutex, P>;
pub type SpiDev = SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Async>, Output<'static>>;

pub struct Board {
    rng: Rng,
    wifi_controller: Option<WifiController<'static>>,
    wifi_interfaces: Option<Interfaces<'static>>,
    max7219: Option<Max7219<SpiDev>>,
}

impl Board {
    pub fn init(peripherals: Peripherals) -> BoardResult<Self> {
        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let rng = Rng::new(peripherals.RNG);

        let systimer = SystemTimer::new(peripherals.SYSTIMER);
        esp_hal_embassy::init(systimer.alarm0);

        // WiFi
        let esp_wifi_ctrl = esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK)
            .map_err(|_| BoardError::WifiInitFail)?;
        let esp_wifi_ctrl = mk_static!(EspWifiController<'static>, esp_wifi_ctrl);

        let (wifi_controller, wifi_interfaces) =
            esp_wifi::wifi::new(esp_wifi_ctrl, peripherals.WIFI)
                .map_err(|_| BoardError::WifiInitFail)?;

        // MAX7219
        let sck = peripherals.GPIO0;
        let miso = peripherals.GPIO2;
        let mosi = peripherals.GPIO4;
        let cs = peripherals.GPIO5;

        let spi = Spi::new(
            peripherals.SPI2,
            SpiConfig::default()
                .with_frequency(Rate::from_khz(100))
                .with_mode(spi::Mode::_0),
        )?
        .with_sck(sck)
        .with_miso(miso)
        .with_mosi(mosi)
        .into_async();
        let spi_bus = mk_static!(SharedDevice<Spi<'static, Async>>, Mutex::new(spi));
        let cs = Output::new(cs, esp_hal::gpio::Level::High, OutputConfig::default());
        let spi_device = SpiDevice::new(spi_bus, cs);

        let max7219 = Max7219::new(spi_device);

        let me = Self {
            rng,
            wifi_controller: Some(wifi_controller),
            wifi_interfaces: Some(wifi_interfaces),
            max7219: Some(max7219),
        };

        Ok(me)
    }

    pub fn rng(&self) -> Rng {
        self.rng
    }

    pub fn take_wifi_controller(&mut self) -> Option<WifiController<'static>> {
        self.wifi_controller.take()
    }

    pub fn take_wifi_interfaces(&mut self) -> Option<Interfaces<'static>> {
        self.wifi_interfaces.take()
    }

    pub fn take_max7219(&mut self) -> Option<Max7219<SpiDev>> {
        self.max7219.take()
    }
}
