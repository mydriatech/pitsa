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

//! Example of using the Time-Stamp protocol over HTTP

use tyst::Tyst;
use tyst::encdec::hex::ToHex;
use upkit_common::x509::cert::parse::CertificateParser;
use upkit_common::x509::tsp::build::TimeStampReq;
use upkit_common::x509::tsp::parse::TimeStampRespParser;
use upkit_common::x509::tsp::types::PkiStatus;
use upkit_common::x509::tsp::validate::TimeStampResponseValidator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(endpoint) = std::env::args().nth(1) {
        let digest_sha3_512_oid = [2, 16, 840, 1, 101, 3, 4, 2, 10].as_slice();
        let mut digest_alg_oid_string = tyst::encdec::oid::as_string(digest_sha3_512_oid);
        let mut digest_alg_oid = digest_sha3_512_oid.to_vec();
        if let Some(supplied_digest_alg_oid) = std::env::args().nth(2) {
            digest_alg_oid = tyst::encdec::oid::from_string(&supplied_digest_alg_oid).unwrap();
            digest_alg_oid_string = supplied_digest_alg_oid;
        }
        println!("Using message digest (hash) algorithm: '{digest_alg_oid_string}'");
        let message = "Prove that this message existed at point in time!";
        let digest_bytes = Tyst::instance()
            .digests()
            .by_oid(&digest_alg_oid_string)
            .unwrap()
            .hash(message.as_bytes());
        println!("Message: '{message}'");
        println!("Request");
        println!("  Message fingerprint: '{}'.", digest_bytes.to_hex());
        // Request
        let cert_req = true;
        let encoded_time_stamp_resp = request_time_stamp(
            &endpoint,
            &TimeStampReq::new(cert_req, &digest_alg_oid, &digest_bytes),
        )?;
        let time_stamp_response = TimeStampRespParser::from_bytes(&encoded_time_stamp_resp)?;
        // Handle response
        println!("Response");
        let status = time_stamp_response.get_status();
        println!("  PKIStatus: {} ({})", status.name(), status.as_value());
        match status {
            PkiStatus::Rejection
            | PkiStatus::Waiting
            | PkiStatus::RevocationWarning
            | PkiStatus::RevocationNotification => {
                if let Some(failure_info) = time_stamp_response.get_failure_info() {
                    println!(
                        "  PKIFailureInfo: {} ({})",
                        failure_info.name(),
                        failure_info.as_index()
                    );
                } else {
                    println!("  PKIFailureInfo: unknown");
                }
                return Ok(());
            }
            _ => {}
        }
        let time_stamp_token = time_stamp_response.get_time_stamp_token().unwrap();
        let tst_info = time_stamp_token.get_tst_info().unwrap();
        let (epoch_time_micros, accuracy_micros) = tst_info.get_epoch_time_micros();
        println!("  Time stamp in epoch time (µs): {epoch_time_micros} ± {accuracy_micros}.");
        let (digest_algorithm_oid, message_digest) = tst_info.get_message_imprint();
        if !digest_algorithm_oid.eq(&digest_alg_oid) {
            println!(
                "  Wrong digest algorithm was returned: {}",
                tyst::encdec::oid::as_string(&digest_algorithm_oid)
            );
        }
        println!("  Message fingerprint: '{}'.", message_digest.to_hex());
        // Extract certs
        let encodede_certificates = time_stamp_token.get_certificates();
        if encodede_certificates.is_empty() {
            if cert_req {
                println!(
                    "  No certificate were present in the response, even though this was requested!"
                );
            } else {
                println!("  No certificate were present in the response.");
            }
        } else {
            let cps = encodede_certificates
                .iter()
                .map(|encoded_certificate| {
                    CertificateParser::from_bytes(encoded_certificate).unwrap()
                })
                .collect::<Vec<_>>();
            cps.iter().enumerate().for_each(|(i, cp)| {
                println!(
                    "  Certificate #{i} subject: '{}'.",
                    serde_json::to_string(&cp.get_subject().unwrap()).unwrap()
                );
            });
            // Assume that trust anchor is in the last position of the response
            // SECURITY NOTICE: You MUST normally supply this to your running app!
            // (This is you to make the demo example convinient to try.)
            let trust_anchor_der = encodede_certificates.last().unwrap();
            println!(
                "  Trust anchor cert's SHA3-512 fingerprint: '{}'.",
                upkit_common::x509::fingerprint_data(&trust_anchor_der)
            );
            // validate TS response
            TimeStampResponseValidator::validate_at_point_of_timestamp(
                vec![trust_anchor_der.to_vec()],
                &encoded_time_stamp_resp,
            )
            .unwrap();
        }
    } else {
        let digest_algos = Tyst::instance()
            .digests()
            .get_algorithm_meta_datas()
            .iter()
            .filter(|amd| amd.oid().is_some())
            .map(|amd| format!("{}: {}", amd.name(), amd.oid().unwrap()))
            .collect::<Vec<_>>();
        println!(
            "
Missing API URL. Run with:

    cargo run --example tsp_example -- http://127.0.0.1:8080/api/v1/tsp [optional hash algorithm OID]

You can also try a public TSA service:

    cargo run --example tsp_example -- http://tsa.belgium.be/connect 2.16.840.1.101.3.4.2.3

SHA3-512 is the default algorithm.

Other possible choices: {digest_algos:?}
"
        );
    }
    Ok(())
}

fn request_time_stamp(
    endpoint_url: &str,
    time_stamp_request: &TimeStampReq,
    //) -> Result<TimeStampRespParser, Box<dyn std::error::Error>> {
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let encoded_time_stamp_resp = ureq::post(endpoint_url)
        .content_type("application/timestamp-query")
        .send(time_stamp_request.as_bytes())?
        .body_mut()
        .read_to_vec()?;
    Ok(encoded_time_stamp_resp)
}
