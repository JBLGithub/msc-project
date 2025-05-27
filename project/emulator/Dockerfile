FROM rust:latest
WORKDIR /app
RUN apt-get update && apt-get install -y strace && apt-get clean && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch
COPY src/ ./src/
RUN cargo build --release
RUN mkdir -p /app/config
RUN mkdir -p /app/logs
RUN chmod -R 777 /app/logs
ENTRYPOINT ["./target/release/emulator"]
