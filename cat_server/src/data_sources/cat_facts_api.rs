use anyhow::Context;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;

pub struct CatFactsApi {
    client: ClientWithMiddleware,
    base_url: String,
}

impl CatFactsApi {
    pub fn new(base_url: String, client: ClientWithMiddleware) -> Self {
        Self { client, base_url }
    }

    pub async fn get_fact(&self) -> Result<String, anyhow::Error> {
        // For an example, see: https://catfact.ninja/fact
        #[derive(Deserialize)]
        struct ResponseModel {
            pub fact: String,
        }

        let response = self
            .client
            .get(format!("{}/fact", self.base_url))
            .send()
            .await
            .context("Failed to make request")?
            .json::<ResponseModel>()
            .await
            .context("Invalid response returned")?;

        Ok(response.fact)
    }
}
