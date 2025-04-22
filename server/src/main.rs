use server::run;

#[tokio::main]
async fn main() {
    env_logger::init();
    run("127.0.0.1:8080").await;
}
