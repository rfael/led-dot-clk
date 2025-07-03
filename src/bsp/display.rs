use core::convert::Infallible;

use chrono::{NaiveTime, Timelike};
use embassy_embedded_hal::shared_bus::SpiDeviceError;
use heapless::String;
use thiserror::Error;

use crate::{bsp::SpiDev, impl_from_variant};

use super::max7219_led_matrix::Max7219;

pub type SpiError = SpiDeviceError<esp_hal::spi::Error, Infallible>;

#[derive(Debug, Error)]
pub enum DisplayError {
    #[error("Spi error")]
    Spi(SpiError),
    #[error("String format error")]
    Format,
}
impl_from_variant!(DisplayError, Spi, SpiError);

pub type DisplayResult<T> = Result<T, DisplayError>;

pub struct Display {
    max7219: Max7219<SpiDev>,
}

impl Display {
    pub async fn init(mut max7219: Max7219<SpiDev>) -> DisplayResult<Self> {
        max7219.init().await?;
        let me = Self { max7219 };
        Ok(me)
    }

    pub async fn set_intensity(&mut self, intensity: u8) -> DisplayResult<()> {
        self.max7219.set_intensity(intensity & 0x0F).await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn write_str(&mut self, text: &str) -> DisplayResult<()> {
        self.max7219.write_str(text).await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn clear(&mut self) -> DisplayResult<()> {
        self.max7219.clear().await?;
        Ok(())
    }

    async fn write_hh_mm(&mut self, hour: u8, minute: u8) -> DisplayResult<()> {
        let mut s: String<4> = String::new();
        s.push((b'0' + hour / 10) as char)
            .map_err(|_| DisplayError::Format)?;
        s.push((b'0' + hour % 10) as char)
            .map_err(|_| DisplayError::Format)?;
        s.push((b'0' + minute / 10) as char)
            .map_err(|_| DisplayError::Format)?;
        s.push((b'0' + minute % 10) as char)
            .map_err(|_| DisplayError::Format)?;
        self.max7219.write_str(&s).await?;
        self.show_clock_dots(true).await?;
        Ok(())
    }

    async fn show_clock_dots(&mut self, show: bool) -> DisplayResult<()> {
        self.max7219.set_pixel(15, 5, show).await?;
        self.max7219.set_pixel(15, 3, show).await?;
        Ok(())
    }

    pub async fn write_time(&mut self, time: NaiveTime) -> DisplayResult<()> {
        self.write_hh_mm(time.hour() as _, time.minute() as _)
            .await?;
        self.show_clock_dots(true).await?;
        Ok(())
    }
}
