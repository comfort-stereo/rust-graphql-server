# Planner
FROM lukemathwalker/cargo-chef as planner
RUN rustup default nightly

WORKDIR /rust-graphql-server
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

# Cacher
FROM lukemathwalker/cargo-chef as cacher
RUN rustup default nightly

WORKDIR /rust-graphql-server
COPY --from=planner /rust-graphql-server/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Builder
FROM rust as builder
RUN rustup default nightly

WORKDIR /rust-graphql-server
COPY --from=cacher /rust-graphql-server/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin rust-graphql-server

# Runtime
FROM rust as runtime

WORKDIR /rust-graphql-server
COPY --from=builder /rust-graphql-server/.env .env
COPY --from=builder /rust-graphql-server/.env.override .env.override
COPY --from=builder /rust-graphql-server/target/release/rust-graphql-server /usr/local/bin
ENV IS_DOCKER true
ENTRYPOINT ["/usr/local/bin/rust-graphql-server"]