use crate::{bsp::BoardError, impl_from_variant, ntp::NtpClientError, wifi::WifiError};

#[derive(Debug)]
pub enum Error {
    Board(BoardError),
    WiFi(WifiError),
    NtpClient(NtpClientError),
    Other(&'static str),
}
impl_from_variant!(Error, Board, BoardError);
impl_from_variant!(Error, WiFi, WifiError);
impl_from_variant!(Error, NtpClient, NtpClientError);

impl Error {
    pub fn other(err: &'static str) -> Self {
        Self::Other(err)
    }
}
