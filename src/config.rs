use chrono::FixedOffset;
use embassy_time::Duration;

use crate::mk_static;

pub struct Config {
    wifi: WiFiConfig,
    ntp_client: NtpClientConfig,
    timezone: FixedOffset,
}

impl Config {
    pub fn get() -> &'static Self {
        let me = Self {
            wifi: Default::default(),
            ntp_client: Default::default(),
            timezone: FixedOffset::east_opt(2 * 3600).unwrap(),
        };
        mk_static!(Config, me)
    }

    pub fn wifi(&self) -> &WiFiConfig {
        &self.wifi
    }

    pub fn ntp_client(&self) -> &NtpClientConfig {
        &self.ntp_client
    }

    pub fn timezone(&self) -> FixedOffset {
        self.timezone
    }
}

pub struct WiFiConfig {
    ssid: &'static str,
    password: &'static str,
    reconnect_timeout: Duration,
}

impl WiFiConfig {
    pub fn ssid(&self) -> &'static str {
        self.ssid
    }

    pub fn password(&self) -> &'static str {
        self.password
    }

    pub fn reconnect_timeout(&self) -> Duration {
        self.reconnect_timeout
    }
}

impl Default for WiFiConfig {
    fn default() -> Self {
        Self {
            ssid: env!("SSID"),
            password: env!("PASSWORD"),
            reconnect_timeout: Duration::from_secs(5),
        }
    }
}

pub struct NtpClientConfig {
    server: &'static str,
    query_period: Duration,
}

impl NtpClientConfig {
    pub fn server(&self) -> &'static str {
        self.server
    }

    pub fn query_period(&self) -> Duration {
        self.query_period
    }
}

impl Default for NtpClientConfig {
    fn default() -> Self {
        Self {
            server: "pool.ntp.org",
            query_period: Duration::from_secs(60 * 5),
        }
    }
}
