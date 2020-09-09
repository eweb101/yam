FROM rust:1.45.2 as builder
WORKDIR /usr/src

RUN USER=root cargo new yam
WORKDIR /usr/src/yam
COPY Cargo.toml Cargo.lock ./
RUN set -x\
 && mkdir -p src\
 && echo "fn main() {println!(\"broken\")}" > src/main.rs\
 && echo "//This is nothing" > src/lib.rs\
 && cargo build --release

COPY src ./src
RUN set -x\
 && touch src/*.rs\
 && cargo install --locked --path .

FROM debian:buster-slim
RUN apt-get update \
    && apt-get install -y libssl-dev\
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/yam /usr/local/bin/yam
CMD ["boat_maintenance"]


