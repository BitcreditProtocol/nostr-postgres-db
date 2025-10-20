FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Install additional tools
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        postgresql-client \
        git \
        curl \
        build-essential \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Install cargo tools
RUN cargo install cargo-watch cargo-edit

# Set the default user
USER vscode
