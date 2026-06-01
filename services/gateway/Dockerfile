FROM rust:1.96-bookworm AS build
WORKDIR /src
# aws-lc-sys + ring need cmake
RUN apt-get update && apt-get install --no-install-recommends -y cmake && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/zt-gateway /usr/local/bin/zt-gateway
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/zt-gateway"]
