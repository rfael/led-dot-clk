use core::net::{IpAddr, SocketAddr};

use chrono::DateTime;
use embassy_executor::{SpawnError, Spawner};
use embassy_net::{
    dns::DnsQueryType,
    udp::{PacketMetadata, UdpSocket},
    Stack,
};
use embassy_time::{Duration, Ticker};
use esp_println::println;
use sntpc::{NtpContext, NtpTimestampGenerator};
use thiserror::Error;

use crate::{
    bsp::{RtcDevice, SharedDevice},
    config::NtpClientConfig,
};

#[derive(Copy, Clone, Default)]
struct Timestamp {
    inner: core::time::Duration,
}

impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        self.inner.as_secs()
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.inner.subsec_micros()
    }
}

#[derive(Debug, Error)]
pub enum NtpClientError {
    #[error("Spawnn error: {0}")]
    SpawnError(#[from] SpawnError),
}

pub type NtpClientResult<T> = Result<T, NtpClientError>;

pub struct NtpClient {
    stack: Stack<'static>,
    config: &'static NtpClientConfig,
    rtc: &'static SharedDevice<RtcDevice>,
}

impl NtpClient {
    pub fn new(
        stack: Stack<'static>,
        config: &'static NtpClientConfig,
        rtc: &'static SharedDevice<RtcDevice>,
    ) -> Self {
        Self { stack, config, rtc }
    }

    pub fn launch(self, spawner: &Spawner) -> NtpClientResult<()> {
        spawner.spawn(ntp_task(self))?;
        Ok(())
    }
}

#[embassy_executor::task]
async fn ntp_task(client: NtpClient) {
    const RETRY_TIMEOUT: Duration = Duration::from_secs(10);

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut socket = UdpSocket::new(
        client.stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(123).unwrap();

    let context = NtpContext::new(Timestamp::default());

    let ntp_addrs = client
        .stack
        .dns_query(client.config.server(), DnsQueryType::A)
        .await
        .expect("Failed to resolve DNS");
    if ntp_addrs.is_empty() {
        panic!("Failed to resolve DNS");
    }

    let mut ticker = Ticker::every(RETRY_TIMEOUT);
    loop {
        ticker.next().await;

        let addr: IpAddr = ntp_addrs[0].into();
        let addr = SocketAddr::from((addr, 123));

        println!("NTP server query...");
        let result = sntpc::get_time(addr, &socket, context).await;
        let time = match result {
            Ok(time) => {
                ticker = Ticker::every(client.config.query_period());
                time
            }
            Err(err) => {
                println!("Error getting time: {err:?}");
                ticker = Ticker::every(RETRY_TIMEOUT);
                continue;
            }
        };

        let Some(time) = DateTime::from_timestamp(time.seconds as _, 0) else {
            println!("Failed to convert NTP response to DateTime");
            ticker = Ticker::every(RETRY_TIMEOUT);
            continue;
        };
        println!("Time received from NTP server: {time}");

        if let Err(err) = client
            .rtc
            .lock()
            .await
            .set_datetime(&time.naive_utc())
            .await
        {
            println!("Updating time in RTC failed: {err:?}");
            ticker = Ticker::every(RETRY_TIMEOUT);
            continue;
        }
    }
}
