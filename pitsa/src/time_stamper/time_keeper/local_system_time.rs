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

//! Local system time and offset with accuracy measurements.

use super::TimeKeeper;
use sntpc::NtpResult;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/** Local system time and offset with accuracy measurements.

By measuring how to local system time drifts compared to known NTP server
measurements, a rough estimate of the local clocks acurracy can be made.
*/
pub struct LocalSystemTime {
    declared_accuracy_micros: u64,
    worst_measured_accuracy_micros: AtomicU64,
    max_drift_between_checks_micros: AtomicU64,
    last_offset: AtomicI64,
}

impl LocalSystemTime {
    /// Return a new instance with a declared worst case acurracy of the local
    /// system time when no reliable NTP measurements can be made.
    pub fn new(declared_accuracy_micros: u64) -> Arc<Self> {
        Arc::new(Self {
            declared_accuracy_micros,
            worst_measured_accuracy_micros: AtomicU64::new(0),
            max_drift_between_checks_micros: AtomicU64::new(0),
            last_offset: AtomicI64::new(0),
        })
    }

    /// If a NTP update is missing we assume that local clock has drifted by
    /// the worst measured drift between updates.
    ///
    /// Once the measured accuracy reaches the declared accuracy, we reset the
    /// measurements to require an NTP update and start reporting the declared
    /// acurracy.
    pub fn update_delta_without_ntp_time(&self) {
        let max_drift = self.max_drift_between_checks_micros.load(Ordering::Relaxed);
        if max_drift != 0 {
            let previous = self
                .worst_measured_accuracy_micros
                .fetch_add(max_drift, Ordering::Relaxed);
            if previous + max_drift >= self.declared_accuracy_micros {
                // Face it.. we don't really know anymore.
                log::warn!("This instance is now operating with local system time accuracy.");
                self.worst_measured_accuracy_micros
                    .store(0, Ordering::Relaxed);
                self.max_drift_between_checks_micros
                    .store(0, Ordering::Relaxed);
                self.last_offset.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Measure local clock drift and maintain a worst estimated accuracy.
    pub fn update_delta_from_ntp_time(&self, ntp_time: &NtpResult) {
        let last_offset = self.last_offset.load(Ordering::Relaxed);
        let mut offset = ntp_time.offset();
        let accuracy = i64::try_from(
            TimeKeeper::get_precision_micros_from_ntp_time(ntp_time) + ntp_time.roundtrip(),
        )
        .unwrap();
        if offset < 0 {
            offset -= accuracy;
        } else {
            offset += accuracy;
        }
        if last_offset != 0 {
            let diff = last_offset.abs_diff(offset);
            let previous_diff = self.max_drift_between_checks_micros.load(Ordering::Relaxed);
            if previous_diff < diff {
                self.max_drift_between_checks_micros
                    .store(diff, Ordering::Relaxed);
            }
            let worst_measured_accuracy_micros =
                self.worst_measured_accuracy_micros.load(Ordering::Relaxed);
            if worst_measured_accuracy_micros == 0 {
                self.worst_measured_accuracy_micros.store(
                    std::cmp::max(last_offset.abs_diff(0), offset.abs_diff(0)),
                    Ordering::Relaxed,
                );
                log::info!(
                    "Initial measurement of local system clock accuracy is {} µs.",
                    offset.abs_diff(0)
                );
            } else if last_offset.abs() < offset.abs()
                && worst_measured_accuracy_micros < offset.abs_diff(0)
            {
                self.worst_measured_accuracy_micros
                    .store(offset.abs_diff(0), Ordering::Relaxed);
                log::info!(
                    "Worst measurement of local system clock accuracy is now {} µs.",
                    offset.abs_diff(0)
                );
            }
        }
        self.last_offset.store(offset, Ordering::Relaxed);
    }

    /// Return the time as the "system time" + "last known NTP offset" and
    /// estimated accuracy (capped by declared accuracy).
    pub fn get_epoch_time_with_accuracy_micros(&self) -> Option<(u64, u64)> {
        let system_time_micros = u64::try_from(
            i64::try_from(upkit_common::util::time::now_epoch_micros()).unwrap()
                + self.last_offset.load(Ordering::Relaxed),
        )
        .unwrap();
        let accuracy_micros = std::cmp::min(
            self.worst_measured_accuracy_micros.load(Ordering::Relaxed),
            self.declared_accuracy_micros,
        );
        if accuracy_micros == 0 {
            // No measurement yet. Use declared acurracy.
            return Some((system_time_micros, self.declared_accuracy_micros));
        }
        Some((system_time_micros, accuracy_micros))
    }
}
