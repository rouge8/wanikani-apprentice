FROM rust:1.62-slim-bullseye AS builder

WORKDIR /src/wanikani-apprentice
COPY . .

RUN cargo install --path .

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/wanikani-apprentice /usr/local/bin/wanikani-apprentice
CMD ["wanikani-apprentice"]
