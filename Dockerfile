FROM rust:1.88 as builder

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y wget lsb-release gnupg software-properties-common && \
    wget https://apt.llvm.org/llvm.sh && \
    chmod +x llvm.sh && \
    ./llvm.sh 18 && \
    rm llvm.sh

RUN apt-get install -y libpolly-18-dev

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y build-essential

COPY --from=builder /usr/src/app/target/release/ratio ./
COPY --from=builder /usr/src/app/input.ratio ./
COPY --from=builder /usr/lib/llvm-18/lib/libLLVM-18.so* ./

COPY run.sh ./
RUN chmod +x run.sh
RUN ldd ratio

CMD ["./run.sh"]

