FROM instrumentisto/rust:1.60.0-alpine3.15 AS builder
WORKDIR /build
COPY ./ /build
RUN apk update \
    && apk --no-cache add clang lld

RUN RUSTFLAGS='-C linker=clang -C link-arg=-fuse-ld=lld -C target-cpu=skylake' \
    cargo build --target x86_64-unknown-linux-musl --release --locked --path . --bin app

FROM grafana/alpine:3.15.4

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/app /app/app
COPY --from=builder /build/src/pages/templates/ /app/static/
COPY --from=builder /build/assets/ /app/static/

EXPOSE 3030
WORKDIR /app
ENTRYPOINT ["./app"]