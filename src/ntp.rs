use core::net::{IpAddr, SocketAddr};

use chrono::{DateTime, Utc};
use embassy_executor::{SpawnError, Spawner};
use embassy_net::{
    dns::DnsQueryType,
    udp::{PacketMetadata, UdpSocket},
    Stack,
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    watch::{self, Watch},
};
use embassy_time::{Duration, Ticker};
use esp_println::println;
use sntpc::{NtpContext, NtpTimestampGenerator};
use thiserror::Error;

use crate::config::{Config, NtpClientConfig};

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
    #[error("Other error: {0}")]
    Other(&'static str),
}

pub type NtpClientResult<T> = Result<T, NtpClientError>;

pub struct NtpClient {
    rx: watch::Receiver<'static, CriticalSectionRawMutex, DateTime<Utc>, 1>,
}

impl NtpClient {
    pub fn launch(
        spawner: &Spawner,
        stack: Stack<'static>,
        config: &'static Config,
    ) -> NtpClientResult<Self> {
        static WATCH: Watch<CriticalSectionRawMutex, DateTime<Utc>, 1> = Watch::new();
        let rx = WATCH.receiver().ok_or(NtpClientError::Other(
            "Can not get receiver end of watch channel",
        ))?;

        let me = Self { rx };
        spawner.spawn(ntp_task(stack, WATCH.sender(), config.ntp_client()))?;

        Ok(me)
    }

    pub async fn changed(&mut self) -> DateTime<Utc> {
        self.rx.changed().await
    }
}

#[embassy_executor::task]
async fn ntp_task(
    stack: Stack<'static>,
    tx: watch::Sender<'static, CriticalSectionRawMutex, DateTime<Utc>, 1>,
    config: &'static NtpClientConfig,
) {
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    socket.bind(123).unwrap();

    let context = NtpContext::new(Timestamp::default());

    let ntp_addrs = stack
        .dns_query(config.server(), DnsQueryType::A)
        .await
        .expect("Failed to resolve DNS");
    if ntp_addrs.is_empty() {
        panic!("Failed to resolve DNS");
    }

    let mut ticker = Ticker::every(config.query_period());
    ticker.reset_after(Duration::MIN);
    loop {
        ticker.next().await;

        let addr: IpAddr = ntp_addrs[0].into();
        let addr = SocketAddr::from((addr, 123));

        let result = sntpc::get_time(addr, &socket, context).await;
        let time = match result {
            Ok(time) => time,
            Err(err) => {
                println!("Error getting time: {err:?}");
                continue;
            }
        };

        match DateTime::from_timestamp(time.seconds as _, 0) {
            Some(dt) => tx.send(dt),
            None => println!("Time: {:?}", time),
        }
    }
}
