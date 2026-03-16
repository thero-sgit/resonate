FROM rust:1.93-alpine AS builder

RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    cmake \
    make \
    g++ \
    librdkafka-dev \
    cyrus-sasl-dev \
    zlib-dev \

WORKDIR /app
COPY . .

RUN rustup target add x86_64-unknown-musl
RUN cargo build --release --target x86_64-unknown-musl

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-musl/release/resonate ./resonate
EXPOSE 8080
CMD ["./resonate"]