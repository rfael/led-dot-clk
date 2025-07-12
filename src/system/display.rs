use core::convert::Infallible;

use chrono::{NaiveTime, Timelike};
use embassy_embedded_hal::shared_bus::SpiDeviceError;
use heapless::String;
use thiserror::Error;

use crate::{
    bsp::{Max7219, Max7219Error, SharedDevice, SpiDev},
    impl_from_variant,
};

pub type SpiError = SpiDeviceError<esp_hal::spi::Error, Infallible>;

#[derive(Debug, Error)]
pub enum DisplayError {
    #[error("Max7219 error")]
    Max7219(Max7219Error<SpiError>),
    #[error("String format error")]
    Format,
}
impl_from_variant!(DisplayError, Max7219, Max7219Error<SpiError>);

pub type DisplayResult<T> = Result<T, DisplayError>;

pub struct Display {
    max7219: &'static SharedDevice<Max7219<SpiDev>>,
}

impl Display {
    pub async fn init(max7219: &'static SharedDevice<Max7219<SpiDev>>) -> DisplayResult<Self> {
        max7219.lock().await.init().await?;
        let me = Self { max7219 };
        Ok(me)
    }

    pub async fn set_intensity(&self, intensity: u8) -> DisplayResult<()> {
        self.max7219.lock().await.set_intensity(intensity & 0x0F).await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn write_str(&self, x: i32, text: &str) -> DisplayResult<()> {
        self.max7219.lock().await.write_str(x, text).await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn clear(&self) -> DisplayResult<()> {
        self.max7219.lock().await.clear().await?;
        Ok(())
    }

    async fn write_hh_mm(&self, hour: u8, minute: u8) -> DisplayResult<()> {
        let mut h: String<2> = String::new();
        let mut m: String<2> = String::new();
        h.push((b'0' + hour / 10) as char).map_err(|_| DisplayError::Format)?;
        h.push((b'0' + hour % 10) as char).map_err(|_| DisplayError::Format)?;
        m.push((b'0' + minute / 10) as char).map_err(|_| DisplayError::Format)?;
        m.push((b'0' + minute % 10) as char).map_err(|_| DisplayError::Format)?;
        {
            let mut disp = self.max7219.lock().await;
            disp.write_str(0, &h).await?;
            disp.write_str(17, &m).await?;
        }
        Ok(())
    }

    async fn show_clock_dots(&self, show: bool) -> DisplayResult<()> {
        {
            let mut disp = self.max7219.lock().await;
            disp.set_pixel(15, 3, show).await?;
            disp.set_pixel(15, 5, show).await?;
        }

        Ok(())
    }

    pub async fn write_time(&self, time: NaiveTime) -> DisplayResult<()> {
        self.write_hh_mm(time.hour() as _, time.minute() as _).await?;
        self.show_clock_dots(true).await?;
        Ok(())
    }
}
