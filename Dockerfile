# Part 1: a builder image with the dependencies to actually build the
# project. Gigabytes on disk.
FROM rust:slim-bookworm AS builder

WORKDIR /

RUN apt update
RUN apt-get install -y \
    # ayb requirements
    libssl-dev \
    gcc \
    g++ \
    pkg-config

COPY . /ayb

RUN cd ayb && cargo build --release

# Part 2: the image with the binaries built by the builder and no
# unnecessary dependencies or build artifacts. Low hundreds of
# megabytes on disk.
FROM debian:bookworm-slim

RUN apt update
RUN apt-get install -y libssl-dev ca-certificates

COPY --from=builder /ayb/target/release/ayb /bin
COPY --from=builder /ayb/target/release/ayb_query_daemon /bin

EXPOSE 5433

CMD ["/bin/ayb", "server"]
