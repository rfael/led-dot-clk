use core::convert::Infallible;

use chrono::{DateTime, Timelike, Utc};
use embassy_embedded_hal::shared_bus::SpiDeviceError;
use embassy_executor::{SpawnError, Spawner};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Sender},
};
use esp_println::println;
use heapless::String;
use thiserror::Error;

use crate::{
    bsp::SpiDev, config::Config, impl_from_variant, max7219_led_matrix::Max7219, mk_static,
};

pub type SpiError = SpiDeviceError<esp_hal::spi::Error, Infallible>;

#[derive(Debug, Error)]
pub enum DisplayError {
    #[error("Spawnn error: {0}")]
    Spawn(#[from] SpawnError),
    #[error("Spi error")]
    Spi(SpiError),
    #[error("String format error")]
    Format,
}
impl_from_variant!(DisplayError, Spi, SpiError);

pub type DisplayResult<T> = Result<T, DisplayError>;

pub struct Display {
    config: &'static Config,
    max7219: Max7219<SpiDev>,
    channel: &'static Channel<CriticalSectionRawMutex, DisplayCommad, 8>,
}

impl Display {
    pub async fn init(
        mut max7219: Max7219<SpiDev>,
        config: &'static Config,
    ) -> DisplayResult<Self> {
        max7219.init().await?;
        let channel =
            mk_static!(Channel<CriticalSectionRawMutex, DisplayCommad, 8>, Channel::new());
        let me = Self {
            config,
            max7219,
            channel,
        };
        Ok(me)
    }

    pub async fn set_intensity(&mut self, intensity: u8) -> DisplayResult<()> {
        self.max7219.set_intensity(intensity & 0x0F).await?;
        Ok(())
    }

    pub async fn write_str(&mut self, text: &str) -> DisplayResult<()> {
        self.max7219.write_str(text).await?;
        Ok(())
    }

    pub async fn clear(&mut self) -> DisplayResult<()> {
        self.max7219.clear().await?;
        Ok(())
    }

    pub async fn write_time(&mut self, hour: u8, minute: u8) -> DisplayResult<()> {
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

    pub async fn launch(
        self,
        spawner: &Spawner,
    ) -> DisplayResult<Sender<'static, CriticalSectionRawMutex, DisplayCommad, 8>> {
        let tx = self.channel.sender();
        spawner.spawn(display_task(self))?;
        Ok(tx)
    }
}

pub enum DisplayCommad {
    ShowTime(DateTime<Utc>),
    Clear,
    ShowText(&'static str),
}

impl From<DateTime<Utc>> for DisplayCommad {
    fn from(value: DateTime<Utc>) -> Self {
        DisplayCommad::ShowTime(value)
    }
}

#[embassy_executor::task]
async fn display_task(mut display: Display) {
    loop {
        match display.channel.receive().await {
            DisplayCommad::ShowTime(time) => {
                let local = time.with_timezone(&display.config.timezone());
                println!("Received time: {local}");

                if let Err(err) = display
                    .write_time(local.hour() as _, local.minute() as _)
                    .await
                {
                    println!("Writing time to display failed: {err}");
                }
            }
            DisplayCommad::Clear => {
                if let Err(err) = display.clear().await {
                    println!("Clearing display failed: {err}");
                }
            }
            DisplayCommad::ShowText(text) => {
                if let Err(err) = display.write_str(text).await {
                    println!("Writing text to display failed: {err}");
                }
            }
        }
    }
}
