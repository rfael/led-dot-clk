use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeDelta, Utc};
use embassy_time::Instant;

use crate::{
    bsp::{RtcDevice, SharedDevice},
    log_wrapper::{debug, error},
};

pub struct WallClock {
    rtc: &'static SharedDevice<RtcDevice>,
    last_known_time: DateTime<Utc>,
    last_time_check: Instant,
    timezone: FixedOffset,
}

impl WallClock {
    pub async fn init(rtc: &'static SharedDevice<RtcDevice>, timezone: FixedOffset) -> Self {
        let now = match rtc.lock().await.datetime().await {
            Ok(t) => {
                debug!("Time read from RTC: {} UTC", t);
                t
            }
            Err(_err) => {
                #[cfg(feature = "log")]
                error!("Reading time from RTC failed: {:?}", _err);
                #[cfg(feature = "defmt")]
                error!("Reading time from RTC failed");
                NaiveDateTime::MIN
            }
        }
        .and_utc();

        Self {
            rtc,
            last_known_time: now,
            last_time_check: Instant::now(),
            timezone,
        }
    }

    pub async fn now_utc(&mut self) -> DateTime<Utc> {
        self.last_known_time = match self.rtc.lock().await.datetime().await {
            Ok(t) => {
                debug!("Time read from RTC: {} UTC", t);
                t.and_utc()
            }
            Err(_err) => {
                #[cfg(feature = "log")]
                error!("Reading time from RTC failed: {:?}", _err);
                #[cfg(feature = "defmt")]
                error!("Reading time from RTC failed");
                self.last_known_time + TimeDelta::microseconds(self.last_time_check.as_micros() as _)
            }
        };

        self.last_time_check = Instant::now();

        self.last_known_time
    }

    pub async fn now_local(&mut self) -> DateTime<FixedOffset> {
        self.now_utc().await.with_timezone(&self.timezone)
    }
}
