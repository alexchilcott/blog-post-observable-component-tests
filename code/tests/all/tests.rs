use crate::api_models::CatFactAndPicture;
use crate::test_harness::TestHarness;

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
