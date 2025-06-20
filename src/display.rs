use core::convert::Infallible;

use embassy_embedded_hal::shared_bus::SpiDeviceError;
use embassy_executor::SpawnError;
use max7219_async::Max7219;

use crate::{bsp::SpiDev, impl_from_variant};

pub type SpiError = SpiDeviceError<esp_hal::spi::Error, Infallible>;

#[derive(Debug)]
pub enum DisplayError {
    SpawnError(SpawnError),
    Spi(SpiError),
    Other(&'static str),
}
impl_from_variant!(DisplayError, SpawnError, SpawnError);
impl_from_variant!(DisplayError, Spi, SpiError);

pub type DisplayResult<T> = Result<T, DisplayError>;

pub struct Display {
    max7219: Max7219<SpiDev>,
}

impl Display {
    pub async fn init(max7219: Max7219<SpiDev>) -> DisplayResult<Self> {
        let mut me = Self { max7219 };
        me.max7219.init().await?;
        me.max7219.power_on().await?;
        me.max7219.set_intensity(0x08).await?;
        Ok(me)
    }

    pub async fn write_i32(&mut self, value: i32) -> DisplayResult<()> {
        self.max7219.write_integer(value).await?;
        Ok(())
    }
}
