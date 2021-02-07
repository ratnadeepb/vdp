# A two stage build
FROM rust:1.49 AS build

WORKDIR /usr/src
RUN rustup target add x86_64-unknown-linux-musl

RUN USER=root cargo new async_client

# Build the dependencies
WORKDIR /usr/src/fdpass-rs
COPY fdpass-rs/Cargo.lock fdpass-rs/Cargo.toml ./
COPY fdpass-rs/src/ ./src/
RUN cargo build

# Build the app
WORKDIR /usr/src/async_client

COPY async_client/Cargo.lock async_client/Cargo.toml ./
COPY async_client/src/ ./src/

RUN cargo install --target x86_64-unknown-linux-musl --path .

# Copy the statically-linked binary into a scratch container
FROM scratch
COPY --from=build /usr/local/cargo/bin/async_client .
USER 1000

CMD ["./async_client"]