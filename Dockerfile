FROM rust:1.86.0-slim-bullseye AS build

RUN USER=root apt-get update && apt-get install -y libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*
RUN USER=root cargo new --bin suuntaava_projekti
WORKDIR /suuntaava_projekti

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release && rm src/*.rs && rm ./target/release/deps/suuntaava_projekti*

COPY ./src ./src

RUN cargo build --release

FROM debian:bullseye-slim

RUN USER=root apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

ARG CONFIG_JSON_KEY
RUN if [ -n "${CONFIG_JSON_KEY}" ]; then echo ${CONFIG_JSON_KEY} | base64 --decode > ./config.json; fi

COPY --from=build /suuntaava_projekti/target/release/suuntaava_projekti usr/src/suuntaava_projekti

CMD ["/usr/src/suuntaava_projekti"]
