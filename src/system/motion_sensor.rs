use circular_buffer::CircularBuffer;
use embassy_executor::{SpawnError, Spawner};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    watch::{Receiver, Watch},
};
use embassy_time::{Duration, Ticker, Timer};
use esp_hal::gpio::Output;
use thiserror::Error;

use crate::{
    bsp::{AdcDev, AdcPin1, SharedDevice},
    mk_static,
};

#[derive(Debug, Error)]
pub enum MotionSensorError {
    #[error("Spawnn error: {0}")]
    SpawnError(#[from] SpawnError),
}
pub type MotionSensorResult<T> = Result<T, MotionSensorError>;

pub struct MotionSensor {
    adc: &'static SharedDevice<AdcDev>,
    adc_pin: AdcPin1,
    sensor_enable_pin: Output<'static>,
    watch: &'static Watch<CriticalSectionRawMutex, (), 1>,
}

impl MotionSensor {
    pub fn new(adc: &'static SharedDevice<AdcDev>, adc_pin: AdcPin1, sensor_enable_pin: Output<'static>) -> Self {
        let watch = mk_static!(Watch<CriticalSectionRawMutex, (), 1>, Watch::new());
        Self {
            adc,
            adc_pin,
            sensor_enable_pin,
            watch,
        }
    }

    pub fn launch(self, spawner: &Spawner) -> MotionSensorResult<()> {
        spawner.spawn(motion_sensor_task(self))?;
        Ok(())
    }

    async fn worker_loop(mut self) -> ! {
        let mut ticker = Ticker::every(Duration::from_millis(100));
        let mut samples: CircularBuffer<64, u16> = CircularBuffer::new();
        loop {
            ticker.next().await;
            self.sensor_enable_pin.set_high();
            let sample = self.adc.lock().await.read_oneshot(&mut self.adc_pin).await;
            self.sensor_enable_pin.set_low();
            // log::debug!("ADC1 = {sample}");

            samples.push_back(sample);
            if samples.len() < 32 {
                continue;
            }

            let average = samples.iter().fold(0u32, |s, v| s + *v as u32) / samples.len() as u32;
            let limit = average as u16 + 100;
            let presence_detected = samples.iter().rev().take(4).all(|v| *v > limit);
            if !presence_detected {
                continue;
            }

            self.watch.sender().send(());
            Timer::after(Duration::from_secs(10)).await;
            samples.clear();
        }
    }

    pub fn watch_receiver(&self) -> Option<Receiver<'static, CriticalSectionRawMutex, (), 1>> {
        self.watch.receiver()
    }
}

#[embassy_executor::task]
async fn motion_sensor_task(ms: MotionSensor) {
    ms.worker_loop().await
}
