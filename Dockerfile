# Planner
FROM lukemathwalker/cargo-chef as planner
RUN rustup default nightly

WORKDIR /amble
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

# Cacher
FROM lukemathwalker/cargo-chef as cacher
RUN rustup default nightly

WORKDIR /amble
COPY --from=planner /amble/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Builder
FROM rust as builder
RUN rustup default nightly

WORKDIR /amble
COPY --from=cacher /amble/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin amble

# Runtime
FROM rust as runtime

WORKDIR /amble
COPY --from=builder /amble/.env .env
COPY --from=builder /amble/.env.override .env.override
COPY --from=builder /amble/target/release/amble /usr/local/bin
ENV IS_DOCKER true
ENTRYPOINT ["/usr/local/bin/amble"]