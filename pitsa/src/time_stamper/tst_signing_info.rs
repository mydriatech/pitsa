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

//! Signature certificate chain and private key.

use crossbeam_skiplist::SkipMap;
use std::sync::Arc;
use tyst::traits::se::PrivateKey;
use tyst::Tyst;
use upkit_common::x509::cert::parse::CertificateParser;
use upkit_common::x509::tsp::build::RevocationInfoVariant;
use upkit_common::x509::tsp::build::TimeStampTokenSigner;
use upkit_leafops::enprov::CertificateEnrollmentOptions;
use upkit_leafops::enprov::CertificateEnrollmentProvider;
use upkit_leafops::enprov::EnrollmentProvider;
use upkit_leafops::enprov::MonitoredChain;
use upkit_leafops::enprov::MonitoredRevocationInfo;

use crate::conf::AppConfig;

/// The currently used time-stamp signing information.
struct CurrentSigningInfo {
    /// Content digest algorithms object identifier.
    pub digest_algorithm_oid: Vec<u32>,
    /// Digital signature algorithm object identifier.
    pub signing_algorithm_oid: Vec<u32>,
    /// Reference to the private key used for the digital signature.
    pub private_key: Arc<Box<dyn PrivateKey>>,
    /// Ordered signing certificate chain with the leaf first.
    pub signing_certificate_chain: Arc<MonitoredChain>,
}

/** Maintains up to date private key, signing certificate chain and revocation
info.

[RFC 8933 3.1](https://datatracker.ietf.org/doc/html/rfc8933#section-3.1):

```text
   ...the same digest algorithm MUST be used to compute both the digest of the
   SignedData encapContentInfo eContent, which is carried in the message-digest
   attribute, and the digest of the DER-encoded signedAttrs, which is passed to
   the signature algorithm.
```

As a simple example `if digest==SHA-256 then sign=SHA-256withECDSA` (or the
other way around...)
*/
pub struct TimeStampTokenSigningInfo {
    app_config: Arc<AppConfig>,
    cep: Arc<CertificateEnrollmentProvider>,
    current_signing_info: SkipMap<(), Arc<CurrentSigningInfo>>,
    certificate_enrollment_options: CertificateEnrollmentOptions,
    certificate_signature_algo_oid: Vec<u32>,
    supported_digest_algorithm_oid: Vec<u32>,
}

impl TimeStampTokenSigningInfo {
    /// Return a new instance.
    pub async fn new(app_config: &Arc<AppConfig>) -> Arc<Self> {
        Arc::new(Self {
            app_config: Arc::clone(app_config),
            cep: CertificateEnrollmentProvider::new(
                &app_config.sign.provider(),
                &app_config.sign.trust(),
            ),
            current_signing_info: SkipMap::default(),
            certificate_enrollment_options: CertificateEnrollmentOptions {
                template: app_config.sign.template(),
                credentials: app_config.sign.credentials(),
                identity: app_config.sign.identity(),
            },
            // TODO
            certificate_signature_algo_oid: app_config.sign.signature_algorithm_oid(),
            supported_digest_algorithm_oid: app_config.sign.digest_algorithm_oid(),
        })
        .init()
        .await
    }

    /// Start background task to ensure that we always have a valid signing
    /// certificate at hand.
    async fn init(self: Arc<Self>) -> Arc<Self> {
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                self_clone.maintain_signing_info().await;
                tokio::time::sleep(tokio::time::Duration::from_micros(30_000_000)).await;
            }
        });

        self
    }

    /// Return the [CurrentSigningInfo] if any.
    fn get_current_signing_info(self: &Arc<Self>) -> Option<Arc<CurrentSigningInfo>> {
        self.current_signing_info
            .front()
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Set the [CurrentSigningInfo].
    fn set_current_signing_info(
        self: &Arc<Self>,
        csi: Arc<CurrentSigningInfo>,
    ) -> Option<Arc<CurrentSigningInfo>> {
        let ret = self.get_current_signing_info();
        self.current_signing_info.insert((), csi);
        ret
    }

    /// Continiously keep signing certificate up to date
    async fn maintain_signing_info(self: &Arc<Self>) {
        log::debug!("Checking for newer signing certificate.");
        // new
        let sing_algo_oid_str = tyst::encdec::oid::as_string(&self.certificate_signature_algo_oid);
        if let Some(mut se) = Tyst::instance().ses().by_oid(&sing_algo_oid_str) {
            loop {
                let (public_key, private_key) = se.generate_key_pair();
                let signing_certificate_chain = self.cep.enroll_from_key_pair(
                    &self.certificate_signature_algo_oid,
                    public_key.as_ref(),
                    private_key.as_ref(),
                    &self.certificate_enrollment_options,
                );
                let signing_certificate_chain = MonitoredChain::new(
                    signing_certificate_chain,
                    &self.supported_digest_algorithm_oid,
                )
                .track_chain_status(3_000)
                .await;
                // Log certificate to allow correlation to this instance.
                if let Some(signing_cert) = signing_certificate_chain
                    .get_parsed_certificate_chain()
                    .first()
                {
                    let issuer_dn = signing_cert
                        .get_issuer()
                        .ok()
                        .and_then(|value| serde_json::to_string(&value).ok())
                        .unwrap_or("unknown".to_string());
                    log::info!(
                        "This instance ('{}') was issued a certificate with issuer '{issuer_dn}' and serial number 0x{}.",
                        self.app_config.context.as_ref().map(|context_config|context_config.get_kubernetes_context()).unwrap_or("(no k8s context detected)".to_string()),
                        signing_cert.get_serial_number().to_hex(),
                    );
                }

                if let Some(old) = self.set_current_signing_info(Arc::new(CurrentSigningInfo {
                    digest_algorithm_oid: self.supported_digest_algorithm_oid.to_vec(),
                    signing_algorithm_oid: self.certificate_signature_algo_oid.to_vec(),
                    private_key: Arc::new(private_key),
                    signing_certificate_chain: Arc::clone(&signing_certificate_chain),
                })) {
                    old.signing_certificate_chain.stop_tracking();
                }
                // Even if crazy short lived certs are used or certs that are revoked upon issuance, we limit the renewals to at least 1 second internvals.
                tokio::time::sleep(tokio::time::Duration::from_millis(1_000)).await;
                signing_certificate_chain
                    .await_leaf_expiration_or_chain_revocation(3 * 60)
                    .await;
            }
        } else {
            log::error!("Unknown signature algorithm '{sing_algo_oid_str}'.");
        }
    }

    /// Return `true`if the current signing certificate is valid.
    ///
    /// Useful for health checking.
    // TODO: Also check revocation!
    pub fn valid_signing_info_available(self: &Arc<Self>) -> bool {
        self.get_current_signing_info().is_some_and(|csi| {
            csi.signing_certificate_chain
                .get_parsed_certificate_chain()
                .first()
                .unwrap()
                .get_validity()
                .is_valid_at(upkit_common::util::time::now_epoch_seconds())
        })
    }

    /// Get a snapshot of the current info
    pub fn get_dynamic_singing_info(self: &Arc<Self>) -> Option<TimeStampTokenSigner> {
        self.get_current_signing_info().and_then(|csi|{
            let digest_algo_oid = csi.digest_algorithm_oid.to_vec();
            let sign_algo_oid = csi.signing_algorithm_oid.to_vec();
            let private_key = Arc::clone(&csi.private_key);
            let mris = csi
                .signing_certificate_chain
                .get_parsed_certificate_chain()
                .iter()
                .map(CertificateParser::fingerprint)
                .map(|fp| csi.signing_certificate_chain.get_revocation_info(fp))
                .collect::<Vec<_>>();
            let mut revocation_infos = vec![];
            for mri in mris {
                match mri {
                    MonitoredRevocationInfo::Crl { encoded } => {
                        revocation_infos.push(RevocationInfoVariant::Crl { encoded })
                    }
                    MonitoredRevocationInfo::OcspResponse { encoded } => {
                        revocation_infos.push(RevocationInfoVariant::OcspResponse { encoded })
                    }
                    MonitoredRevocationInfo::NotDefinedInCertificate => {}
                    MonitoredRevocationInfo::Missing => {
                        log::warn!("Missing revocation information. Unable to produce self-contained responses.");
                        return None;
                    }
                }
            }
            Some(TimeStampTokenSigner::new(
                digest_algo_oid,
                sign_algo_oid,
                private_key,
                csi
                    .signing_certificate_chain
                    .get_encoded_certificate_chain()
                    .to_vec(),
                revocation_infos,
            ))
        })
    }
}
