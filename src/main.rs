mod auth;
mod config;
mod context;
mod db;
mod executor;
mod models;
mod schema;
mod state;

use std::process::Command;

use anyhow::Result;
use clap::{App, ArgMatches, SubCommand};
use juniper::http::GraphQLRequest;
use tide::{http::mime, log, Body, Request, Response, Server, StatusCode};

use config::Config;
use context::Context;
use db::{connect_to_db, connect_to_redis};
use schema::SCHEMA;
use state::State;

async fn graphql(mut request: Request<State>) -> tide::Result {
    let query: GraphQLRequest = request.body_json().await?;
    let context = Context::new(request).await;
    let response = query.execute(&SCHEMA, &context).await;
    let status = if response.is_ok() {
        StatusCode::Ok
    } else {
        StatusCode::BadRequest
    };

    let body = Body::from_json(&response)?;
    let response = Response::builder(status)
        .content_type(mime::JSON)
        .body(body);

    Ok(response.build())
}

fn parse_args() -> ArgMatches<'static> {
    App::new("Amble")
        .version("0.1.0")
        .subcommand(SubCommand::with_name("generate"))
        .get_matches()
}

fn generate() {
    log::info!("Writing generated files...");

    {
        log::info!("Writing schema.gql...");
        std::fs::write("./schema.gql", SCHEMA.as_schema_language())
            .expect("Failed to write schema.gql.");
    }

    {
        log::info!("Running sqlx migrations...");
        Command::new("cargo")
            .arg("sqlx")
            .arg("migrate")
            .arg("run")
            .output()
            .expect("Failed to run sqlx migrations. To run migrations, the database needs to be running and sqlx-cli must be installed.");

        log::info!("Writing sqlx-data.json...");
        Command::new("cargo")
            .arg("sqlx")
            .arg("prepare")
            .output()
            .expect("Failed to write sqlx-data.json, To get this data, the database needs to be running and sqlx-cli must be installed.");
    }

    log::info!("Done");
}

/// Run the server with the provided configuration settings.
async fn run(config: Config) -> Result<()> {
    log::debug!("Running with config: {:#?}", config);

    log::info!("Connecting to database...");
    let db = connect_to_db(&config).await?;
    log::info!("Connecting to redis...");
    let redis = connect_to_redis(&config).await?;

    let mut server = Server::with_state(State::new(config.clone(), db, redis));
    server.at("/graphql").post(graphql);
    server.listen(format!("0.0.0.0:{}", &config.port)).await?;

    Ok(())
}

#[async_std::main]
async fn main() -> Result<()> {
    log::start();

    // Parse configuration from environment variables and .env files.
    let config = Config::load().await;

    // Parse command line arguments.
    let args = parse_args();
    if args.subcommand_matches("generate").is_some() {
        // If the second argument is "generate", just run codegen and exit.
        generate();
    } else {
        // If no sub-command was provided, start the server.
        run(config).await?;
    }

    Ok(())
}
