use ds3231::{Config as RtcConfig, DS3231, DS3231Error, SquareWaveFrequency, TimeRepresentation};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{
    Async,
    gpio::{Output, OutputConfig},
    i2c::master::{Config as I2cConfig, ConfigError as I2cConfigError, Error as I2cError, I2c},
    peripherals::Peripherals,
    rng::Rng,
    spi::{
        self,
        master::{Config as SpiConfig, ConfigError as SpiConfigError, Spi},
    },
    time::Rate,
    timer::{systimer::SystemTimer, timg::TimerGroup},
};
use esp_wifi::{
    EspWifiController,
    wifi::{Interfaces, WifiController},
};
use thiserror::Error;

use crate::{impl_from_variant, mk_static};

mod max7219_led_matrix;

pub use max7219_led_matrix::{Max7219, Max7219Error};

#[derive(Debug, Error)]
pub enum BoardError {
    #[error("WiFi initialization failed")]
    WifiInitFail,
    #[error("SPI initialization error: {0}")]
    SpiConfigError(#[from] SpiConfigError),
    #[error("I2C initialization error: {0}")]
    I2CConfigError(#[from] I2cConfigError),
    #[error("RTC error: {0:?}")]
    RtcError(DS3231Error<I2cError>),
}
impl_from_variant!(BoardError, RtcError, DS3231Error<I2cError>);

pub type BoardResult<T> = Result<T, BoardError>;
pub type SharedDevice<P> = Mutex<CriticalSectionRawMutex, P>;
pub type SpiDev = SpiDevice<'static, CriticalSectionRawMutex, Spi<'static, Async>, Output<'static>>;

pub type RtcDevice = DS3231<I2c<'static, Async>>;

pub struct Board {
    rng: Rng,
    wifi_controller: Option<WifiController<'static>>,
    wifi_interfaces: Option<Interfaces<'static>>,
    display: &'static SharedDevice<Max7219<SpiDev>>,
    rtc: &'static SharedDevice<RtcDevice>,
}

impl Board {
    pub async fn init(peripherals: Peripherals) -> BoardResult<Self> {
        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let rng = Rng::new(peripherals.RNG);

        let systimer = SystemTimer::new(peripherals.SYSTIMER);
        esp_hal_embassy::init(systimer.alarm0);

        // WiFi
        let esp_wifi_ctrl = esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK).map_err(|_| BoardError::WifiInitFail)?;
        let esp_wifi_ctrl = mk_static!(EspWifiController<'static>, esp_wifi_ctrl);

        let (wifi_controller, wifi_interfaces) =
            esp_wifi::wifi::new(esp_wifi_ctrl, peripherals.WIFI).map_err(|_| BoardError::WifiInitFail)?;

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
        let display = mk_static!(SharedDevice<Max7219<SpiDev>>, Mutex::new(max7219));

        // DS3231
        let sda = peripherals.GPIO10;
        let scl = peripherals.GPIO18;
        let i2c = I2c::new(peripherals.I2C0, I2cConfig::default().with_frequency(Rate::from_khz(100)))?
            .with_sda(sda)
            .with_scl(scl)
            .into_async();
        let mut rtc = DS3231::new(i2c, 0x68);
        let rtc_config = RtcConfig {
            time_representation: TimeRepresentation::TwentyFourHour,
            square_wave_frequency: SquareWaveFrequency::Hz8192,
            interrupt_control: ds3231::InterruptControl::SquareWave,
            battery_backed_square_wave: false,
            oscillator_enable: ds3231::Oscillator::Disabled,
        };
        rtc.configure(&rtc_config).await?;
        let rtc = mk_static!(SharedDevice<DS3231<I2c<Async>>>, Mutex::new(rtc));

        let me = Self {
            rng,
            wifi_controller: Some(wifi_controller),
            wifi_interfaces: Some(wifi_interfaces),
            display,
            rtc,
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

    pub fn display(&self) -> &'static SharedDevice<Max7219<SpiDev>> {
        self.display
    }

    pub fn rtc(&self) -> &'static SharedDevice<RtcDevice> {
        self.rtc
    }
}
