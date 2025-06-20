use core::convert::Infallible;

use embassy_embedded_hal::shared_bus::SpiDeviceError;
use embassy_executor::SpawnError;
use max7219_async::{DecodeMode, Max7219};
use thiserror::Error;

use crate::{bsp::SpiDev, impl_from_variant};

pub type SpiError = SpiDeviceError<esp_hal::spi::Error, Infallible>;

#[derive(Debug, Error)]
pub enum DisplayError {
    #[error("Spawnn error: {0}")]
    SpawnError(#[from] SpawnError),
    #[error("Spi error")]
    Spi(SpiError),
}
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
        me.max7219.set_decode_mode(DecodeMode::NoDecode).await?;
        me.max7219.set_scan_limit(8).await?;
        me.clear().await?;
        me.set_intensity(0x01).await?;
        Ok(me)
    }

    pub async fn write_i32(&mut self, value: i32) -> DisplayResult<()> {
        self.max7219.write_integer(value).await?;
        Ok(())
    }

    pub async fn set_intensity(&mut self, intensity: u8) -> DisplayResult<()> {
        self.max7219.set_intensity(intensity & 0x0F).await?;
        Ok(())
    }

    pub async fn clear(&mut self) -> DisplayResult<()> {
        self.max7219.clear().await?;
        Ok(())
    }
}
