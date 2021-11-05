use crate::api_models::CatFactAndPicture;
use crate::test_harness::TestHarness;
use crate::utilities::retry_loop::{retry_until_ok, RetryTimeoutError};
use crate::utilities::span_extensions::SpanExt;
use anyhow::{anyhow, Context};
use mock_jaeger_collector::{
    jaeger_models::{Span, TagValue},
    DetachedJaegerCollectorServer,
};
use opentelemetry::global::force_flush_tracer_provider;
use prometheus_parse::{Scrape, Value};
use rctree::Node;
use reqwest::StatusCode;
use std::time::Duration;
use tracing::info_span;
use tracing_futures::Instrument;

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
    // Set up pre-conditions for a successful request to /cat,
    // then send the request to the server.
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
        .expect("Failed to make request to server")
        .error_for_status()
        .expect("Server returned an error status code");

    // Assert
    // Parse the body of the response from the /metrics endpoint
    let metrics = Scrape::parse(
        response
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
        .expect(r#"No matching http_requests_total sample found for "/cat" endpoint"#);

    assert_eq!(sample.value, Value::Counter(1.into()));
}

#[actix_rt::test]
pub async fn metrics_endpoint_after_unsuccessfully_handling_cat_request_returns_correct_http_requests_total_metric(
) {
    // Arrange
    // Set up pre-conditions for a unsuccessful request to /cat,
    // then send the request to the server.
    let test_harness = TestHarness::start().await;
    test_harness.mock_cat_images_api.setup_failure().await;
    test_harness.mock_cat_facts_api.setup_failure().await;

    let status_code = test_harness
        .client
        .get(test_harness.build_url("/cat"))
        .send()
        .await
        .expect("Failed to make request to server")
        .status();
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);

    // Act
    // Call the /metrics endpoint
    let response = test_harness
        .client
        .get(test_harness.build_url("/metrics"))
        .send()
        .await
        .expect("Failed to make request to server")
        .error_for_status()
        .expect("Server returned an error status code");

    // Assert
    // Parse the body of the response from the /metrics endpoint
    let metrics = Scrape::parse(
        response
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
                && sample.labels.get("status") == Some("500")
        })
        .expect(r#"No matching http_requests_total sample found for "/cat" endpoint"#);

    assert_eq!(sample.value, Value::Counter(1.into()));
}

#[actix_rt::test]
pub async fn cat_endpoint_sends_a_trace_that_show_outgoing_http_calls() {
    // Arrange
    // Set up pre-conditions for a successful call to /cat
    let test_harness = TestHarness::start().await;
    test_harness
        .mock_cat_images_api
        .configure_cat_image_url()
        .await;
    test_harness.mock_cat_facts_api.configure_cat_fact().await;

    // Act
    // Open a span for the test, and propagate it to our server.
    // Make the outgoing http call to the cat endpoint. Fail if it returns an error.
    // Return the trace's id.
    let trace_id = {
        let test_span = info_span!("cat_endpoint_sends_a_trace_that_show_outgoing_http_calls");
        test_harness
            .client
            .get(test_harness.build_url("/cat"))
            .send()
            .instrument(test_span.clone())
            .await
            .expect("Failed to make request to server")
            .error_for_status()
            .expect("Expected a success response");

        test_span.otel_trace_id()
    };

    // Assert
    // We now expect, within a reasonable time frame, for our span to be available in our
    // local jaeger instance
    wait_for_trace(
        test_harness.global_jaeger_collector_server,
        trace_id,
        |trace| {
            let image_request_span = trace
                .descendants()
                .find(|s| s.borrow().operation_name == "GET /v1/images/search")
                .ok_or_else(|| anyhow!(r#"No span found named "GET /v1/images/search""#))?;

            check_tag(&image_request_span, "http.method", TagValue::String("GET"))
                .context("cat images api span was not correct")?;
            check_tag(&image_request_span, "http.status_code", TagValue::Long(200))
                .context("cat images api span was not correct")?;

            let fact_request_span = trace
                .descendants()
                .find(|s| s.borrow().operation_name == "GET /fact")
                .ok_or_else(|| anyhow!(r#"No span found named "GET /fact""#))?;

            check_tag(&fact_request_span, "http.method", TagValue::String("GET"))
                .context("cat images api span was not correct")?;
            check_tag(&fact_request_span, "http.status_code", TagValue::Long(200))
                .context("cat images api span was not correct")
        },
    )
    .await
    .expect("Expected trace was not available within timeout");
}

#[actix_rt::test]
pub async fn cat_endpoint_sends_a_trace_that_shows_the_incoming_http_request() {
    // Arrange
    // Set up pre-conditions for a successful call to /cat
    let test_harness = TestHarness::start().await;
    test_harness
        .mock_cat_images_api
        .configure_cat_image_url()
        .await;
    test_harness.mock_cat_facts_api.configure_cat_fact().await;

    // Act
    // Open a span for the test, and propagate it to our server.
    // Make the outgoing http call to the cat endpoint. Fail if it returns an error.
    // Return the trace's id.
    let trace_id = {
        let test_span =
            info_span!("cat_endpoint_sends_a_trace_that_shows_the_incoming_http_request");
        test_harness
            .client
            .get(test_harness.build_url("/cat"))
            .send()
            .instrument(test_span.clone())
            .await
            .expect("Failed to make request to server")
            .error_for_status()
            .expect("Expected a success response");

        test_span.otel_trace_id()
    };

    // Assert
    // We now expect, within a reasonable time frame, for our span to be available in our
    // local jaeger instance
    wait_for_trace(
        test_harness.global_jaeger_collector_server,
        trace_id.clone(),
        |trace| {
            let span_node = trace
                .descendants()
                .find(|s| s.borrow().operation_name == "HTTP request")
                .ok_or_else(|| anyhow!(r#"No span found named "HTTP request""#))?;

            check_tag(&span_node, "http.method", TagValue::String("GET"))
                .context("span method was not correct")?;
            check_tag(&span_node, "http.route", TagValue::String("/cat"))
                .context("span route was not correct")?;
            check_tag(&span_node, "http.status_code", TagValue::Long(200))
                .context("span status code was not correct")
        },
    )
    .await
    .expect("Expected trace was not available within timeout");
}

fn check_tag(span: &Node<Span>, key: &str, expected_value: TagValue) -> Result<(), anyhow::Error> {
    let span_ref = span.borrow();
    let tag = span_ref
        .get_tag(key)
        .ok_or_else(|| anyhow!(format!("No tag with key {} was found", key)))?;

    let value = tag.value().context("Could not interpret tag value")?;
    if value == expected_value {
        Ok(())
    } else {
        Err(anyhow!(format!(
            "Tag with key {} was found, but its value was {:?}, not {:?}",
            key, value, expected_value
        )))
    }
}

async fn wait_for_trace<F>(
    otel_collector: &DetachedJaegerCollectorServer,
    trace_id: String,
    check_trace: F,
) -> Result<(), RetryTimeoutError<anyhow::Error>>
where
    F: Fn(Node<Span>) -> Result<(), anyhow::Error>,
{
    let timeout = Duration::from_secs(5);
    retry_until_ok(
        || async {
            // Since our telemetry state is global and shared between our
            // test and our server, we can cheat a little here and force
            // `opentelemetry` to flush any pending traces
            force_flush_tracer_provider();
            let trace = otel_collector.get_trace(&trace_id).await?;
            check_trace(trace)
        },
        timeout,
        timeout,
        Duration::from_millis(100),
    )
    .await
}
