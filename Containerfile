FROM docker.io/library/rust:alpine as builder
WORKDIR /work
COPY . .
RUN \
    apk add musl-dev curl xz && \
    cargo update && \
    cargo build --target=x86_64-unknown-linux-musl --release && \
    mkdir out && \
    cd  out && \
    mv ../target/x86_64-unknown-linux-musl/release/clacheless-bin clacheless && \
    mv ../target/x86_64-unknown-linux-musl/release/clacheless-cli clacheless-cli && \
    xz -k -6 clacheless && \
    xz -k -9 clacheless-cli && \
    cd ../ && \
    mkdir -p /work/data && \
    ./bin/extract-third-party-licenses.sh && \
    XZ_OPT='-9' tar cJf licenses.tar.xz licenses/

FROM ghcr.io/mydriatech/the-ground-up:1.0.0 as tgu

FROM scratch

LABEL org.opencontainers.image.source="https://github.com/mydriatech/clacheless.git"
LABEL org.opencontainers.image.description="Lightweight distributed in-memory cache for web application back-ends."
LABEL org.opencontainers.image.licenses="Apache-2.0 WITH FWM-Exception-1.0.0 AND Apache-2.0 AND BSD-2-Clause AND BSD-3-Clause AND ISC AND MIT AND Unicode-3.0 AND Zlib AND LGPL-2.1"
LABEL org.opencontainers.image.vendor="MydriaTech AB"

COPY --from=tgu  --chown=10001:0 /licenses-tgu.tar.xz /licenses-tgu.tar.xz
COPY --from=tgu  --chown=10001:0 /the-ground-up /clacheless
COPY --from=tgu  --chown=10001:0 /the-ground-up /clacheless-cli
COPY --from=tgu  --chown=10001:0 /the-ground-up-bin /clacheless-bin
COPY --from=tgu  --chown=10001:0 /the-ground-up-bin /clacheless-cli-bin
COPY --from=builder --chown=10001:0 /work/out/clacheless.xz /clacheless.xz
COPY --from=builder --chown=10001:0 /work/out/clacheless-cli.xz /clacheless-cli.xz
COPY --from=builder --chown=10001:0 --chmod=770 /work/data /data
COPY --from=builder --chown=10001:0 --chmod=770 /work/licenses.tar.xz /licenses.tar.xz

WORKDIR /

USER 10001:0

# REST API for side-car use.
EXPOSE 8080
# gRPC inter-Pod communication.
EXPOSE 9000

ENV PATH "/"
ENV LOG_LEVEL "DEBUG"
# Override using K8s Downward API
ENV POD_NAME "clacheless-0"
# Template for inter-Pod gRPC connections. "ORDINAL" will be replaced.
ENV CLACHELESS_ADDR_TEMPLATE "statefulsetname-ORDINAL.headlessservicename.namespace.svc:9090"
# How long to keep a cached item in seconds.
ENV CLACHELESS_TTL "3600"

CMD ["/clacheless"]
