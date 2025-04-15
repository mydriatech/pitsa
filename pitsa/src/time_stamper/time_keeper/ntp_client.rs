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

//! Network Time Protocol (NTP) client abstraction.

use sntpc::get_time;
use sntpc::NtpContext;
pub use sntpc::NtpResult;
use sntpc::StdTimestampGen;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::net::UdpSocket;

// https://docs.rs/sntpc/0.5.2/sntpc/
// https://github.com/vpetrigo/sntpc

/** Network Time Protocol (NTP) client abstraction.

Wrapper around the [`sntpc`](https://github.com/vpetrigo/sntpc) crate SNTPv4
implementation.

See also:

* [RFC 4330](https://datatracker.ietf.org/doc/html/rfc4330) Simple Network Time Protocol (SNTP) Version 4 for IPv4, IPv6 and OSI
* [RFC 5905](https://www.rfc-editor.org/rfc/rfc5905) Network Time Protocol Version 4: Protocol and Algorithms Specification
*/
pub struct NtpClient {
    server_addr: SocketAddr,
    client_socket: UdpSocket,
    context: NtpContext<StdTimestampGen>,
    timeout_micros: u64,
}

impl NtpClient {
    /// Return a new instance to the NTP server at `ntp_host` (`hostname:port` format).
    pub async fn new(ntp_host: &str, timeout_micros: u64) -> Arc<Self> {
        let ntp_host = if ntp_host.contains(':') {
            ntp_host.to_string()
        } else {
            ntp_host.to_string() + ":123"
        };
        let server_addr: SocketAddr = ntp_host
            .to_socket_addrs()
            .expect("Unable to resolve host")
            .next()
            .unwrap();
        let client_socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("Unable to crate UDP socket");
        log::info!(
            "Local UDP listener bound to {:?}.",
            client_socket.local_addr()
        );

        let context = NtpContext::new(StdTimestampGen::default());
        Arc::new(Self {
            server_addr,
            client_socket,
            context,
            timeout_micros,
        })
    }

    /// Request a NTP packet.
    pub async fn request_ntp_time(&self) -> Option<NtpResult> {
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_micros(self.timeout_micros);
        let res_res = tokio::time::timeout_at(
            deadline,
            get_time(self.server_addr, &self.client_socket, self.context),
        )
        .await;
        match res_res {
            Err(_e) => {
                log::warn!("No NTP response within {} µs.", self.timeout_micros);
            }
            Ok(Err(e)) => {
                log::warn!("Failed NTP request: {e:?}");
            }
            Ok(Ok(ntp_result)) => {
                return Some(ntp_result);
            }
        }
        None
    }
}
