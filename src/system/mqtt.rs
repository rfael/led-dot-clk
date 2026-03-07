use core::net::Ipv4Addr;

use embassy_executor::{SpawnError, Spawner};
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_time::{Duration, Ticker};
use rust_mqtt::{
    buffer::{BufferProvider, BumpBuffer},
    client::{Client, options::ConnectOptions},
    config::{ClientConfig, KeepAlive, SessionExpiryInterval},
};
use thiserror::Error;

use crate::log_wrapper::{debug, error, info};

#[derive(Debug, Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MqttClientError {
    #[error("Spawnn error: {0}")]
    SpawnError(#[from] SpawnError),
}

pub type MqttClientResult<T> = Result<T, MqttClientError>;

pub struct MqttClient {
    stack: Stack<'static>,
}

impl MqttClient {
    pub fn new(stack: Stack<'static>) -> Self {
        Self { stack }
    }

    pub fn launch(self, spawner: &Spawner) -> MqttClientResult<()> {
        spawner.spawn(mqtt_task(self))?;
        Ok(())
    }
}

#[embassy_executor::task]
async fn mqtt_task(client: MqttClient) {
    const RETRY_TIMEOUT: Duration = Duration::from_secs(10);

    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];
    let opts = ConnectOptions {
        clean_start: true,
        keep_alive: KeepAlive::Seconds(5),
        session_expiry_interval: SessionExpiryInterval::Seconds(60),
        user_name: None,
        password: None,
        will: None,
    };
    let cfg = ClientConfig {
        session_expiry_interval: SessionExpiryInterval::Seconds(30),
    };

    let mut ticker = Ticker::every(RETRY_TIMEOUT);
    loop {
        ticker.next().await;

        let mut socket = TcpSocket::new(client.stack, &mut rx_buf, &mut tx_buf);
        match socket.connect((Ipv4Addr::new(192, 168, 1, 11), 1883)).await {
            Ok(_) => info!("Connected to broker"),
            Err(err) => {
                error!("Failed to connect to host: {}", err);
                continue;
            }
        }

        let mut client_buf = [0u8; 1024];
        let mut buffer = BumpBuffer::new(&mut client_buf);
        let mut mqtt_client: Client<'_, _, _, 1, 1, 1> = Client::new(&mut buffer);

        match mqtt_client.connect(socket, &opts, None).await {
            Ok(c) => {
                info!("Connected to server: {:?}", c);
            }
            Err(err) => {
                error!("Failed to connect to broker: {}", err);
                continue;
            }
        };
    }
}
