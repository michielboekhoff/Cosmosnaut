use std::{collections::HashMap, sync::Arc};

use axum::{
    async_trait,
    extract::{FromRef, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, IntoResponseParts},
    routing::post,
    Json, Router,
};
use azure_data_cosmos::resources::Database as CosmosDatabase;
use serde::Deserialize;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("database with id {0} already exists")]
    DatabaseExists(String),
}

#[async_trait]
pub trait Store {
    async fn create_database(&mut self, id: String) -> Result<(), DatabaseError>;
}

struct Database {
    pub id: String,
}

struct HashMapStore {
    entries: HashMap<String, Database>,
}

impl HashMapStore {
    fn new() -> Self {
        HashMapStore {
            entries: HashMap::new(),
        }
    }
}

#[async_trait]
impl Store for HashMapStore {
    async fn create_database<'a>(&'a mut self, id: String) -> Result<(), DatabaseError> {
        if self.entries.contains_key(&id) {
            Err(DatabaseError::DatabaseExists(id))
        } else {
            self.entries.insert(id.clone(), Database { id });
            Ok(())
        }
    }
}

struct AppState<S: Store> {
    database_store: Arc<Mutex<S>>,
}

impl<S: Store> AppState<S> {
    pub fn new(store: S) -> AppState<S> {
        AppState {
            database_store: Arc::new(Mutex::new(store)),
        }
    }
}

impl<S: Store> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            database_store: self.database_store.clone(),
        }
    }
}

impl<T: Store> FromRef<AppState<T>> for Arc<Mutex<T>> {
    fn from_ref(input: &AppState<T>) -> Self {
        input.database_store.clone()
    }
}

struct CosmosHeaders {
    charge: f64,
    etag: String,
    session_token: String,
}

impl IntoResponseParts for CosmosHeaders {
    type Error = StatusCode;

    fn into_response_parts(
        self,
        mut res: axum::response::ResponseParts,
    ) -> Result<axum::response::ResponseParts, Self::Error> {
        let headers = res.headers_mut();
        headers.insert(
            "x-ms-request-charge",
            self.charge
                .to_string()
                .parse()
                .expect("charge should always be valid"),
        );

        headers.insert(
            "etag",
            HeaderValue::from_str(self.etag.as_str()).expect("Expected etag to be valid"),
        );

        headers.insert(
            "x-ms-session-token",
            HeaderValue::from_str(self.session_token.as_str())
                .expect("Session token should always be valid"),
        );

        headers.insert(
            "x-ms-last-state-change-utc",
            HeaderValue::from_static("Fri, 25 Mar 2016 22:54:09.213 GMT"),
        );

        headers.insert(
            "x-ms-resource-quota",
            HeaderValue::from_static("collections=5000"),
        );

        headers.insert(
            "x-ms-resource-usage",
            HeaderValue::from_static("collections=28"),
        );

        headers.insert("x-ms-quorum-acked-lsn", HeaderValue::from_static("7902"));

        headers.insert("x-ms-current-write-quorum", HeaderValue::from_static("3"));

        headers.insert(
            "x-ms-current-replica-set-size",
            HeaderValue::from_static("4"),
        );

        headers.insert("x-ms-schemaversion", HeaderValue::from_static("1.1"));

        headers.insert(
            "x-ms-serviceversion",
            HeaderValue::from_static("version=1.6.52.5"),
        );

        headers.insert(
            "x-ms-activity-id",
            HeaderValue::from_static("6050a48c-828d-4016-b73b-5e0ce2ad6271"),
        );

        headers.insert(
            "x-ms-gatewayversion",
            HeaderValue::from_static("version=1.6.52.5"),
        );

        Ok(res)
    }
}

async fn create_database<T: Store>(
    State(svc): State<Arc<Mutex<T>>>,
    Json(payload): Json<CreateDatabaseRequest>,
) -> (StatusCode, CosmosHeaders, impl IntoResponse) {
    let mut x = svc.lock().await;
    match x.create_database(payload.id.clone()).await {
        Ok(_) => (
            StatusCode::OK,
            CosmosHeaders {
                charge: 1.2,
                etag: String::from("33a64df551425fcc55e4d42a148795d9f25f89d4"),
                session_token: String::from("session_token"),
            },
            Json(CosmosDatabase {
                id: payload.id,
                rid: String::from("123"),
                ts: 12345,
                _self: String::from("123"),
                etag: String::from("123"),
                colls: String::from("123"),
                users: String::from("123"),
            }),
        ),
        Err(DatabaseError::DatabaseExists(_)) => (
            StatusCode::BAD_REQUEST,
            CosmosHeaders {
                charge: 1.2,
                etag: String::from("33a64df551425fcc55e4d42a148795d9f25f89d4"),
                session_token: String::from("session_token"),
            },
            Json(CosmosDatabase {
                id: String::from("123"),
                rid: String::from("123"),
                ts: 12345,
                _self: String::from("123"),
                etag: String::from("123"),
                colls: String::from("123"),
                users: String::from("123"),
            }),
        ),
    }
}

#[derive(Deserialize)]
struct CreateDatabaseRequest {
    pub id: String,
}

pub fn app() -> Router {
    let store = HashMapStore::new();
    let app_state = AppState::new(store);
    Router::new()
        .route("/dbs", post(create_database))
        .with_state(app_state)
}
