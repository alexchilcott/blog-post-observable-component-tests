use serde_json::json;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockCatFactsApi(MockServer);

impl MockCatFactsApi {
    pub async fn new() -> Self {
        Self(MockServer::builder().start().await)
    }

    pub fn base_url(&self) -> String {
        self.0.uri()
    }

    pub async fn configure_cat_fact(&self) -> String {
        let fact = format!("This cat is called '{}'.", Uuid::new_v4());
        Mock::given(method("GET"))
            .and(path("/fact"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "fact": fact })))
            .mount(&self.0)
            .await;
        fact
    }
}
