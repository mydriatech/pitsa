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

//! Time-Stamp app

mod time_keeper;
mod tst_signing_info;

use self::time_keeper::TimeKeeper;
use self::tst_signing_info::TimeStampTokenSigningInfo;
use crate::conf::AppConfig;
use std::sync::Arc;
use tyst::Tyst;
use upkit_common::x509::tsp::build::TimeStampResp;
use upkit_common::x509::tsp::parse::TimeStampReqParser;
use upkit_common::x509::tsp::types::PkiFailureInfo;
use upkit_common::x509::tsp::types::TimeStampToken;
use upkit_common::x509::tsp::types::TstInfo;

/** Time-Stamp app.

This object is responsible for maintaining (by delegation):

* Time stamping signature chain and keys
* A time source
*/
pub struct TimeStamper {
    allowed_digest_oids: Vec<String>,
    allowed_policy_oids: Vec<String>,
    tst_signing_info: Arc<TimeStampTokenSigningInfo>,
    time_keeper: Arc<TimeKeeper>,
}

impl TimeStamper {
    /// Return a new instance of the app.
    pub async fn new(app_config: &Arc<AppConfig>) -> Arc<Self> {
        let time_keeper = TimeKeeper::new(
            app_config.time.ntp_host(),
            app_config.time.ntp_timeout_micros(),
            app_config.time.system_time_accuracy_micros(),
            app_config.time.ntp_sync_interval_micros(),
            app_config.time.tolerable_accuracy_micros(),
            app_config.time.ntp_query_for_every_request(),
        )
        .await;
        Arc::new(Self {
            allowed_digest_oids: app_config.sign.allowed_digest_oids(),
            allowed_policy_oids: vec![tyst::encdec::oid::as_string(&app_config.sign.policy_oid())],
            tst_signing_info: TimeStampTokenSigningInfo::new(app_config).await,
            time_keeper,
        })
    }

    /// Return `true` when a usable TS signing certificate and private key is
    /// avaialable and the configured time source is has an acceptable acurracy.
    pub fn is_ready(self: &Arc<Self>) -> bool {
        self.tst_signing_info.valid_signing_info_available()
            && self.time_keeper.is_within_tolerance()
    }

    /// Process encoded request and respond with an encoded signed time-stamp.
    pub async fn raw_time_stamp_request(self: &Arc<Self>, time_stamp_request: &[u8]) -> Vec<u8> {
        match TimeStampReqParser::from_bytes(time_stamp_request) {
            Ok(time_stamp_req) => self
                .time_stamp_request(&time_stamp_req)
                .await
                .as_bytes()
                .to_vec(),
            Err(e) => TimeStampResp::with_rejection(
                &[format!("Unable to parse request: {e:?}")],
                &Some(PkiFailureInfo::SystemFailure),
            )
            .as_bytes()
            .to_vec(),
        }
    }

    /// Process request and respond with a signed time-stamp.
    async fn time_stamp_request(
        self: &Arc<Self>,
        time_stamp_req: &TimeStampReqParser,
    ) -> TimeStampResp {
        let imprint_digest_oid = time_stamp_req.get_message_imprint_digest_oid();
        if let Some(known_digest) = Tyst::instance().digests().by_oid(&imprint_digest_oid) {
            // Assert correct message imprint digest size
            let expected = known_digest.get_digest_size_bits() / 8;
            let actual = time_stamp_req.get_message_imprint_digest_len_octets();
            if expected != actual {
                return TimeStampResp::with_rejection(
                    &[format!(
                        "Message imprint digest length ({actual} bytes) does not match the claimed algorithm's ({expected} bytes)."
                    )],
                    &Some(PkiFailureInfo::BadDataFormat)
                );
            }
            // Assert that message digest algo is allowed by configuration
            // Empty = allow any known
            if !self.allowed_digest_oids.is_empty()
                && !self.allowed_digest_oids.contains(&imprint_digest_oid)
            {
                return TimeStampResp::with_rejection(
                    &[format!("Message digest algorithm '{imprint_digest_oid}' in message imprint is not allowed.")], 
                    &Some(PkiFailureInfo::BadAlgo)
                );
            }
        } else {
            return TimeStampResp::with_rejection(
                &[format!(
                    "Unknown message digest algorithm '{imprint_digest_oid}' in message imprint."
                )],
                &Some(PkiFailureInfo::BadAlgo),
            );
        }
        // Verify policy oid against allow list. (Allow any policy if list is empty.)
        let mut response_policy_oid = [2, 5, 29, 32, 0].to_vec();
        if let Some(policy_oid) = time_stamp_req.get_req_policy_oid() {
            if self.allowed_policy_oids.is_empty() || self.allowed_policy_oids.contains(&policy_oid)
            {
                response_policy_oid = tyst::encdec::oid::from_string(&policy_oid).unwrap();
            } else {
                return TimeStampResp::with_rejection(
                    &[format!(
                        "Requested policy '{policy_oid}' is not allowed by this service."
                    )],
                    &Some(PkiFailureInfo::UnacceptedPolicy),
                );
            }
        } else if let Some(first_allowed) = self.allowed_policy_oids.first() {
            response_policy_oid = tyst::encdec::oid::from_string(first_allowed).unwrap();
        }
        // Assert that there are no critical extensions (since we don't understand any atm).
        if !time_stamp_req.get_critical_extension_oids().is_empty() {
            return TimeStampResp::with_rejection(
                &["Requested extension(s) are not supported by this service.".to_string()],
                &Some(PkiFailureInfo::UnacceptedExtension),
            );
        }
        if let Some((point_in_time_epoch_micros, accuracy_micros)) =
            self.time_keeper.get_epoch_time_with_accuracy_micros().await
        {
            // Build time stamp token info
            let tst_info = TstInfo::new(
                time_stamp_req,
                &response_policy_oid,
                point_in_time_epoch_micros,
                accuracy_micros,
            );
            // Sign and insert certs, ocsp responses etc
            if let Some(tst_signer) = self.tst_signing_info.get_dynamic_singing_info() {
                let time_stamp_token =
                    TimeStampToken::new(tst_info, &tst_signer, time_stamp_req.get_cert_req());
                TimeStampResp::with_success(false, time_stamp_token)
            } else {
                TimeStampResp::with_rejection(
                    &["Failed to sign response.".to_string()],
                    &Some(PkiFailureInfo::SystemFailure),
                )
            }
        } else {
            TimeStampResp::with_rejection(
                &["Failed to recieve current time with tolerable acurracy.".to_string()],
                &Some(PkiFailureInfo::TimeNotAvailable),
            )
        }
    }
}
