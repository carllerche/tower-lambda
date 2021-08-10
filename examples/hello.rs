use axum::prelude::*;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = route("/", get(|| async { "Hello, World!" }));

    tower_lambda::run(app).await.unwrap();
}
