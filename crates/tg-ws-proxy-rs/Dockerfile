# syntax=docker/dockerfile:1.7

FROM rust:1-alpine AS builder

WORKDIR /app

RUN apk add --no-cache ca-certificates cmake g++ make musl-dev perl pkgconf

ARG TARGETPLATFORM

# The Android JNI shim is a workspace member, so cargo refuses to load the
# workspace without its manifest and a lib source -- even though
# `default-members` keeps it out of every build in this image.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/android-jni/Cargo.toml ./crates/android-jni/Cargo.toml

RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=target-${TARGETPLATFORM},target=/app/target \
    mkdir -p src crates/android-jni/src && \
    printf 'fn main() {}\n' > src/main.rs && \
    printf '// lib\n' > src/lib.rs && \
    printf '// lib\n' > crates/android-jni/src/lib.rs && \
    cargo build --release --locked && \
    rm -rf src crates/android-jni/src

COPY src ./src
COPY crates ./crates

# `touch` before building, and it is load-bearing. `COPY` preserves the build
# context's mtimes, and a fresh checkout's are older than the dependency layer
# above, which ran minutes later inside this build. Cargo decides freshness by
# mtime, so it compares the real sources against the stub's artifacts, calls
# everything up to date, does nothing, and leaves `fn main() {}` sitting in
# target/release/tg-ws-proxy for the `cp` below to ship. That is what happened
# to 2.3.0: a 300 KB image whose entrypoint exits 0 without listening.
#
# The trap only springs when the layer above actually executes rather than
# coming from cache, which is why it stayed hidden until a release changed
# Cargo.toml, Cargo.lock and that layer's command all at once.
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=target-${TARGETPLATFORM},target=/app/target \
    find src crates -name '*.rs' -exec touch {} + && \
    cargo build --release --locked && \
    cp target/release/tg-ws-proxy /usr/local/bin/tg-ws-proxy

# The stub is a valid binary that exits 0, so no later step -- not the COPY into
# scratch, not the image size, not a smoke test that only checks the container
# starts -- can tell it from the real one. Ask the binary what it is, and fail
# the build here rather than on someone's router.
RUN expected="tg-ws-proxy $(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)" && \
    actual="$(/usr/local/bin/tg-ws-proxy --version)" && \
    if [ "$actual" != "$expected" ]; then \
        echo "built binary reports '$actual', expected '$expected'" >&2; \
        exit 1; \
    fi

FROM scratch AS runtime

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /usr/local/bin/tg-ws-proxy /usr/local/bin/tg-ws-proxy

COPY <<EOF /etc/passwd
tgws:x:1000:1000:tg-ws-proxy user:/nonexistent:/sbin/nologin
EOF
COPY <<EOF /etc/group
tgws:x:1000:
EOF

USER 1000:1000
EXPOSE 1443

ENTRYPOINT ["tg-ws-proxy"]
