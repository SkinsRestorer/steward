FROM docker.io/library/rust:1.100.0-slim-trixie@sha256:6f14b2b4b278ec0d50aa1d69541c304ce0f95e0f6268487aa9924af1c2e9aebe AS build
# The Discord gateway uses native TLS, which links against OpenSSL.
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev pkg-config
WORKDIR /app
COPY . .
RUN cargo build --release --locked && mkdir logs

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:532c7a94f9e96ba9db75eb9388f4227751131c7cb0a411d1a8521af4096135de
WORKDIR /app
COPY --from=build /app/target/release/steward /usr/local/bin/steward
COPY --from=build --chown=nonroot:nonroot /app/logs logs
ENTRYPOINT ["/usr/local/bin/steward"]
