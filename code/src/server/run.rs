use crate::data_sources::cat_facts_api::CatFactsApi;
use crate::data_sources::cat_images_api::CatImagesApi;
use crate::Configuration;
use actix_web::dev::Server;
use actix_web::web::{get, Data};
use actix_web::{App, HttpServer};
use anyhow::Context;
use reqwest::ClientBuilder;
use std::net::TcpListener;

use super::get_cat_route;

pub async fn run_server(
    config: Configuration,
    listener: TcpListener,
) -> Result<Server, anyhow::Error> {
    let client = ClientBuilder::new()
        .build()
        .context("Failed to build http client")?;

    let cat_facts_api = Data::new(CatFactsApi::new(
        config.cat_facts_api_base_url,
        client.clone(),
    ));

    let cat_images_api = Data::new(CatImagesApi::new(config.cat_images_api_base_url, client));

    let server = HttpServer::new(move || {
        App::new()
            .app_data(cat_images_api.clone())
            .app_data(cat_facts_api.clone())
            .route("/cat", get().to(get_cat_route::handler))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
