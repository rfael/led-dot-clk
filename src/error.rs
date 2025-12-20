use crate::{
    bsp::BoardError,
    ntp::NtpClientError,
    system::{display::DisplayError, motion_sensor::MotionSensorError},
    wifi::WifiError,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Board error: {0}")]
    Board(#[from] BoardError),
    #[error("WiFi error: {0}")]
    WiFi(#[from] WifiError),
    #[error("NTP error: {0}")]
    NtpClient(#[from] NtpClientError),
    #[error("Display error: {0}")]
    Display(#[from] DisplayError),
    #[error("Motion Sensor error: {0}")]
    MotionSensor(#[from] MotionSensorError),
    #[error("Other error: {0}")]
    Other(&'static str),
}

impl Error {
    pub fn other(err: &'static str) -> Self {
        Self::Other(err)
    }
}
