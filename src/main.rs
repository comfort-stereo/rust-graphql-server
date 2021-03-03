mod auth;
mod config;
mod context;
mod db;
mod executor;
mod models;
mod schema;
mod state;

use anyhow::Result;
use clap::{App, ArgMatches, SubCommand};
use juniper::http::GraphQLRequest;
use tide::{http::mime, log, Body, Request, Response, Server, StatusCode};

use config::Config;
use context::Context;
use db::{connect_to_db, connect_to_redis, run_migrations};
use schema::SCHEMA;
use state::State;

/// Handle a GraphQL request.
async fn graphql(mut request: Request<State>) -> tide::Result {
    // Attempt to parse the GraphQL query from the request.
    let query: GraphQLRequest = request.body_json().await?;
    // Initialize a context struct for the request. This context may include configuration,
    // connections to databases, authentication info, etc..
    let context = Context::new(request).await;
    // Execute the query using our GraphQL schema.
    let response = query.execute(&SCHEMA, &context).await;
    // If we get an error while executing the query, return a bad request status.
    let status = if response.is_ok() {
        StatusCode::Ok
    } else {
        StatusCode::BadRequest
    };

    // Build and return the response.
    let response = Response::builder(status)
        .content_type(mime::JSON)
        .body(Body::from_json(&response)?);

    Ok(response.build())
}

/// Parse command line arguments for the server.
fn parse_args() -> ArgMatches<'static> {
    App::new("rust-graphql-server")
        .version("0.1.0")
        .subcommand(SubCommand::with_name("generate"))
        .subcommand(SubCommand::with_name("dev"))
        .get_matches()
}

/// Write generated files. At the moment this only includes the GraphQL schema.
fn generate() {
    log::info!("Writing generated files...");

    // Write the derived GraphQL schema.
    {
        log::info!("Writing schema.gql...");
        std::fs::write("./schema.gql", SCHEMA.as_schema_language())
            .expect("Failed to write schema.gql.");
    }

    log::info!("Done");
}

/// Run the server with the provided configuration settings.
async fn run(config: Config) -> Result<()> {
    log::debug!("Running with config: {:#?}", config);

    log::info!("Connecting to Postgres database...");
    let db = connect_to_db(&config).await?;
    log::info!("Connecting to Redis database...");
    let redis = connect_to_redis(&config).await?;

    log::info!("Running any pending database migrations...");
    run_migrations(&db).await?;

    let mut server = Server::with_state(State::new(config.clone(), db, redis));
    server.at("/graphql").post(graphql);
    server.listen(format!("0.0.0.0:{}", &config.port)).await?;

    Ok(())
}

#[async_std::main]
async fn main() -> Result<()> {
    // Setup server logging.
    log::start();

    // Parse configuration from environment variables and .env files.
    let config = Config::load().await;

    // Parse command line arguments.
    let args = parse_args();
    if args.subcommand_matches("generate").is_some() {
        // If the second argument is "generate", write generated files and exit.
        generate();
    } else if args.subcommand_matches("dev").is_some() {
        // If the second argument is "dev", write generated files and start the server.
        generate();
        run(config).await?;
    } else {
        // If no sub-command was provided, start the server.
        run(config).await?;
    }

    Ok(())
}
