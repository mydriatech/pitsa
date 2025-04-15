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

use lib::conf::AppConfig;
use pitsa as lib;
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    if let Err(e) = init_logger() {
        println!("Failed to initialize logging: {e:?}");
        return ExitCode::FAILURE;
    }
    let app_config = Arc::new(AppConfig::new(env!("CARGO_BIN_NAME")));
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(app_config.limits.available_parallelism())
        .build()
        .unwrap()
        .block_on(lib::run_async(app_config))
}

/// Initialize the logging system and apply filters.
fn init_logger() -> Result<(), log::SetLoggerError> {
    env_logger::builder()
        // Set default log level
        .filter_level(log::LevelFilter::Debug)
        // Customize logging for dependencies
        .filter(Some("actix_http::h1"), log::LevelFilter::Debug)
        .filter(Some("mio::poll"), log::LevelFilter::Debug)
        .filter(Some("h2"), log::LevelFilter::Info)
        .filter(Some("actix_server"), log::LevelFilter::Warn)
        .write_style(env_logger::fmt::WriteStyle::Never)
        .target(env_logger::fmt::Target::Stdout)
        .is_test(false)
        .parse_env(
            env_logger::Env::new()
                .filter("LOG_LEVEL")
                .write_style("LOG_STYLE"),
        )
        .try_init()
}
