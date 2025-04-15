# PiTSA app

This folder contains the Rust implementation of the Pico Time-Stamp Authority (PiTSA).

## Features

* Implementing standards:
    * [RFC 3161](https://www.rfc-editor.org/rfc/rfc3161) Time-Stamp Protocol (TSP)
    * [RFC 5816](https://www.rfc-editor.org/rfc/rfc5816) ESSCertIDv2 Update for RFC 3161 (allow non-SHA1)
* Target configurable operational compliance with:
    * [RFC 3628](https://www.rfc-editor.org/rfc/rfc3628) Policy Requirements for Time-Stamping Authorities (TSAs)
    * [ETSI EN 319 421](https://www.etsi.org/deliver/etsi_en/319400_319499/319421/01.01.01_60/en_319421v010101p.pdf) Policy and Security Requirements for Trust Service Providers issuing Time-Stamps
    * [ETSI EN 319 422](https://www.etsi.org/deliver/etsi_en/319400_319499/319422/01.01.01_60/en_319422v010101p.pdf) Time-stamping protocol and time-stamp token profiles
* OCI containerizable app with tiny ("pico") memory footprint written in safe Rust.


## Limitations

* One TSA policy per deployment.


## Dependencies

This create depends on the following crates and is subject to the same features
and limitations as they evolve:

* TSU signing and message digest algorithm support depends on
  [`tyst`](https://github.com/mydriatech/tyst).
* TS extension depends on support in
  [`upkit-common`](https://github.com/mydriatech/upkit-common).
* TSU signing certificate enrollment protocol depends on
  [`upkit-leafops`](https://github.com/mydriatech/upkit-leafops) support.


## Development

See the [repository](https://github.com/mydriatech/pitsa) root how to contribute.

### Building and Running

Building the container locally:

```text
# Run from the repository root
podman build --pull-always -t localhost/mydriatech/pitsa:latest -f Containerfile .
```

Run the local container:

```text
podman run \
    --rm \
    --log-driver none \
    --name pitsa \
    --env PITSA_TIME_NTPHOST=pool.ntp.org \
    --stop-signal SIGINT --stop-timeout 10 \
    --publish "127.0.0.1:8080:8080" \
    localhost/mydriatech/pitsa:latest
```

### Examples

Once you have it running on `127.0.0.1:8080`, you can proceed with running the
examples.

Query the Time-Stamping Authority (TSA) using
[example client code](examples/tsp_example.rs) from the project:

```text
cargo run --example tsp_example -- http://127.0.0.1:8080/api/v1/tsp
```

### REST API documentation

Please see the generated OpenAPI documentation in [`openapi.json`](openapi.json).

A small CLI is provided for re-generating this during development.

```text
# Run from the repository root
cargo run --bin openapi -- pitsa/openapi.json
```

