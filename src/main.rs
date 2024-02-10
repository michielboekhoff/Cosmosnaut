use anyhow::Result;
use axum::{
    async_trait,
    extract::{FromRef, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Deserialize;

#[async_trait]
pub trait Store {
    async fn create_database(self, id: String) -> Result<()>;
}

#[derive(Clone)]
struct NoopStore {}

#[async_trait]
impl Store for NoopStore {
    async fn create_database(self, _id: String) -> Result<()> {
        Ok(())
    }
}

impl FromRef<AppState<Self>> for NoopStore {
    fn from_ref(input: &AppState<Self>) -> Self {
        input.database_store.clone()
    }
}

#[derive(Clone)]
struct AppState<S: Store> {
    database_store: S,
}

async fn create_database<T: Store>(
    State(_svc): State<T>,
    Json(_payload): Json<CreateDatabaseRequest>,
) -> Result<Response, StatusCode> {
    Ok(().into_response())
}

#[derive(Deserialize)]
struct CreateDatabaseRequest {
    pub id: String,
}

#[tokio::main]
async fn main() {
    let store = NoopStore {};
    let app_state = AppState {
        database_store: store,
    };

    let app = Router::new()
        .route("/dbs", post(create_database::<NoopStore>))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap()
}
