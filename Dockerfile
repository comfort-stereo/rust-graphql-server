FROM rust

RUN rustup default nightly

WORKDIR /usr/src/amble

COPY . .

ENV IS_DOCKER true
ENV SQLX_OFFLINE true
RUN cargo install --path . --locked

CMD ["amble"]