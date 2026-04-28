# Build capnpc from Rust source
FROM rust:1.81-alpine AS capnpc-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY capnp/ capnp/
COPY capnpc/ capnpc/
# Limit workspace to only needed crates
RUN sed -i '/^members/,/^\]/c\members = ["capnp", "capnpc"]' Cargo.toml
RUN cargo build --release -p capnpc

# Build capnp from C++ source
FROM alpine:3.21 AS capnp-builder

RUN apk add --no-cache \
    build-base \
    cmake \
    curl \
    linux-headers

RUN curl -fSL -O "https://capnproto.org/capnproto-c++-1.0.1.tar.gz" \
  && tar zxf "capnproto-c++-1.0.1.tar.gz"

RUN cd "capnproto-c++-1.0.1" \
  && cmake -B build \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/opt/capnp \
    -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_EXE_LINKER_FLAGS="-static" \
  && cmake --build build -j"$(nproc)" \
  && cmake --install build

ENV PATH="/opt/capnp/bin:${PATH}"

FROM alpine:3.21

COPY --from=capnpc-builder /app/target/release/capnpc-rust /usr/local/bin/
COPY --from=capnpc-builder /app/target/release/capnpc-rust-bootstrap /usr/local/bin/
COPY --from=capnp-builder /opt/capnp/bin/capnp /usr/local/bin/
