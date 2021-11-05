# When Is A Black Box Test Not A Black Box Test?

The code in this repo accompanies [this article](??), as a demonstration of one way the observability of a service can be exercised and verified during automated tests.

Disclaimer: This code is provided AS IS. It is NOT intended as an example of a production-ready service.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install), version >= `1.56.1`.

## Layout

This repository contains a cargo workspace consisting of two crates:

- `cat_server`, the main binary of our hypothetical HTTP server;
- `mock_jaeger_collector`, a library crate consisting of a mock Jaeger collector service, for use in component testing.

## To Run

- `cargo run`
- `curl http://localhost:12345/cat`
- Optionally, you can also run `docker-compose up` to start a local Jaeger instance, viewable at http://localhost:16686

## To Test

Run `cargo test`. No additional services are assumed to be running.
