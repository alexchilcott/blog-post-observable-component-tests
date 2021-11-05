use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockCatImagesApi(MockServer);

impl MockCatImagesApi {
    pub async fn new() -> Self {
        Self(MockServer::builder().start().await)
    }

    pub fn base_url(&self) -> String {
        self.0.uri()
    }

    pub async fn configure_cat_image_url(&self) -> String {
        let url = format!("http://my-cat-pictures.com/{}.jpg", Uuid::new_v4());
        Mock::given(method("GET"))
            .and(path("/v1/images/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{ "url": url }])))
            .mount(&self.0)
            .await;
        url
    }
}
