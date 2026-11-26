FROM docker.io/library/rust:1.100.0-slim-trixie@sha256:3b199fd6db7d709274df56565b8f4003f9e30d7bfcff9d53148c86053b7855e2 AS build
# The Discord gateway uses native TLS, which links against OpenSSL.
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev pkg-config
WORKDIR /app
COPY . .
RUN cargo build --release --locked && mkdir logs

FROM gcr.io/distroless/cc-debian13:nonroot@sha256:29ee70762214fe8190cea04ef1c0a84a8365879e047ce8f911ae5a5ab2355fa2
WORKDIR /app
COPY --from=build /app/target/release/steward /usr/local/bin/steward
COPY --from=build --chown=nonroot:nonroot /app/logs logs
ENTRYPOINT ["/usr/local/bin/steward"]
