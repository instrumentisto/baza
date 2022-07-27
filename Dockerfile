#
# Stage `dist` creates project distribution.
#

# https://github.com/instrumentisto/rust-docker-image/pkgs/container/rust
ARG rust_ver=latest
FROM ghcr.io/instrumentisto/rust:${rust_ver} AS dist
ARG rustc_mode=release
ARG rustc_opts=--release

# Create the user and group files that will be used in the running container to
# run the process as an unprivileged user.
RUN mkdir -p /out/etc/ \
 && echo 'nobody:x:65534:65534:nobody:/:' > /out/etc/passwd \
 && echo 'nobody:x:65534:' > /out/etc/group

# Prepare Cargo workspace for building dependencies only.
COPY api/s3/Cargo.toml /app/api/s3/
COPY lib/Cargo.toml /app/lib/
COPY e2e/Cargo.toml /app/e2e/
COPY Cargo.toml Cargo.lock README.md /app/
WORKDIR /app/
RUN mkdir -p api/s3/src/ && touch api/s3/src/lib.rs \
 && mkdir -p lib/src/ && touch lib/src/lib.rs \
 && mkdir -p e2e/src/ && touch e2e/src/lib.rs \
 && mkdir -p src/ && touch src/lib.rs

# Build dependencies only.
RUN cargo build -p baza --lib ${rustc_opts}
# Remove fingreprints of pre-built empty project sub-crates
# to rebuild them correctly later.
RUN rm -rf /app/target/${rustc_mode}/.fingerprint/baza-* \
           /app/target/${rustc_mode}/.fingerprint/baza \
           /app/src/lib.rs

# Prepare project sources for building.
COPY api/s3/ /app/api/s3/
COPY lib/ /app/lib/
COPY src/ /app/src/

# Build project distribution binary.
# TODO: Use `--out-dir` once stabilized:
#       https://github.com/rust-lang/cargo/issues/6790
RUN cargo build -p baza ${rustc_opts}

# Prepare project distribution binary and all dependent dynamic libraries.
RUN cp /app/target/${rustc_mode}/baza /out/baza \
 && ldd /out/baza \
        # These libs are not reported by ldd(1) on binary,
        # but are vital for DNS resolution.
        # See: https://forums.aws.amazon.com/thread.jspa?threadID=291609
        /lib/$(uname -m)-linux-gnu/libnss_dns.so.2 \
        /lib/$(uname -m)-linux-gnu/libnss_files.so.2 \
    | awk 'BEGIN{ORS=" "}$1~/^\//{print $1}$3~/^\//{print $3}' \
    | sed 's/,$/\n/' \
    | tr -d ':' \
    | tr ' ' "\n" \
    | xargs -I '{}' cp -fL --parents '{}' /out/ \
 && rm -rf /out/out




#
# Stage `runtime` creates final Docker image to use in runtime.
#

# https://hub.docker.com/_/scratch
FROM scratch AS runtime

COPY --from=dist /out/ /

USER nobody:nobody

ENTRYPOINT ["/baza"]
