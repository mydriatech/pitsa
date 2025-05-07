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

use super::AppConfigDefaults;
use config::builder::BuilderState;
use config::ConfigBuilder;
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;
use upkit_common::x509::cert::types::IdentityFragment;
use upkit_common::x509::cert::types::WellKnownAttribute;
use upkit_common::x509::cert::types::WellKnownGeneralName;
use upkit_leafops::enprov::CertificateEnrollmentOptions;

/// Configuration for the time source.
#[derive(Deserialize, Serialize)]
pub struct SignerConfig {
    /// See [policy_oid()](Self::policy_oid()).
    policy: String,
    /// See [signature_algorithm_oid()](Self::signature_algorithm_oid()).
    signature: String,
    /// See [digest_algorithm_oid()](Self::digest_algorithm_oid()).
    digest: String,
    /// See [enrollment_provider_options()](Self::enrollment_provider_options()).
    enprov: Option<String>,
}

impl std::fmt::Debug for SignerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendConfig")
            .field("policy", &self.policy)
            .field("signature", &self.signature)
            .field("digest", &self.digest)
            .field("enprov", &self.enprov)
            .field("enprov_options", &self.enrollment_provider_options())
            .finish()
    }
}

impl AppConfigDefaults for SignerConfig {
    /// Provide defaults for this part of the configuration
    fn set_defaults<T: BuilderState>(
        config_builder: ConfigBuilder<T>,
        prefix: &str,
    ) -> ConfigBuilder<T> {
        config_builder
            .set_default(prefix.to_string() + "." + "policy", "2.5.29.32.0")
            .unwrap()
            // ML-DSA-65:       2.16.840.1.101.3.4.3.18
            // ecdsa_sha_384:   1.2.840.10045.4.3.3];
            // ed25519:         1.3.101.112
            .set_default(
                prefix.to_string() + "." + "signature",
                "1.3.101.112".to_string(),
            )
            .unwrap()
            // SHA3-512
            .set_default(
                prefix.to_string() + "." + "digest",
                "2.16.840.1.101.3.4.2.10".to_string(),
            )
            .unwrap()
            .set_default(prefix.to_string() + "." + "enprov", "")
            .unwrap()
    }
}

impl SignerConfig {
    /// Get the TSA policy OID.
    pub fn policy_oid(&self) -> Vec<u32> {
        tyst::encdec::oid::from_string(&self.policy)
            .map_err(|e|{
                log::warn!("Unable to parse configured policy '{}' as OID. Will use default. Error was: {e:?}", &self.policy);
            })
            .ok()
            .unwrap_or(vec![2, 5, 29, 32, 0])
    }

    /// Return a list of OID strings with allowed message imprint digest
    /// algorithms.
    ///
    /// Empty implies that any message digest algorithm supported by the `tyst`
    /// crate is allowed.
    pub fn allowed_digest_oids(&self) -> Vec<String> {
        // Allow any message digest algorithm supported by TYST for now.
        vec![]
    }

    /// Return [CertificateEnrollmentOptions] from the configured JSON file.
    pub fn enrollment_provider_options(&self) -> CertificateEnrollmentOptions {
        if let Some(enprov) = self.enprov.as_ref() {
            let enprov = enprov.trim();
            if !enprov.is_empty() && enprov.ends_with(".json") {
                let full_filename = std::path::PathBuf::from(enprov);
                if log::log_enabled!(log::Level::Debug) {
                    log::trace!("Loading '{}'.", full_filename.display());
                }
                match std::fs::read_to_string(&full_filename) {
                    Ok(content) => match CertificateEnrollmentOptions::from_str(&content) {
                        Ok(options) => {
                            return options;
                        }
                        Err(e) => {
                            log::warn!("Failed to parse '{}': {e}", full_filename.display());
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to read '{}': {e}", full_filename.display());
                    }
                }
            }
        }
        // Provide a default for testing
        CertificateEnrollmentOptions {
            provider: "self_signed".to_string(),
            template: "server".to_string(),
            credentials: None,
            identity: vec![
                IdentityFragment {
                    name: WellKnownAttribute::CommonName.as_name(),
                    value: "Dummy self-signed TSA unit cert".to_string(),
                },
                IdentityFragment {
                    name: WellKnownAttribute::CountryName.as_name(),
                    value: "SE".to_string(),
                },
                IdentityFragment {
                    name: WellKnownGeneralName::Rfc822Name.as_name(),
                    value: "no-reply@example.com".to_string(),
                },
            ],
            service: None,
            trust: None,
        }
    }

    /*
    /// Get the enrollment provider name.
    pub fn provider(&self) -> String {
        self.provider.to_owned()
    }

    /// Return what the enrollment provider should trust.
    pub fn connection(&self) -> EnrollmentConnection {
        let connection = self
            .connection
            .split(',')
            .filter_map(Self::property_to_tuplet)
            .collect::<Vec<_>>();
        if !self.connection.is_empty() {
            // Custom parsing here as well.. "shared_secret=foo123" or "username=someone,password=foo123" etc
            match connection
                .first()
                .map(|(key, _value)| key.to_string())
                .unwrap()
                .as_str()
            {
                "base_url" => {
                    return EnrollmentConnection::BaseUrl {
                        base_url: connection.first().unwrap().1.to_string()
                    }
                }
                unknown_key => {
                    log::debug!("Failed to detect type of enrollment connection. First key was '{unknown_key}'.");
                }
            }
        }
        EnrollmentConnection::External
    }

    /// Return what the enrollment provider should trust.
    pub fn trust(&self) -> EnrollmentTrust {
        if !self.trust.is_empty() {
            log::info!("Only '' (external responsibility) trust is supported atm.");
        }
        EnrollmentTrust::External
    }

    /// Get the certificate template used during certificate enrollment.
    pub fn template(&self) -> String {
        self.template.to_owned()
    }

    /// Return the credentials
    pub fn credentials(&self) -> EnrollmentCredentials {
        let credentials = self
            .credentials
            .split(',')
            .filter_map(Self::property_to_tuplet)
            .collect::<Vec<_>>();
        if !self.credentials.is_empty() {
            // Custom parsing here as well.. "shared_secret=foo123" or "username=someone,password=foo123" etc
            match credentials
                .first()
                .map(|(key, _value)| key.to_string())
                .unwrap()
                .as_str()
            {
                "shared_secret" => {
                    return EnrollmentCredentials::SharedSecret {
                        secret: tyst::encdec::base64::decode(
                            credentials.first().unwrap().1.as_str(),
                        )
                        .unwrap(),
                    }
                }
                unknown_key => {
                    log::debug!("Failed to detect type of enrollment credentials. First key was '{unknown_key}'.");
                }
            }
        }
        EnrollmentCredentials::External
    }

    /// Return the requested identity as fragments.
    pub fn identity(&self) -> Vec<IdentityFragment> {
        Self::to_identity_fragments(&self.identity.split(',').collect::<Vec<_>>())
    }

    fn to_identity_fragments(aavs: &[&str]) -> Vec<IdentityFragment> {
        aavs.iter()
            .filter_map(|aav| Self::property_to_tuplet(aav))
            .filter_map(|tuplet| IdentityFragment::try_from(tuplet).ok())
            .collect()
    }

    fn property_to_tuplet(property: &str) -> Option<(String, String)> {
        let mut split = property.splitn(2, '=');
        let (key, value) = (split.next(), split.next());
        key.and_then(|key| value.map(|value| (key.to_string(), value.to_string())))
    }
    */

    /// Get the signature algorithm OID.
    pub fn signature_algorithm_oid(&self) -> Vec<u32> {
        tyst::encdec::oid::from_string(&self.signature).unwrap()
    }

    /// Get the message digest algorithm OID.
    pub fn digest_algorithm_oid(&self) -> Vec<u32> {
        tyst::encdec::oid::from_string(&self.digest).unwrap()
    }
}
