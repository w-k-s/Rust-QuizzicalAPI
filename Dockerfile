FROM rust:1.26.2

COPY . /app
WORKDIR /app
RUN cargo install
CMD cargo run --release
