FROM rust:1.86.0-slim-bullseye AS build
# install some required dependencies
RUN USER=root apt-get update && apt-get install -y libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*
# create a new rust application
RUN USER=root cargo new --bin suuntaava_projekti
WORKDIR /suuntaava_projekti
# copy the manifest
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
# build the dependencies only so they get cached
RUN cargo build --release && rm src/*.rs && rm ./target/release/deps/suuntaava_projekti*
# copy the source code
COPY ./src ./src
# build the application
RUN cargo build --release

FROM debian:bullseye-slim
# install ca certificates
RUN USER=root apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
# create config file from build argument
ARG CONFIG_JSON_KEY
RUN echo ${CONFIG_JSON_KEY}
RUN if [ -n "${CONFIG_JSON_KEY}" ]; then echo ${CONFIG_JSON_KEY} | base64 --decode > ./config.json; fi
# copy the application from the build stage
COPY --from=build /suuntaava_projekti/target/release/suuntaava_projekti usr/src/suuntaava_projekti

CMD ["/usr/src/suuntaava_projekti"]
