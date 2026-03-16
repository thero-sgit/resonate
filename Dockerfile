FROM rust:1.93-alpine AS builder

RUN apk add --no-cache \
    bash \
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
    zlib-static

WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/resonate ./resonate
EXPOSE 8080
CMD ["./resonate"]