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

use crate::impl_from_variant;

const NTP_SERVER: &str = "pool.ntp.org";

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

#[derive(Debug)]
pub enum NtpError {
    SpawnError(SpawnError),
}
impl_from_variant!(NtpError, SpawnError, SpawnError);

pub fn init_ntp_task(spawner: &Spawner, stack: Stack<'static>) -> Result<(), NtpError> {
    spawner.spawn(ntp_task(stack))?;
    Ok(())
}

#[embassy_executor::task]
async fn ntp_task(stack: Stack<'static>) {
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
        .dns_query(NTP_SERVER, DnsQueryType::A)
        .await
        .expect("Failed to resolve DNS");
    if ntp_addrs.is_empty() {
        panic!("Failed to resolve DNS");
    }

    let mut ticker = Ticker::every(Duration::from_secs(5));
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
            Some(dt) => println!("Time: {}", dt),
            None => println!("Time: {:?}", time),
        }
    }
}
