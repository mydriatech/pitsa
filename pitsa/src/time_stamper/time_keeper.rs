/*
    Copyright 2025 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Time with accuracy measurements.

mod local_system_time;
mod ntp_client;

use self::local_system_time::LocalSystemTime;
use self::ntp_client::NtpClient;
use sntpc::NtpResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/* Keeper of current time with accuracy measurements.

...a.k.a. guardian of space time.

## Requirements in short

* Continiously ensure that time is in sync with UTC (RFC 3628 7.3.2)
    * Detect jumps and drifts
    * Handle leap-seconds
* Ensure an accuracy of at most half a second
    * Other signed X.509 structures usually defines time with second granularity.
    * "The declared accuracy shall be of 1 second or better." (ETSI EN 319 421 7.7.2.b)
    * "the accuracy field shall be present and a minimum accuracy of one second shall be supported;" (ETSI EN 319 422 5.2.2 )
* Reject requests if time is not in sync

## Fulfilling requrements

This service assumes that the NTP-service can be trusted and that there are no
MITM or tampering with the requests and responses from the NTP service.

When this service is configured to get a fresh NTP response which means that as
long as the NTP service handles time correctly, so will this service.
Offset to the local system time is still tracked, but simply for statistics.

When a period sync to the NTP service is configured the drift between each call
compared to local system time will tracked to ensure that the reported accuracy
between syncs has a sufficiently large margin to be correct.

When running without an explicit NTP service, the local systems is assumed to be
trusted within the declared accuracy.

## References:

* [RFC 3628](https://www.rfc-editor.org/rfc/rfc3628) Policy Requirements for Time-Stamping Authorities (TSAs).
* [ETSI EN 319 421](https://www.etsi.org/deliver/etsi_en/319400_319499/319421/01.01.01_60/en_319421v010101p.pdf) Policy and Security Requirements for Trust Service Providers issuing Time-Stamps
* [ETSI EN 319 422](https://www.etsi.org/deliver/etsi_en/319400_319499/319422/01.01.01_60/en_319422v010101p.pdf) Time-stamping protocol and time-stamp token profiles
*/

/// Guardian of space and time.
pub struct TimeKeeper {
    ntp_host: Option<String>,
    tolerable_accuracy_micros: u64,
    ntp_query_for_every_request: bool,
    local_system_time: Arc<LocalSystemTime>,
    ntp_client: Option<Arc<NtpClient>>,
    within_tolerance: AtomicBool,
}

impl TimeKeeper {
    /// Return a new instance
    pub async fn new(
        ntp_host: Option<String>,
        ntp_timeout_micros: u64,
        system_time_accuracy_micros: u64,
        ntp_sync_interval_micros: u64,
        tolerable_accuracy_micros: u64,
        ntp_query_for_every_request: bool,
    ) -> Arc<Self> {
        let ntp_client = if let Some(ntp_host) = &ntp_host {
            log::info!("TimeKeeper started with NTP host '{ntp_host}'.");
            Some(NtpClient::new(ntp_host.as_str(), ntp_timeout_micros).await)
        } else {
            log::info!("TimeKeeper started without any NTP host.");
            None
        };
        Arc::new(Self {
            ntp_host,
            tolerable_accuracy_micros,
            ntp_query_for_every_request: ntp_client.is_some() && ntp_query_for_every_request,
            local_system_time: LocalSystemTime::new(system_time_accuracy_micros),
            ntp_client,
            within_tolerance: AtomicBool::new(false),
        })
        .init(ntp_sync_interval_micros)
        .await
    }

    /// Return true if measured accuracy is within the tolerable limit.
    pub fn is_within_tolerance(self: &Arc<Self>) -> bool {
        self.within_tolerance.load(Ordering::Relaxed)
    }

    /// Initialize background tasks like periodic time sync.
    async fn init(self: Arc<Self>, ntp_sync_interval_micros: u64) -> Arc<Self> {
        if self.ntp_client.is_some() {
            let self_clone = Arc::clone(&self);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_micros(
                        ntp_sync_interval_micros,
                    ))
                    .await;
                    let self_clone = Arc::clone(&self_clone);
                    tokio::spawn(async move { self_clone.update_local_time_diff_from_ntp().await });
                }
            });
        }
        self
    }

    /// Update tracking of [LocalSystemTime] with NTP response.
    async fn update_local_time_diff_from_ntp(self: &Arc<Self>) {
        if let Some(ntp_client) = self.ntp_client.as_ref() {
            if let Some(ntp_time) = ntp_client.request_ntp_time().await {
                self.local_system_time.update_delta_from_ntp_time(&ntp_time);
                log::info!(
                    "NTP server '{}' status: stratum: {}, offset: {} µs, roundtrip: {} µs, precision: 2^{} s ({} µs).",
                    self.ntp_host.as_ref().unwrap(),
                    ntp_time.stratum(),
                    ntp_time.offset(),
                    ntp_time.roundtrip(),
                    ntp_time.precision(),
                    Self::get_precision_micros_from_ntp_time(&ntp_time),
                );
            } else {
                self.local_system_time.update_delta_without_ntp_time();
            }
        }
        // Flag health check if we don't have a sufficiently accurate time.
        self.within_tolerance.store(
            self.get_epoch_time_with_accuracy_micros().await.is_some(),
            Ordering::Relaxed,
        );
    }

    /// Return the current time in microseconds with accuracy measurement.
    pub async fn get_epoch_time_with_accuracy_micros(&self) -> Option<(u64, u64)> {
        let mut res = None;
        if self.ntp_query_for_every_request {
            if let Some(ntp_time) = self.ntp_client.as_ref().unwrap().request_ntp_time().await {
                // https://datatracker.ietf.org/doc/html/rfc4330#section-3 NTPv4 Timestamp Format
                let epoch_micros = Self::get_epoch_micros_from_ntp_time(&ntp_time);
                let precision_micros = Self::get_precision_micros_from_ntp_time(&ntp_time);
                // Accuracy is also affected by the round trip time
                // This must however have been measured by the local time source and is not really reliable
                // Adding the full round trip time is probably not a bad idea to avoid lying about the time.
                let accuracy_micros = precision_micros + ntp_time.roundtrip();
                res = Some((epoch_micros, accuracy_micros));
            }
        }
        if res.is_none() {
            res = self.local_system_time.get_epoch_time_with_accuracy_micros();
        }
        let res = res.filter(|(_epoch_micros, accuracy_micros)| {
            accuracy_micros <= &self.tolerable_accuracy_micros
        });
        // Set last failure for healthcheck here if accurracy was too low
        self.within_tolerance
            .store(res.is_some(), Ordering::Relaxed);
        res
    }

    /// Aggregate time from ntp result into microseconds.
    fn get_epoch_micros_from_ntp_time(ntp_time: &NtpResult) -> u64 {
        /*
        The life-time extension beyond 2036 outlined in https://datatracker.ietf.org/doc/html/rfc4330#section-3
        is already handled by this library in
        https://github.com/vpetrigo/sntpc/blob/283bcc25a64d4083da60b79bde919d70308f5c29/sntpc/src/types.rs#L129
        */
        u64::from(ntp_time.seconds) * 1_000_000
            + u64::from(sntpc::fraction_to_microseconds(ntp_time.sec_fraction()))
    }

    /// Convert NTP precision from "power of 2 seconds" to microseconds.
    fn get_precision_micros_from_ntp_time(ntp_time: &NtpResult) -> u64 {
        (2f64.powi(i32::from(ntp_time.precision())) * 1_000_000f64).round() as u64
    }
}
