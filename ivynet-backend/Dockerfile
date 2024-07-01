FROM rust:alpine

ARG CONTEXT_APK
RUN test -n "$CONTEXT_APK"

WORKDIR /build

FROM base AS builder
ARG CONTEXT_APK
RUN mkdir -p "/build/"
COPY $CONTEXT_APK/ srv_app/
COPY Cargo.lock srv_app/
COPY ivynet-core ivynet-core/

WORKDIR /build/srv_app
# Dependency caching magic happens here
RUN cargo build --release --verbose

COPY Cargo.lock ./Cargo.lock
COPY $CONTEXT_APK/ ./srv_app
RUN cargo fmt -- --check
RUN cargo clippy
RUN #cargo test --release

# Build main binary
RUN cargo install --locked --path . --root /output

# Move the binary to fresh, lightweight container leaving the trash behind
FROM rust:alpine
ARG CONTEXT_APK
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /output/bin/$CONTEXT_APK /usr/local/bin/internal_service.app

RUN chmod +x /usr/local/bin/internal_service.app

CMD ["/usr/local/bin/internal_service.app"]
