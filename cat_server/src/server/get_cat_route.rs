use actix_web::{web, HttpResponse, Responder};
use anyhow::Context;
use serde::Serialize;
use tracing::instrument;

use crate::data_sources::cat_facts_api::CatFactsApi;
use crate::data_sources::cat_images_api::CatImagesApi;

#[derive(Serialize)]
struct CatFactAndPicture {
    pub fact: String,
    pub image_url: String,
}

#[instrument(skip(cat_facts_api, cat_images_api))]
async fn get_cat_fact_and_image(
    cat_facts_api: &CatFactsApi,
    cat_images_api: &CatImagesApi,
) -> Result<CatFactAndPicture, anyhow::Error> {
    let fact = cat_facts_api
        .get_fact()
        .await
        .context("Failed to get a cat fact")?;

    let image_url = cat_images_api
        .get_image_url()
        .await
        .context("Failed to get a cat image url")?;

    Ok(CatFactAndPicture { fact, image_url })
}

#[instrument(skip(cat_facts_api, cat_images_api))]
pub async fn handler(
    cat_facts_api: web::Data<CatFactsApi>,
    cat_images_api: web::Data<CatImagesApi>,
) -> impl Responder {
    get_cat_fact_and_image(&cat_facts_api, &cat_images_api)
        .await
        .map_or_else(
            |error| HttpResponse::InternalServerError().body(format!("{:?}", error)),
            |fact| HttpResponse::Ok().json(fact),
        )
        .await
}
