#![no_std]
#![no_main]

use chrono::Timelike;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;

use crate::{bsp::Board, config::Config, ntp::NtpClient, wifi::WifiInterface};

esp_bootloader_esp_idf::esp_app_desc!();

mod bsp;
mod config;
mod error;
mod ntp;
mod system;
mod utils;
mod wifi;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    fallible_main(spawner)
        .await
        .inspect_err(|err| log::error!("Main failed: {err}"))
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
        log::debug!("tick...");
    }
}

async fn fallible_main(spawner: Spawner) -> Result<(), error::Error> {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let mut board = Board::init(peripherals).await?;
    let config = Config::get();

    {
        let datetime = board
            .rtc()
            .lock()
            .await
            .datetime()
            .await
            .map_err(|_| error::Error::Other("Failed to get initial RTC time"))?
            .and_utc();
        let datetime = datetime.with_timezone(&config.timezone());
        log::info!("Initial RTC time: {datetime}");
        board
            .display()
            .lock()
            .await
            .write_time(datetime.time())
            .await
            .map_err(|_| error::Error::Other("Failed to display initial time"))?;
    }

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

    let ntp_client = NtpClient::new(wifi.stack(), config.ntp_client(), board.rtc());
    ntp_client.launch(&spawner)?;

    const RETRY_TIMEOUT: Duration = Duration::from_secs(10);

    let mut timeout = Duration::from_secs(0);
    loop {
        Timer::after(timeout).await;
        let datetime = match board.rtc().lock().await.datetime().await {
            Ok(dt) => dt.and_utc().with_timezone(&config.timezone()),
            Err(err) => {
                log::error!("Failed to fetch time from RTC: {err:?}");
                timeout = RETRY_TIMEOUT;
                continue;
            }
        };

        log::info!("Time to display: {datetime}");

        match board
            .display()
            .lock()
            .await
            .write_time(datetime.time())
            .await
        {
            Ok(()) => {
                timeout = Duration::from_secs(60 - datetime.second() as u64);
            }
            Err(err) => {
                log::error!("Failed to display time: {err}");
                timeout = RETRY_TIMEOUT;
            }
        }
    }

    // Ok(())
}
