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

//! Parsing of configuration for the time source.

use config::builder::BuilderState;
use config::ConfigBuilder;
use serde::{Deserialize, Serialize};

use super::AppConfigDefaults;

/// Configuration for the time source.
#[derive(Debug, Deserialize, Serialize)]
pub struct TimeSourceConfig {
    /// See [ntp_host()](Self::ntp_host()).
    ntphost: Option<String>,
    /// See [ntp_timeout_micros()](Self::ntp_timeout_micros()).
    timeout: u64,
    /// See [system_time_accuracy_micros()](Self::system_time_accuracy_micros()).
    accuracy: u64,
    /// See [ntp_sync_interval_micros()](Self::ntp_sync_interval_micros()).
    interval: u64,
    /// See [tolerable_accuracy_micros()](Self::tolerable_accuracy_micros()).
    tolerance: u64,
    /// See [ntp_query_for_every_request()](Self::ntp_query_for_every_request()).
    always: bool,
}

impl AppConfigDefaults for TimeSourceConfig {
    /// Provide defaults for this part of the configuration
    fn set_defaults<T: BuilderState>(
        config_builder: ConfigBuilder<T>,
        prefix: &str,
    ) -> ConfigBuilder<T> {
        config_builder
            .set_default(prefix.to_string() + "." + "ntphost", "")
            .unwrap()
            .set_default(prefix.to_string() + "." + "timeout", "250000")
            .unwrap()
            .set_default(prefix.to_string() + "." + "accuracy", "30000000")
            .unwrap()
            .set_default(prefix.to_string() + "." + "interval", "15000000")
            .unwrap()
            .set_default(prefix.to_string() + "." + "tolerance", "500000")
            .unwrap()
            .set_default(prefix.to_string() + "." + "always", "false")
            .unwrap()
    }
}

impl TimeSourceConfig {
    /// NTP host in the form `hostname:port`. An empty string will disable NTP.
    pub fn ntp_host(&self) -> Option<String> {
        if self
            .ntphost
            .as_ref()
            .is_none_or(|ntp_host| ntp_host.is_empty())
        {
            return None;
        }
        self.ntphost.clone()
    }

    /// How long to wait for an NTP response before considering it lost.
    pub fn ntp_timeout_micros(&self) -> u64 {
        self.timeout
    }

    /// Estimated worst case accuracy of the local system time.
    pub fn system_time_accuracy_micros(&self) -> u64 {
        self.accuracy
    }

    /// How often to compare the local time with the NTP time source.
    pub fn ntp_sync_interval_micros(&self) -> u64 {
        self.interval
    }

    /// The worst time source accuracy that can be tolerated.
    pub fn tolerable_accuracy_micros(&self) -> u64 {
        self.tolerance
    }

    /// Query the NTP server for every time stamp request.
    pub fn ntp_query_for_every_request(&self) -> bool {
        self.always
    }
}
