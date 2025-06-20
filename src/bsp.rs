use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{
    peripherals::Peripherals,
    rng::Rng,
    timer::{systimer::SystemTimer, timg::TimerGroup},
};
use esp_wifi::{
    wifi::{Interfaces, WifiController},
    EspWifiController,
};

use crate::mk_static;

#[derive(Debug)]
pub enum BoardError {
    WifiInitFail,
}

pub type BoardResult<T> = Result<T, BoardError>;
pub type SharedDevice<P> = Mutex<CriticalSectionRawMutex, P>;

pub struct Board {
    rng: Rng,
    wifi_controller: Option<WifiController<'static>>,
    wifi_interfaces: Option<Interfaces<'static>>,
}

impl Board {
    pub fn init(peripherals: Peripherals) -> BoardResult<Self> {
        let timg0 = TimerGroup::new(peripherals.TIMG0);
        let rng = Rng::new(peripherals.RNG);

        let systimer = SystemTimer::new(peripherals.SYSTIMER);
        esp_hal_embassy::init(systimer.alarm0);

        let esp_wifi_ctrl = esp_wifi::init(timg0.timer0, rng, peripherals.RADIO_CLK)
            .map_err(|_| BoardError::WifiInitFail)?;
        let esp_wifi_ctrl = mk_static!(EspWifiController<'static>, esp_wifi_ctrl);

        let (wifi_controller, wifi_interfaces) =
            esp_wifi::wifi::new(esp_wifi_ctrl, peripherals.WIFI)
                .map_err(|_| BoardError::WifiInitFail)?;

        let me = Self {
            rng,
            wifi_controller: Some(wifi_controller),
            wifi_interfaces: Some(wifi_interfaces),
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
}
