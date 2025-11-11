FROM debian:bookworm-slim AS monero_setup

COPY monero_setup.sh .
RUN ./monero_setup.sh

FROM fossable/turbine:latest

COPY gpg_setup.sh .
RUN ./gpg_setup.sh

COPY entrypoint.sh /
ENTRYPOINT [ "/entrypoint.sh" ]
