FROM docker.io/library/rust:1.89.0-alpine as builder
WORKDIR /work
COPY . .
RUN \
    apk add musl-dev curl xz && \
    cargo update && \
    cargo build --target=x86_64-unknown-linux-musl --release && \
    xz -k -6 target/x86_64-unknown-linux-musl/release/pitsa && \
    mv target/x86_64-unknown-linux-musl/release/pitsa.xz pitsa.xz && \
    mkdir -p /work/data && \
    ./bin/extract-third-party-licenses.sh && \
    tar cJf licenses.tar.xz licenses/

FROM ghcr.io/mydriatech/the-ground-up:1.0.0 as tgu

FROM scratch

LABEL org.opencontainers.image.source="https://github.com/mydriatech/pitsa"
LABEL org.opencontainers.image.description="Pico Time-Stamp Authority"
LABEL org.opencontainers.image.licenses="Apache-2.0 WITH AdditionRef-FWM-Exception-1.0.0 AND Apache-2.0 AND BSD-2-Clause AND BSD-3-Clause AND ISC AND MIT AND Unicode-3.0 AND MPL-2.0 AND Zlib"
LABEL org.opencontainers.image.vendor="MydriaTech AB"

COPY --from=tgu  --chown=10001:0 /licenses-tgu.tar.xz /licenses-tgu.tar.xz
COPY --from=tgu  --chown=10001:0 /the-ground-up /pitsa
COPY --from=tgu  --chown=10001:0 /the-ground-up-bin /pitsa-bin
COPY --from=builder --chown=10001:0 /work/pitsa.xz /pitsa.xz
COPY --from=builder --chown=10001:0 --chmod=770 /work/licenses.tar.xz /licenses.tar.xz

WORKDIR /

USER 10001:0

EXPOSE 8080

#ENV LOG_LEVEL "DEBUG"

# See pitsa/src/conf/rest_api_config.rs for description and defaults.
#ENV PITSA_API_ADDRESS
#ENV PITSA_API_PORT

# See pitsa/src/conf/time_source_config.rs for description and defaults.
#ENV PITSA_TIME_NTPHOST
#ENV PITSA_TIME_TIMEOUT
#ENV PITSA_TIME_ACCURACY
#ENV PITSA_TIME_INTERVAL
#ENV PITSA_TIME_TOLERANCE
#ENV PITSA_TIME_ALWAYS

# See pitsa/src/conf/signer_config.rs for description and defaults.
#ENV PITSA_SIGN_DIGEST
#ENV PITSA_SIGN_SIGNATURE
#ENV PITSA_SIGN_ENPROV

# See pitsa/src/conf/context_config.rs for description and defaults.
#ENV PITSA_CONTEXT_POD
#ENV PITSA_CONTEXT_SERVICE
#ENV PITSA_CONTEXT_NAMESPACE

CMD ["/pitsa"]
