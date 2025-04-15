# PiTSA - Pico Time-Stamp Authority

Cloud native Time-Stamping implementation with minimalistic footprint
that you can run as a side-car container or at planet scale.

Implements [RFC 3161](https://www.rfc-editor.org/rfc/rfc3161) (and [more](pitsa/README.md)).


## Why Time-Stamps?

Time stamps proves the existence of data at a certain point in time.
This is a useful primitive building block in audit and/or long term archiving
systems with centralized trust.

Block chain systems provide a similar functionality with decentralized trust,
but comes with a different set of complexity.


## Quick start

Add the Helm repository:

```text
helm repo add mt-pitsa https://mydriatech.github.io/pitsa
```

Install the PiTSA Helm chart using self-signed time-stamping certificates with the `ed25519` signature algorithm:

```text
helm upgrade --install --atomic --create-namespace \
    --namespace pitsa-demo \
    --set ingress.enabled=true \
    --set ingress.host=tsa.dev.yourcompany.tld \
    --set app.signature.signatureAlgorithmOid=1.3.101.112 \
    --set app.signature.identity="common_name=Demo TSA\,organizational_unit_name=Your Company\,country_name=EU\,rfc822_name=no-reply@yourcompany.tld" \
    pitsa mt-pitsa/pitsa
```

Query the Time-Stamping Authority (TSA) using using `openssl` and `curl`:

```text
# Generate request
openssl ts -query -nonce -cert -data README.md -sha3-512 -out request.tsq

# Show generated request
openssl ts -query -in request.tsq -text

# Send generated request to the TSA
curl -H 'Content-Type: application/timestamp-query' --data-binary @request.tsq http://tsa.dev.yourcompany.tld -o response.tsr

# Show time-stamp response returned by the TSA
openssl ts -reply -in response.tsr -text

# Verify the time-stamp response (this is fairly pointless when using self-signed certs)
#openssl ts -verify -in response.tsr -queryfile request.tsq -CAfile trust_anchor.pem

# Show bundled certificates in the time-stamp response
openssl ts -reply -in response.tsr -token_out | openssl pkcs7 -inform der -print_certs
```

(Optional) uninstall the chart:

```text
helm delete --namespace pitsa-demo pitsa
```

## Implementation

See the [pitsa/](pitsa/) directory for implementation details.


## License

[Apache License 2.0 with Free world makers exception 1.0.0](LICENSE-Apache-2.0-with-FWM-Exception-1.0.0)

The intent of this license to

* Allow makers, innovators, integrators and engineers to do what they do best without blockers.
* Give commercial and non-commercial entities in the free world a competitive advantage.
* Support a long-term sustainable business model where no "open core" or "community edition" is ever needed.


## Governance model

This projects uses the [Benevolent Dictator Governance Model](http://oss-watch.ac.uk/resources/benevolentdictatorgovernancemodel) (site only seem to support plain HTTP).

See also [Code of Conduct](CODE_OF_CONDUCT.md) and [Contributing](CONTRIBUTING.md).

