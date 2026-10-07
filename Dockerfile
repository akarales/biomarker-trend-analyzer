# Multi-stage build for the axum API (workspace).
# builder and runtime pinned to the SAME Debian release (trixie): a newer
# glibc in the builder than in the runtime makes the binary fail to start
FROM rust:1.96-slim-trixie AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY crates ./crates
# query! macros check against the committed .sqlx/ metadata (no DB at build);
# migrations are embedded in the binary by sqlx::migrate!
ENV SQLX_OFFLINE=true
RUN cargo build --release -p biomarker-api

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin app
COPY --from=builder /build/target/release/biomarker-api /usr/local/bin/
WORKDIR /app
ENV APP_DEMO_DATA=/app/demo
USER app
EXPOSE 8003
CMD ["biomarker-api"]
