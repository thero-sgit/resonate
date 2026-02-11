FROM rust:1.93 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release


FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    librdkafka1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/resonate .

EXPOSE 8080
CMD ["./resonate"]