FROM rust:1.62-slim-bullseye AS builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends pkg-config libssl-dev git \
  && apt-get clean

WORKDIR /src/wanikani-apprentice
COPY . .

RUN cargo install --locked --path .

FROM debian:bullseye-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && apt-get clean
COPY --from=builder /usr/local/cargo/bin/wanikani-apprentice /usr/local/bin/wanikani-apprentice
WORKDIR /app
COPY static static
COPY templates templates
CMD ["wanikani-apprentice"]
