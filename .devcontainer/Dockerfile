FROM ubuntu:26.04

ARG USERNAME=ubuntu

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        libudev-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

USER ${USERNAME}

ENV PATH=/home/${USERNAME}/.cargo/bin:${PATH}

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup component add clippy rustfmt
