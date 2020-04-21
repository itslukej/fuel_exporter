FROM ekidd/rust-musl-builder as builder

WORKDIR /home/rust/

COPY Cargo.toml .
COPY Cargo.lock .
RUN echo "fn main() {}" > src/main.rs
RUN cargo test
RUN cargo build --release

COPY . .
RUN sudo touch src/main.rs

RUN cargo test
RUN cargo build --release

RUN strip target/x86_64-unknown-linux-musl/release/fuel_exporter

FROM alpine
WORKDIR /home/rust/

RUN apk add -U --no-cache ca-certificates

COPY --from=builder /home/rust/target/x86_64-unknown-linux-musl/release/fuel_exporter .
ENTRYPOINT ["./fuel_exporter"]
