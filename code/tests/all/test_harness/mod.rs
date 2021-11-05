mod mocks;

use self::mocks::{MockCatFactsApi, MockCatImagesApi};
use black_box_cat_api::{run_server, Configuration};
use std::net::TcpListener;

pub struct TestHarness {
    pub client: reqwest::Client,
    pub config: Configuration,
    pub mock_cat_images_api: MockCatImagesApi,
    pub mock_cat_facts_api: MockCatFactsApi,
}

impl TestHarness {
    pub async fn start() -> Self {
        let mock_cat_images_api = MockCatImagesApi::new().await;
        let mock_cat_facts_api = MockCatFactsApi::new().await;

        let host = "127.0.0.1";
        let listener = TcpListener::bind(format!("{}:0", host)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let config = Configuration {
            host: host.into(),
            port,
            cat_images_api_base_url: mock_cat_images_api.base_url(),
            cat_facts_api_base_url: mock_cat_facts_api.base_url(),
        };

        let server = run_server(config.clone(), listener);

        let _join_handle = tokio::spawn(server);

        let client = reqwest::ClientBuilder::new()
            .build()
            .expect("Failed to build http client");

        TestHarness {
            client,
            config,
            mock_cat_images_api,
            mock_cat_facts_api,
        }
    }

    pub fn build_url(&self, relative_path: impl Into<String>) -> String {
        format!(
            "http://{}:{}{}",
            self.config.host,
            self.config.port,
            relative_path.into()
        )
    }
}
