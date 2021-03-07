# Rust GraphQL Server

This is a relatively simple Rust GraphQL API server with the following features:

1. A fully async GraphQl API that allows creating, querying and authentication of users.
2. Log in, log out and refresh endpoints for authentication.
3. Email verification for users.
4. Can be built into a relatively small Docker container.
5. Communicates with a Postgres database for data persistence.
6. Communicates with a Redis database for session management and email verification.
7. Compile time verification of SQL queries via the `sqlx` crate.

# Initial Setup

1. Install Rust with `rustup` on your machine.
2. Switch to `nightly` rust using `rustup`.
3. Install the `sqlx` command line utility:

   ```sh
   cargo install sqlx-cli
   ```

4. Install Docker and `docker-compose`.
5. Start Docker.
6. Start Postgres and Redis databases:

    ```sh
    docker-compose up --build db redis
    ```

7. Build and run the server:

    ```sh
    cargo run
    ```

    The server runs all pending database migrations on startup, so if the server starts you should be good to go.

# Development Workflow

1. Start the Postgres and Redis databases by running:

   ```sh
   docker-compose up --build db redis
   ```

2. Compile and run the server in development mode:

   ```sh
   cargo run dev
   ```

   The `dev` command will regenerate the `schema.gql` file in the project's root directory on startup, then run the server as normal.

   The schema is automatically generated from Rust code. If you just want to update the schema without running the server, run:

   ```sh
   cargo run generate
   ```

   If you want to auto-recompile and restart the server on every code change, make sure `cargo-watch` is installed and run:

   ```sh
   cargo watch -x "run dev"
   ```

3. You should be able to access `http://localhost:8080/graphql` using your GraphQL client of choice.

   If you update or add any `sqlx` queries you'll get a compile error as, by default, the .env file has `SQLX_OFFLINE=true` set. To fix the compilation error, run:

   ```sh
   cargo sqlx prepare
   ```

   This will compare your SQL queries with the running database to update `sqlx-data.json` with new query information. If your queries are valid, the compile error will go away.

# Building as a Docker Container

1. To build the server into a Docker container and start it, run:

    ```sh
    docker-compose up --build
    ```

    This may take a bit. Rust compile times admittedly aren't great and this project has quite a few dependencies.

    However, repeated builds on your machine should be cached and any changes to the source code should only cause recompilation of this crate and not the dependencies.

2. The GraphQL API should be available at: `http://localhost:8080/graphql`.

# Possible Future Work

* Add endpoints requiring authentication.
* Provide Relay-style paginated endpoints.
* Improve GraphQL error handling.
* Use dataloaders to reduce any N+1 problems.