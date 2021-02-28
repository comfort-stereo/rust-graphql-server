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
        std::fs::write("./schema.gql", SCHEMA.as_schema_language()).expect("to write schema.gql");
    }

    {
        log::info!("Running sqlx migrations...");
        Command::new("cargo")
            .arg("sqlx")
            .arg("migrate")
            .arg("run")
            .output()
            .expect("to run sqlx migrations, the database needs to be running and sqlx-cli must be installed");

        log::info!("Writing sqlx-data.json...");
        Command::new("cargo")
            .arg("sqlx")
            .arg("prepare")
            .output()
            .expect("to write sqlx-data.json, the database needs to be running and sqlx-cli must be installed");
    }

    log::info!("Done");
}

async fn run(config: Config) -> Result<()> {
    log::debug!("Running with config: {:#?}", config);

    let db = connect_to_db(&config)
        .await
        .expect("to connect to the database");
    let redis = connect_to_redis(&config)
        .await
        .expect("to connect to redis");

    let mut server = Server::with_state(State::new(config.clone(), db, redis));
    server.at("/graphql").post(graphql);
    server.listen(format!("0.0.0.0:{}", &config.port)).await?;

    Ok(())
}

#[async_std::main]
async fn main() -> Result<()> {
    log::start();

    let config = Config::load().await;
    let args = parse_args();
    if args.subcommand_matches("generate").is_some() {
        generate();
    } else {
        run(config).await?;
    }

    Ok(())
}
