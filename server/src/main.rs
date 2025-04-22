use server::run;

#[tokio::main]
async fn main() {
    env_logger::init();
    run("0.0.0.0:8080").await;
}
