# Time-Stamp Unit (TSU) certificate enrollment with EJBCA CE

EJBCA CE 9.1.1 was used for writing this guide.

Only the minimal EJBCA configuration required for a functional example is
provided in this guide, but the CMP integration remains valid even in a
production setup.

## Prerequisites

* Functional Kubernetes installation with `Ingress` support.
* Basic knowledge about EJBCA and OpenSSL.
* Sufficient resources to run EJBCA.

## Deploy EJBCA CE using Helm

Install EJBCA CE into the `ejbca-ce` namespace using external DNS name
`ejbca.localdomain` and an ephemeral database:

```text
helm upgrade --install --atomic --create-namespace \
    --namespace ejbca-ce \
    --set ejbca.useEphemeralH2Database=true \
    --set ingress.enabled=true \
    --set 'ingress.hosts[0].host=ejbca.localdomain' \
    --set 'ingress.hosts[0].paths[0].path="/ejbca"' \
    --set 'ingress.hosts[0].paths[0].pathType=Prefix' \
    ejbca-ce oci://repo.keyfactor.com/charts/ejbca-ce
```

Watch the EJBCA logs until it has started:

```text
kubectl -n ejbca-ce logs deployment.apps/ejbca-ce -f
```

Access EJBCA's Admin UI using `https://ejbca.localdomain/ejbca/adminweb/`.

## Setup a new issuing CA

### Create a new Crypto Token for the CA

`Admin UI → CA functions -> Crypto Tokens -> Create new...`

Generate a key for certificate issuance and one for everything else. E.g.

* `signKey` using `ML-DSA-65`
* `defaultKey` using `RSA-3072`

### Create a new Certificate Authority

`Admin UI → CA functions -> Certificate Authorities -> "tsu-ca" Create...`

* Use Crypto Token that was created in the previous step.
* Specify a validity like "`1y`"
* Disable `Enforce unique DN`

## Setup a "tsu-cp" Certificate Profile

`Admin UI → CA functions -> Certificate Profiles -> "tsu-cp" Add...`

* Key Usage: `Non-repudiation` only
* Extended Key Usage: `Time Stamping` only
* Available CAs: `tsu-ca`

## Setup a "tsu-eep" End Entity Profile

`Admin UI → CA functions -> End Entity Profiles -> "tsu-eep" Add Profile...`

`Admin UI → CA functions -> End Entity Profiles -> tsu-eep Edit End Entity Profile`

Subject DN Attributes:

* CN: Empty. Required and modifiable.
* O: `MydriaTech AB`. Not required and not modifiable.
* C: `SE`. Not required and not modifiable.

Subject Alternative Name:

* RFC 822 Name: `no-reply@example.com`. Required and not modifiable.

Main Certificate Data:

* Default Certificate Profile: `tsu-cp`
* Available Certificate Profiles: `tsu-cp`
* Default CA: `tsu-ca`
* Available CAs: `tsu-ca`
* Default Token: `User Generated`
* Available Tokens: `User Generated`


## Setup a "tsu" CMP alias

`Admin UI → System Configuration -> CMP Configuration -> Add`

* Name: "`tsu`"
* CMP Operational Mode: `RA mode`
* CMP Authentication Module: `HMAC` | `Specify secret` | "`foobar123`"
* RA Name Generation Scheme: `RANDOM`
* RA Name Generation Prefix: "`tsu-ee-`"
* RA End Entity Profile: `tsu-eep`
* Certificate renewal with same keys: Disabled

UI work-around: Save and edit the CMP alias `tsu` again.

* CMP Response Protection: "`pbe`"
* CMP Response Additional CA certificates: Add "`tsu-ca`"

Save

## Configure and deploy PiTSA

Create configuration `pitsa-values-with-cmp.yaml` for using EJBCA as the source of TSU certificates:

```text
app:
  signature:
    signatureAlgorithmOid: 2.16.840.1.101.3.4.3.18
    enprov:
      # Certificate Management Protocol (CMP) provider
      provider: cmp
      template: tsu
      identity:
        - name: common_name
          value: The greatest little TSA unit
        - name: organization_name
          value: MydriaTech AB
        - name: country_name
          value: SE
        # This value must match the value specified in "tsu-eep"
        - name: rfc822_name
          value: no-reply@example.com
      service:
      #  # A trailing slash and the value of 'template' will be appended.
      #  base_url: https://ca.example.com/.well-known/cmp/p
        base_url: http://ejbca-ce.ejbca-ce.svc.cluster.local:8081/ejbca/publicweb/cmp
      credentials:
        shared_secret: foobar123

ingress:
  enabled: true
  host: pitsa-demo.localdomain

# Make sure we have more than one instance that will request a certificate.
replicaCount: 3
```

```text
helm repo add mt-pitsa https://mydriatech.github.io/pitsa
helm repo update
```

```text
helm upgrade --install --atomic --create-namespace \
    --namespace pitsa-demo \
    --values pitsa-values-with-cmp.yaml \
    pitsa mt-pitsa/pitsa
```

## Run examples

```text
# Generate request
openssl ts -query -nonce -cert -data README.md -sha3-512 -out request.tsq

# Show generated request
openssl ts -query -in request.tsq -text

# Send generated request to the TSA
curl -H 'Content-Type: application/timestamp-query' --data-binary @request.tsq http://pitsa-demo.localdomain/api/v1/tsp -o response.tsr

# Show time-stamp response returned by the TSA
openssl ts -reply -in response.tsr -text

# Download the CA certificate ("CN=tsu-ca" -> caid=-606243993)
curl "http://ejbca.localdomain/ejbca/ra/cert?caid=-606243993&chain=false&format=pem" -o tsu-ca.pem

# Verify the time-stamp response (requires ML-DSA support in OpenSSL)
openssl ts -verify -in response.tsr -queryfile request.tsq -CAfile tsu-ca.pem

# Show bundled certificates in the time-stamp response
openssl ts -reply -in response.tsr -token_out | openssl pkcs7 -inform der -print_certs
```

As you can see, the TSU certificate used to sign the time-stamp response was
issued by the deployed EJBCA. → Success!


## Clean up

```text
helm delete --namespace pitsa-demo pitsa
helm delete --namespace ejbca-ce ejbca-ce
```

## Rolling over the shared secret

* In EJBCA
    * Clone CMP alias `tsu` into `tsu-v2`.
    * Update secret in CMP alias `tsu-v2` to `foobar456`.
* Alter the PiTSA Helm chart configuration:
    * `app.signature.enprov.template` to `tsu-v2`
    * `app.signature.enprov.credentials.shared_secret` to `foobar456`
* Re-run the `helm upgrade ...` command for PiTSA.
* In EJBCA
    * Delete the CMP alias `tsu`.

