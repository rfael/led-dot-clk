#![no_std]
#![no_main]

use chrono::{DateTime, TimeDelta, Timelike};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_println::println;

use crate::{
    bsp::Board,
    config::Config,
    display::{Display, DisplayCommad},
    ntp::NtpClient,
    wifi::WifiInterface,
};

esp_bootloader_esp_idf::esp_app_desc!();

mod bsp;
mod config;
mod display;
mod error;
mod max7219_led_matrix;
mod ntp;
mod utils;
mod wifi;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    fallible_main(spawner)
        .await
        .inspect_err(|err| println!("Main failed: {err}"))
        .unwrap();

    // let sclk = peripherals.GPIO0;
    // let miso = peripherals.GPIO2;
    // let mosi = peripherals.GPIO4;
    // let cs = peripherals.GPIO5;
    // let dma_channel = peripherals.DMA_CH0;
    //
    // let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32000);
    // let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    // let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();
    //
    // let mut spi = Spi::new(
    //     peripherals.SPI2,
    //     Config::default()
    //         .with_frequency(Rate::from_khz(100))
    //         .with_mode(Mode::_0),
    // )
    // .unwrap()
    // .with_sck(sclk)
    // .with_mosi(mosi)
    // .with_miso(miso)
    // // .with_cs(cs)
    // .with_dma(dma_channel)
    // .with_buffers(dma_rx_buf, dma_tx_buf);
    // // .into_async();
    //
    // let delay = Delay::new();
    // let bus = ExclusiveDevice::new(spi, cs, delay);
    // let display = max7219_async::Max7219::new(bus);

    loop {
        Timer::after(Duration::from_millis(1_000)).await;
        println!("tick...");
    }
}

async fn fallible_main(spawner: Spawner) -> Result<(), error::Error> {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let mut board = Board::init(peripherals)?;
    let config = Config::get();

    let max7219 = board
        .take_max7219()
        .ok_or(error::Error::other("No display device"))?;
    let mut display = Display::init(max7219, config).await?;
    println!("Display initialized");
    display.set_intensity(0x01).await?;
    let display_tx = display.launch(&spawner).await?;
    display_tx.send(DisplayCommad::Clear).await;
    display_tx.send(DisplayCommad::ShowText("elo!")).await;

    let wifi_interfaces = board
        .take_wifi_interfaces()
        .ok_or(error::Error::other("No WiFi interface"))?;
    let wifi_controller = board
        .take_wifi_controller()
        .ok_or(error::Error::other("No WiFi controller"))?;
    let wifi = WifiInterface::init(
        &spawner,
        board.rng(),
        wifi_interfaces,
        wifi_controller,
        config,
    )
    .await?;

    let mut ntp_client = NtpClient::launch(&spawner, wifi.stack(), config)?;

    let mut time = DateTime::from_timestamp(0, 0).unwrap_or_default();
    let mut timeout = Duration::from_secs(60);
    loop {
        match select(ntp_client.changed(), Timer::after(timeout)).await {
            Either::First(t) => {
                time = t;
                println!("NTP time update: {time}");
                display_tx.send(time.into()).await;
                timeout = Duration::from_secs(60 - time.second() as u64);
            }
            Either::Second(_) => {
                timeout = Duration::from_secs(60);
                time += TimeDelta::minutes(1);
                display_tx.send(time.into()).await;
            }
        }
    }

    // Ok(())
}
