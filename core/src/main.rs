#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    danneo_core::run().await;
}
