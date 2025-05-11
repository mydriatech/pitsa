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

//! Time-Stamp Protocol via HTTP as defined in
//! [RFC3161 3.4](https://www.rfc-editor.org/rfc/rfc3161#section-3.4).

use super::AppState;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::Result;
use actix_web::error;
use actix_web::post;
use actix_web::web;
use futures::StreamExt;

const CONTENT_TYPE_TS_QUERY: &str = "application/timestamp-query";
const CONTENT_TYPE_TS_REPLY: &str = "application/timestamp-reply";
// A TimeStamp request is usually around 5 KiB.
const MAX_SIZE: usize = 8 * 1024;

#[allow(dead_code)]
#[derive(utoipa::ToSchema)]
#[schema(format = Binary, value_type = Object)]
struct BinaryType(Vec<u8>);

/// Time-Stamp Protocol via HTTP as defined in
/// [RFC3161 3.4](https://www.rfc-editor.org/rfc/rfc3161#section-3.4).
#[utoipa::path(
    context_path = "/api/v1",
    request_body(
        description = "DER encoded TimeStampReq.",
        content_type = CONTENT_TYPE_TS_QUERY,
        content = inline(BinaryType)
    ),
    responses(
        (
            status = 200,
            description = "Ok. Parse the DER encoded TimeStampResponse for actual status defined in the time-stamping protocol.",
            content_type = CONTENT_TYPE_TS_REPLY,
            body = inline(BinaryType),
        ),
        (status = 404, description = "Bad Reqest"),
    ),
)]
#[post("/tsp")]
pub async fn tsp_raw_time_stamp_request(
    http_request: HttpRequest,
    app_state: web::Data<AppState>,
    //path: web::Path<String>,
    mut payload: web::Payload,
) -> Result<HttpResponse, Error> {
    if log::log_enabled!(log::Level::Debug)
        && !http_request
            .headers()
            .get("content-type")
            .is_some_and(|value| {
                value
                    .to_str()
                    .is_ok_and(|value| value.eq(CONTENT_TYPE_TS_QUERY))
            })
    {
        log::debug!("Wrong content-type in request. (Allowing this to proceed anyway.)");
    }
    let content_length_estimate = http_request
        .headers()
        .get("content-length")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|header_value_str| header_value_str.parse::<usize>().ok())
        .unwrap_or(1024);
    if content_length_estimate > MAX_SIZE {
        return Err(error::ErrorBadRequest(
            "Content-length indicates that this request is too large.",
        ));
    }
    // TODO: Since MAX_SIZE is fairly low for TSRs, we could avoid heap alloc.
    //       This however only makes sense when redoing the rest of the call stack.
    let mut request_body = web::BytesMut::with_capacity(content_length_estimate);
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (request_body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("Request is too large."));
        }
        request_body.extend_from_slice(&chunk);
    }
    let time_stamp_request = request_body.freeze();
    let time_stamp_response = app_state
        .app
        .raw_time_stamp_request(&time_stamp_request)
        .await;
    Ok(HttpResponse::Ok()
        .insert_header(("content-type", CONTENT_TYPE_TS_REPLY))
        .body(time_stamp_response))
}
