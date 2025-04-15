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

//! Environment context of the app.

use super::AppConfigDefaults;
use serde::{Deserialize, Serialize};

/// Environment context of the app.
#[derive(Debug, Deserialize, Serialize)]
pub struct ContextConfig {
    /// Helm chart can inject `metadata.name` here using K8s Downward API.
    pod: Option<String>,
    /// Helm chart can inject service name fronting Pod here.
    service: Option<String>,
    /// Helm chart can inject `metadata.namespace` here using K8s Downward API.
    namespace: Option<String>,
}

impl AppConfigDefaults for ContextConfig {}

impl ContextConfig {
    /// Return the Kubernetes DNS name `pod.service.namespace.svc` if available.
    pub fn get_kubernetes_context(&self) -> String {
        format!(
            "{}.{}.{}.svc",
            self.pod.clone().unwrap_or("?".to_string()),
            self.service.clone().unwrap_or("?".to_string()),
            self.namespace.clone().unwrap_or("?".to_string()),
        )
    }
}
