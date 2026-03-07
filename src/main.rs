#![no_std]
#![no_main]

use chrono::{NaiveTime, Timelike};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, rom::software_reset, system::reset_reason};
#[cfg(feature = "defmt")]
use esp_println as _;

use crate::{
    bsp::Board,
    config::Config,
    log_wrapper::{debug, error, info},
    ntp::NtpClient,
    system::{display::Display, motion_sensor::MotionSensor, time::WallClock},
    wifi::WifiInterface,
};

esp_bootloader_esp_idf::esp_app_desc!();

mod bsp;
mod config;
mod error;
mod log_wrapper;
mod ntp;
mod system;
mod utils;
mod wifi;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    if let Err(err) = fallible_main(spawner).await {
        error!("Main failed: {}", err)
    }

    Timer::after(Duration::from_secs(1)).await;
    software_reset()
}

async fn fallible_main(spawner: Spawner) -> Result<(), error::Error> {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger_from_env();

    if let Some(reason) = reset_reason() {
        info!("Last reset reason: {:?}", reason as u32)
    }

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let mut board = Board::init(peripherals).await?;
    let config = Config::get();
    let display = Display::init(board.display()).await?;
    display.set_intensity(0x01).await?;

    let mut wall_clock = WallClock::init(board.rtc(), config.timezone()).await;

    let adc_pin1 = board.take_adc_pin().ok_or(error::Error::other("Can not take ADC PIN1"))?;
    let s_enbl_pin = board
        .take_sensor_enable_pin()
        .ok_or(error::Error::other("Can not take sensor enable pin"))?;
    let motion_sensor = MotionSensor::new(board.adc(), adc_pin1, s_enbl_pin);
    let mut sensor_rx = motion_sensor
        .watch_receiver()
        .ok_or(error::Error::other("Can not get Motion Sensor receiver endpoint"))?;
    motion_sensor.launch(&spawner)?;

    let datetime = wall_clock.now_local().await;
    info!("Initial date time: {}", datetime);
    display.write_time(datetime.time()).await?;

    let wifi_interfaces = board.take_wifi_interfaces().ok_or(error::Error::other("No WiFi interface"))?;
    let wifi_controller = board
        .take_wifi_controller()
        .ok_or(error::Error::other("No WiFi controller"))?;
    let wifi = WifiInterface::init(&spawner, board.rng(), wifi_interfaces, wifi_controller, config).await?;

    let ntp_client = NtpClient::new(wifi.stack(), config.ntp_client(), board.rtc());
    ntp_client.launch(&spawner)?;

    let day_start = NaiveTime::from_hms_opt(6, 30, 0).ok_or(error::Error::other("Invalid night start time"))?;
    let day_end = NaiveTime::from_hms_opt(22, 0, 0).ok_or(error::Error::other("Invalid night end time"))?;
    let day_time = day_start..day_end;

    let mut timeout = Duration::from_secs(1);
    loop {
        let presence_detected = match select(Timer::after(timeout), sensor_rx.changed()).await {
            Either::First(_) => false,
            Either::Second(_) => {
                info!("Presence detected, displaying time");
                true
            }
        };

        let datetime = wall_clock.now_local().await;

        if !(presence_detected || day_time.contains(&datetime.time())) {
            if let Err(err) = display.clear().await {
                error!("Failed to clear display: {}", err);
            }
            continue;
        }

        debug!("Time to display: {}", datetime);

        match display.write_time(datetime.time()).await {
            Ok(()) if presence_detected => {
                timeout = Duration::from_secs(15);
            }
            Ok(()) => {
                timeout = Duration::from_secs(60 - datetime.second() as u64);
            }
            Err(err) => {
                error!("Failed to display time: {}", err);
                timeout = Duration::from_secs(10);
            }
        }
    }

    // Ok(())
}
