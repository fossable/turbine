FROM rust as builder

RUN USER=root cargo new --bin turbine
WORKDIR /turbine
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD src ./src
ADD templates ./templates
ADD assets ./assets

RUN rm ./target/release/deps/turbine*
RUN cargo build --release --all-features

FROM debian:bookworm-slim
ARG APP=/app
EXPOSE 80
ENV TZ=Etc/UTC

RUN apt-get update \
    && apt-get install -y bzip2 ca-certificates curl tzdata \
    && rm -rf /var/lib/apt/lists/*

# Install monero wallet RPC daemon
RUN curl https://downloads.getmonero.org/cli/monero-linux-x64-v0.18.3.3.tar.bz2 | tar --transform='flags=r;s|.*/||' -xjf - -C /usr/bin monero-x86_64-linux-gnu-v0.18.3.3/monero-wallet-rpc

# Install turbine
COPY --from=builder /turbine/target/release/turbine ${APP}/turbine

WORKDIR ${APP}
ENTRYPOINT ["/app/turbine", "serve"]
