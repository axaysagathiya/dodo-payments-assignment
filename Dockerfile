FROM rust:slim-bullseye AS app-base

WORKDIR /app

RUN apt-get update \
    && apt-get install -y pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install sqlx-cli --version 0.9.0 --no-default-features --features postgres,rustls

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# ---

FROM app-base AS api-runtime

EXPOSE 8080
CMD ["sh", "-c", "cargo sqlx migrate run && cargo run --release --bin invoice-payment-system"]

# ---

FROM app-base AS mock-psp-runtime

EXPOSE 3001
CMD ["cargo", "run", "--release", "--bin", "mock_psp"]
