FROM fossable/turbine:latest

COPY gpg_setup.sh .
RUN ./gpg_setup.sh

# Replace the entrypoint with one that starts several turbine instances
COPY entrypoint.sh /
ENTRYPOINT [ "/entrypoint.sh" ]
