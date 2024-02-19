use tokio::net::TcpListener;

use cosmosnaut::app;

#[tokio::main]
async fn main() {
    let app = app();

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap()
}
