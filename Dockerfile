FROM rust:1.65 AS builder
COPY . .
RUN apt-get update && apt install -y clang libclang1
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder ./target/release/snarkvm_abci ./target/release/snarkvm_abci
RUN apt-get update && apt install -y libcurl4
CMD ["/target/release/snarkvm_abci"]
