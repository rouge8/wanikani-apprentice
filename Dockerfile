FROM rust:slim-bookworm AS chef
WORKDIR /src
RUN cargo install cargo-chef

# Determine dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build dependencies
FROM chef AS builder
RUN apt-get update \
  && apt-get install -y --no-install-recommends pkg-config libssl-dev git \
  && apt-get clean
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
RUN git config --global --add safe.directory /src
COPY . .
RUN cargo install --locked --path .

# Bundle the application
FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && apt-get clean
COPY --from=builder /usr/local/cargo/bin/wanikani-apprentice /usr/local/bin/wanikani-apprentice
WORKDIR /app
CMD ["wanikani-apprentice"]
