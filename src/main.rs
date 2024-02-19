use std::{collections::HashMap, sync::Arc};

use axum::{
    async_trait,
    extract::{FromRef, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
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
    async fn create_database<'a>(&'a mut self, id: String) -> Result<(), DatabaseError>;
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

async fn create_database<T: Store>(
    State(svc): State<Arc<Mutex<T>>>,
    Json(payload): Json<CreateDatabaseRequest>,
) -> StatusCode {
    let mut x = svc.lock().await;
    match x.create_database(payload.id).await {
        Ok(_) => StatusCode::OK,
        Err(DatabaseError::DatabaseExists(_)) => StatusCode::BAD_REQUEST,
    }
}

#[derive(Deserialize)]
struct CreateDatabaseRequest {
    pub id: String,
}

#[tokio::main]
async fn main() {
    let store = HashMapStore::new();
    let app_state = AppState::new(store);
    let app = Router::new()
        .route("/dbs", post(create_database))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap()
}
