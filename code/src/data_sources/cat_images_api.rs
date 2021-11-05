use anyhow::{anyhow, Context};
use serde::Deserialize;

pub struct CatImagesApi {
    client: reqwest::Client,
    base_url: String,
}

impl CatImagesApi {
    pub fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { client, base_url }
    }

    pub async fn get_image_url(&self) -> Result<String, anyhow::Error> {
        // For an example, see: https://api.thecatapi.com/v1/images/search
        #[derive(Deserialize)]
        struct ImageModel {
            pub url: String,
        }

        let response = self
            .client
            .get(format!("{}/v1/images/search", self.base_url))
            .send()
            .await
            .context("Failed to make request")?
            .json::<Vec<ImageModel>>()
            .await
            .context("Invalid response returned")?;

        let first_image = response
            .get(0)
            .ok_or_else(|| anyhow!("Empty array of results returned"))?;

        Ok(first_image.url.to_owned())
    }
}
