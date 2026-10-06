# Multi-stage build for the axum API (workspace).
FROM rust:1.96-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations
RUN cargo build --release -p biomarker-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin app
COPY --from=builder /build/target/release/biomarker-api /usr/local/bin/
COPY --from=builder /build/migrations /app/migrations
WORKDIR /app
ENV APP_DEMO_CSV=/app/demo_labs.csv
USER app
EXPOSE 8003
CMD ["biomarker-api"]
