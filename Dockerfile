FROM rust:latest as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    libgtk-3-dev \
    libasound2-dev\
    cmake

WORKDIR /app

COPY . .

RUN cargo build --release

FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    libgtk-3-0t64 \
    libasound2t64 \
    libudev1 \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/bindkey-software /usr/local/bin/bindkey_app

CMD ["bindkey_app"]