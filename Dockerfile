FROM rust:1.86.0-slim-bullseye AS build

RUN USER=root apt-get update && apt-get install -y libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*
RUN USER=root cargo new --bin mqtt_test2
WORKDIR /mqtt_test2

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release && rm src/*.rs && rm ./target/release/deps/mqtt_test2*

COPY ./src ./src
COPY ./config.json ./

RUN cargo build --release

FROM debian:bullseye-slim

RUN USER=root apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=build ./mqtt_test2/config.json ./
COPY --from=build /mqtt_test2/target/release/mqtt_test2 usr/src/mqtt_test2

CMD ["/usr/src/mqtt_test2"]
