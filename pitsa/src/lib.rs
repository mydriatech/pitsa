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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

//! Library entry point

pub mod conf;
pub mod rest_api;
mod time_stamper;

use conf::AppConfig;
use std::process::ExitCode;
use std::sync::Arc;
pub use time_stamper::TimeStamper;
use tokio::signal::unix::{SignalKind, signal};
use tyst_api_rest_health::AppHealth;

/// Async code entry point.
pub async fn run_async(app_config: Arc<AppConfig>) -> ExitCode {
    let app_future = run_async_abortable_with_logging(&app_config);
    let signals_future = block_until_signaled();
    tokio::select! {
        _ = app_future => {
            log::trace!("app_future finished");
        },
        _ = signals_future => {
            log::trace!("signals_future finished");
        },
    };
    ExitCode::SUCCESS
}

/// Block until SIGTERM or SIGINT is recieved.
async fn block_until_signaled() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {
            log::debug!("SIGTERM recieved.")
        },
        _ = sigint.recv() => {
            log::debug!("SIGINT recieved.")
        },
    };
}

/// Simple health check that gets the provider instance.
pub struct SimpleHealth {
    time_stamper: Arc<TimeStamper>,
}
impl SimpleHealth {
    fn with_app(time_stamper: &Arc<TimeStamper>) -> Arc<dyn AppHealth> {
        Arc::new(Self {
            time_stamper: Arc::clone(time_stamper),
        })
    }
}
impl AppHealth for SimpleHealth {
    fn is_health_started(&self) -> bool {
        self.is_health_ready()
    }
    fn is_health_ready(&self) -> bool {
        self.time_stamper.is_ready()
    }
    fn is_health_live(&self) -> bool {
        self.is_health_ready()
    }
}

async fn run_async_abortable_with_logging(app_config: &Arc<AppConfig>) {
    let app = TimeStamper::new(&Arc::clone(app_config)).await;
    let app_health: Arc<dyn AppHealth> = SimpleHealth::with_app(&app);
    rest_api::run_http_server(
        app_config.limits.available_parallelism(),
        &app_config.api.bind_address(),
        app_config.api.bind_port(),
        &app_health,
        &app,
    )
    .await
    .unwrap();
}
