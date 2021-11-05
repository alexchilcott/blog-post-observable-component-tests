use crate::api_models::CatFactAndPicture;
use crate::test_harness::TestHarness;
use prometheus_parse::{Scrape, Value};

#[actix_rt::test]
pub async fn cat_endpoint_retrieves_fact_and_image_url_from_apis_and_returns_them_on_response() {
    // Arrange
    // Set up pre-conditions for a successful call to /cat
    let test_harness = TestHarness::start().await;
    let cat_image_url = test_harness
        .mock_cat_images_api
        .configure_cat_image_url()
        .await;
    let cat_fact = test_harness.mock_cat_facts_api.configure_cat_fact().await;

    // Act
    // Call the /cat endpoint and parse the response
    let response_body = test_harness
        .client
        .get(test_harness.build_url("/cat"))
        .send()
        .await
        .expect("Failed to make request to server")
        .json::<CatFactAndPicture>()
        .await
        .expect("Failed to deserialize body");

    // Assert
    // Check the response contains the expected fact and url
    assert_eq!(response_body.fact, cat_fact);
    assert_eq!(response_body.image_url, cat_image_url);
}

#[actix_rt::test]
pub async fn metrics_endpoint_after_successfully_handling_cat_request_returns_correct_http_requests_total_metric(
) {
    // Arrange
    // Set up pre-conditions for a successful call to /cat, then make the
    // call, so our server has received a single call to /cat
    let test_harness = TestHarness::start().await;
    test_harness
        .mock_cat_images_api
        .configure_cat_image_url()
        .await;
    test_harness.mock_cat_facts_api.configure_cat_fact().await;

    test_harness
        .client
        .get(test_harness.build_url("/cat"))
        .send()
        .await
        .expect("Failed to make request to server")
        .error_for_status()
        .expect("Server returned an error status code");

    // Act
    // Call the /metrics endpoint
    let response = test_harness
        .client
        .get(test_harness.build_url("/metrics"))
        .send()
        .await
        .expect("Failed to make request to server");

    // Assert
    // Parse the body of the response from the /metrics endpoint
    let metrics = Scrape::parse(
        response
            .error_for_status()
            .expect("Server returned an error status code")
            .text()
            .await
            .expect("Failed to read body")
            .lines()
            .map(|line| Ok(line.to_owned())),
    )
    .expect("Failed to parse prometheus response");

    // Then check the `http_requests_total` metric
    let sample = metrics
        .samples
        .iter()
        .find(|sample| {
            sample.metric == "http_requests_total"
                && sample.labels.get("endpoint") == Some("/cat")
                && sample.labels.get("method") == Some("GET")
                && sample.labels.get("status") == Some("200")
        })
        .expect(r#"No http_requests_total sample found for "/cat" endpoint"#);

    assert_eq!(sample.value, Value::Counter(1f64));
}
