use crate::cosmos_transport::CosmosTestClient;
use std::{error::Error, sync::Arc};

use axum_test::{TestServer, TestServerConfig};
use azure_core::TransportOptions;
use azure_data_cosmos::{
    clients::CosmosClientBuilder, resources::permission::AuthorizationToken, ConsistencyLevel,
};
use cosmosnaut::app;

use assertor::*;

mod cosmos_transport {
    use std::{collections::HashMap, pin::Pin, str::FromStr};

    use axum::async_trait;
    use azure_core::{
        error::ErrorKind,
        headers::{HeaderName, HeaderValue, Headers},
        Body, HttpClient, StatusCode,
    };
    use futures::TryStreamExt;
    use futures_core::Stream;
    use reqwest::header::HeaderMap as ReqwestHeaderMap;
    use url::Url;

    #[derive(Debug)]
    pub struct CosmosTestClient {
        base_url: Url,
        client: reqwest::Client,
    }

    impl CosmosTestClient {
        pub fn new(base_url: Url) -> Self {
            CosmosTestClient {
                base_url,
                client: reqwest::Client::default(),
            }
        }
    }

    type PinnedStream = Pin<Box<dyn Stream<Item = azure_core::Result<bytes::Bytes>> + Send + Sync>>;

    #[async_trait]
    impl HttpClient for CosmosTestClient {
        #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
        async fn execute_request(
            &self,
            request: &azure_core::Request,
        ) -> Result<azure_core::Response, azure_core::Error> {
            let mut url = self.base_url.clone();
            url.set_path(request.url().path());

            let http_method = reqwest::Method::from_str(request.method().to_string().as_str())
                .expect("expected method to be valid");
            let mut request_builder = self
                .client
                .request(http_method, url)
                // TODO this line is actually incorrect but we need this because otherwise Axum
                // doesn't accept the request due to the extractor
                .header("Content-Type", "application/json");

            for (name, value) in request.headers().iter() {
                request_builder = request_builder.header(name.as_str(), value.as_str());
            }

            let body = request.body().clone();
            let request = match body {
                Body::Bytes(bytes) => request_builder.body(bytes).build(),
                Body::SeekableStream(s) => {
                    request_builder.body(reqwest::Body::wrap_stream(s)).build()
                }
            }
            .map_err(|error| {
                azure_core::error::Error::full(
                    ErrorKind::DataConversion,
                    error,
                    "error during body conversion",
                )
            })?;

            let response = self.client.execute(request).await.map_err(|err| {
                azure_core::Error::full(ErrorKind::Io, err, "failed to make request")
            })?;
            let response_headers = to_headers(response.headers());

            let b: PinnedStream = Box::pin(
                response
                    .bytes_stream()
                    .map_err(|error| azure_core::error::Error::full(ErrorKind::Io, error, "")),
            );

            return Ok(azure_core::Response::new(
                StatusCode::Created,
                response_headers,
                b,
            ));
        }
    }

    fn to_headers(headers: &ReqwestHeaderMap) -> Headers {
        let mapped_headers: HashMap<_, _> = headers
            .iter()
            .filter_map(|(k, v)| {
                let key = k.as_str();
                if let Ok(val) = v.to_str() {
                    Some((
                        HeaderName::from(key.to_owned()),
                        HeaderValue::from(val.to_owned()),
                    ))
                } else {
                    None
                }
            })
            .collect();
        Headers::from(mapped_headers)
    }
}

#[tokio::test]
async fn create_database() -> Result<(), Box<dyn Error>> {
    let test_server_config = TestServerConfig::builder().http_transport().build();
    let server = TestServer::new_with_config(app(), test_server_config)?;

    let server_address = server.server_address().unwrap();
    let cosmos_test_client = Arc::new(CosmosTestClient::new(server_address));
    let transport = TransportOptions::new(cosmos_test_client);

    let client = CosmosClientBuilder::new("cosmos_account", AuthorizationToken::primary_key("")?)
        .transport(transport)
        .build();

    let _database_creation_result = client
        .create_database("my_awesome_db")
        .consistency_level(ConsistencyLevel::Strong)
        .into_future()
        .await;

    assert_that!(_database_creation_result).is_ok();

    Ok(())
}
