pub mod mocks;
use crate::utilities::retry_loop;

use self::mocks::{MockCatFactsApi, MockCatImagesApi};
use actix_rt::System;
use cat_server::{initialise_tracing, run_server, Configuration};
use mock_jaeger_collector::DetachedJaegerCollectorServer;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use std::future::pending;
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tokio::sync::OnceCell;

static TRACING_INIT: OnceCell<DetachedJaegerCollectorServer> = OnceCell::const_new();

/// Initialising the telemetry collection for these tests requires a bit of a ballet.
/// Since the `tracing` crate and the `opentelemetry` crate rely quite heavily on global
/// state, we are required to configure this in our tests exactly once.
/// This function can be called multiple times. The first time it is called, it will
/// initialise the global `tracing` and `opentelemetry` state, in the same way as would
/// happen during our application start up. It will also create a single
/// [`DetachedJaegerCollectorServer`] instance to receive traces and store them in
/// memory, for verification from tests. Subsequent calls will do nothing other than
/// return the stored reference to the previously-started DetachedJaegerCollectorServer.
async fn initialise_telemetry_collection() -> &'static DetachedJaegerCollectorServer {
    // This [`OnceCell`] guarantees the initialisation logic will be called only once, even
    // if this function is called multiple times.
    let server = TRACING_INIT
        .get_or_init(|| async {
            // Create the DetachedJaegerCollectorServer to receive traces from our application.
            let detached_jaeger_collector_server =
                DetachedJaegerCollectorServer::start().expect("Failed to start Jaeger collector");

            // Wait for the DetachedJaegerCollectorServer to be ready to accept
            // connections.
            retry_loop::retry_until_ok(
                || async { detached_jaeger_collector_server.ping().await },
                Duration::from_secs(10),
                Duration::from_secs(1),
                Duration::from_millis(50),
            )
            .await
            .expect("Timed out waiting for Jaeger collector to become ready");

            // `opentelemetry` requires a running async runtime exists that it can
            // use to spawn tasks. However, when running tests, each test is run
            // in its own actix `System` which is stopped upon completion of the test.
            //
            // Since the `opentelemetry` state is global, we need to avoid it capturing
            // a reference to a short lived `System` that will be stopped when the test
            // that created it completes.
            //
            // To do that, we spawn a new thread (which we detach, so it lives until
            // the process terminates), and create a new `System` within that thread.
            // We then perform our applications initialisation logic inside the new
            // `System`. We use a `mpsc::channel()` to communicate back to the thread
            // executing the initialisation logic when it is safe to proceed.
            let (sender, receiver) = mpsc::channel();
            let collector_url = detached_jaeger_collector_server.base_url();
            thread::spawn(move || {
                System::new().block_on(async move {
                    initialise_tracing(&collector_url);
                    sender.send(()).unwrap();
                    pending::<()>().await
                })
            });
            receiver.recv().unwrap();

            // Finally, we return the created jaeger collector server.
            detached_jaeger_collector_server
        })
        .await;

    server
}

pub struct TestHarness {
    /// A `reqwest_middleware::ClientWithMiddleware`, configured to propagate
    /// tracing context on requests, as our service expects clients to.
    pub client: ClientWithMiddleware,

    /// The configuration that this instance of the service started with.
    pub config: Configuration,

    /// The mock cat images API.
    pub mock_cat_images_api: MockCatImagesApi,

    /// The mock cat facts API.
    pub mock_cat_facts_api: MockCatFactsApi,

    /// A reference to the shared jaeger trace collector.
    /// It should be noted that this is static and shared across tests.
    /// Tests should therefore only be querying for specific traces they
    /// produce.
    pub jaeger_collector_server: &'static DetachedJaegerCollectorServer,
}

impl TestHarness {
    /// Starts a new instance of the service and any required mocks,
    /// returning a `TestHarness` which can be used to interact with
    /// the service and the mocks.
    pub async fn start() -> TestHarness {
        let mock_otel_collector = initialise_telemetry_collection().await;
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
            collector_url: mock_otel_collector.base_url(),
        };

        let server = run_server(config.clone(), listener);

        let _server_join_handle = actix_rt::spawn(server);

        let client = ClientBuilder::new(
            reqwest::ClientBuilder::new()
                .build()
                .expect("Failed to build http client"),
        )
        .with(TracingMiddleware)
        .build();

        TestHarness {
            client,
            config,
            mock_cat_images_api,
            mock_cat_facts_api,
            jaeger_collector_server: mock_otel_collector,
        }
    }

    /// Builds a URL to a relative path hosted by our service
    pub fn build_url(&self, relative_path: impl Into<String>) -> String {
        format!(
            "http://{}:{}{}",
            self.config.host,
            self.config.port,
            relative_path.into()
        )
    }
}
