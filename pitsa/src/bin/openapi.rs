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

//! CLI for generation of OpenAPI documentation without running the app.

extern crate pitsa as this_crate;

/// Write generated OpenAPI to stdout or file
fn main() {
    if let Some(filename) = std::env::args().nth(1) {
        let open_api_as_string = this_crate::rest_api::openapi_as_string();
        if filename.eq("-") {
            println!("{}", open_api_as_string)
        } else {
            std::fs::write(&filename, open_api_as_string)
                .map_err(|e| {
                    log::error!("Failed to write file '{filename}': {e:?}");
                })
                .ok();
        }
    } else {
        println!(
            "
Missing output target. Run with:

    cargo run --bin openapi -- -

    cargo run --bin openapi -- openapi.json
"
        );
    }
}
