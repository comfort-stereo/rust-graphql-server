# Rust GraphQL Server

This is a relatively simple Rust GraphQL API server with the following features:

1. A fully async GraphQl API that allows creating, querying and authentication of users.
6. Log in, log out and refresh endpoints for authentication.
2. Email verification for users.
3. Can be built into a relatively small Docker container.
4. Communicates with a Postgres database for data persistence.
5. Communicates with a Redis database for session management and email verification.
6. Compile time verification of SQL queries via the "sqlx" crate.

# Initial Setup

1. Install Rust with "rustup" on your machine.
2. Switch to "nightly" rust using "rustup".
3. Install the "sqlx" command line utility by running:
   ```
   cargo install sqlx-cli
   ```
4. Install Docker and "docker-compose".
5. Start Docker.
6. Start the Postgres and Redis databases by running:

    ```
    docker-compose up --build db redis
    ```

# Development Workflow

1. Start the Postgres and Redis databases by running:

   ```
   docker-compose up --build db redis
   ```

2. Compile and run the server by running:

   ```
   cargo run
   ```

3. The server will run all pending migrations on startup. You should be able to access the "http://localhost:8080/graphql" endpoint using your GraphQL client of choice.

   Make changes to the source code, compile and re-run the server when you want to test.

   If you make changes that affect the GraphQL schema, make sure to run the codegen command to update the "schema.gql" file:

   ```
   cargo run generate
   ```

   If you update or add any sqlx queries you'll get a compile error as, by default, the .env file has "SQLX_OFFLINE=true" set. To fix this, run:

   ```
   cargo sqlx prepare
   ```

   This will compare your SQL queries with the running database to update the "sqlx-data.json" file with new query information. If your queries are valid, the compile errors will go away.

# Building as a Docker Container

1. To build the server into a Docker container and start it, run:

    ```
    docker-compose up --build
    ```

    This may take a while because Rust compile times admittedly aren't great and this project has quite a few dependencies. However, repeated builds on your machine should be cached and any changes to the source code (other than changes to "Cargo.toml") should only cause recompilation of this crate and not the dependencies.

2. The GraphQL API should be available at: "http://localhost:8080/graphql".

# Possible Future Work

* Add endpoints requiring authentication.
* Provide Relay-style paginated endpoints.
* Improve GraphQL error handling.
* Use dataloaders to reduce any N+1 problems.