FROM rust as build

WORKDIR /build
COPY . .
RUN cargo build --release --all-features

FROM debian:bookworm-slim
ENV TZ=Etc/UTC
EXPOSE 80
ARG BUILDARCH

RUN apt-get update \
    && apt-get install -y bzip2 ca-certificates curl tzdata git gpg \
    && rm -rf /var/lib/apt/lists/*

# Install monero wallet RPC daemon
RUN curl https://downloads.getmonero.org/cli/monero-linux-x64-v0.18.3.3.tar.bz2 | tar --transform='flags=r;s|.*/||' -xjf - -C /usr/bin monero-x86_64-linux-gnu-v0.18.3.3/monero-wallet-rpc

# Install turbine
COPY --from=build /build/target/release/turbine /usr/bin/turbine

ENTRYPOINT ["/usr/bin/turbine", "serve"]
