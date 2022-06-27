#
# Stage 'dist' creates project distribution.
#

# https://github.com/instrumentisto/rust-docker-image/pkgs/container/rust
ARG rust_ver=latest
FROM ghcr.io/instrumentisto/rust:${rust_ver} AS dist
ARG rustc_mode=release
ARG rustc_opts=--release

RUN mkdir /out

COPY api/ /app/api/
COPY lib/ /app/lib/
COPY e2e/ /app/e2e/
COPY src/ /app/src/
COPY Cargo.toml Cargo.lock /app/

WORKDIR /app/

# Build project distribution binary.
# TODO: use --out-dir once stabilized
# TODO: https://github.com/rust-lang/cargo/issues/6790
RUN cargo build ${rustc_opts}

# Prepare project distribution binary and all dependent dynamic libraries.
RUN cp /app/target/${rustc_mode}/baza /out/baza \
 && ldd /out/baza \
    | awk 'BEGIN{ORS=" "}$1~/^\//{print $1}$3~/^\//{print $3}' \
    | sed 's/,$/\n/' \
    | tr -d ':' \
    | tr ' ' "\n" \
    | xargs -I '{}' cp -fL --parents '{}' /out/ \
 && rm -rf /out/out

#
# Stage 'runtime' creates final Docker image to use in runtime.
#

# https://hub.docker.com/_/scratch
FROM alpine:latest AS runtime

COPY --from=dist /out/ /

ENTRYPOINT ["ls", "-la"]
