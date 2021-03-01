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
2. Install the "sqlx" command line utility by running:
   ```
   cargo install sqlx-cli
   ```
3. Install Docker and "docker-compose".
4. Start the Postgres and Redis databases by running:

    ```
    docker-compose up --build db redis
    ```
5. Run migrations for the database by running:
    ```
    cargo sqlx migrate run
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

3. You should be able to access the "http://localhost:8080/graphql" endpoint using your GraphQL client of choice. Make changes to the source code and re-run the server when you want to test your changes.

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
* Have the server run migrations on startup, this is currently blocked by an issue I've noticed with "sqlx" where it falsely detects changed migrations.