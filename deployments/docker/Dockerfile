FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p sdkwork-api-account-standalone-gateway

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/target/release/sdkwork-api-account-standalone-gateway /usr/local/bin/sdkwork-api-account-standalone-gateway
ENV ACCOUNT_API_BIND=0.0.0.0:18095
EXPOSE 18095
CMD ["sdkwork-api-account-standalone-gateway"]
