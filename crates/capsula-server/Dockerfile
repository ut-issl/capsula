FROM rust:1.90 as builder
WORKDIR /usr/src/capsula-server
COPY . .
RUN cargo install --path .

FROM debian:stable-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/capsula-server /usr/local/bin/capsula-server
EXPOSE 80800
CMD ["capsula-server"]
