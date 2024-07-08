FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /build
FROM chef AS planner
COPY . .

WORKDIR /build

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && apt-get install -y protobuf-compiler
ARG CONTEXT_APK
COPY --from=planner /build/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin $CONTEXT_APK

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
ARG CONTEXT_APK
RUN apt-get update && apt-get install -y openssl
COPY --from=builder /build/target/release/$CONTEXT_APK /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]
