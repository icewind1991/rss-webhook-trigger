FROM ekidd/rust-musl-builder AS build

COPY Cargo.toml Cargo.lock ./

# Build with a dummy main to pre-build dependencies
RUN mkdir src && \
 echo "fn main(){}" > src/main.rs && \
 cargo build --release && \
 rm -r src

COPY src/* ./src/
RUN sudo chown -R rust:rust . && touch src/main.rs

RUN cargo build --release

FROM scratch

COPY --from=build /home/rust/src/target/x86_64-unknown-linux-musl/release/rss-webhook-trigger /

CMD ["/rss-webhook-trigger"]