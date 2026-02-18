#[tokio::main]
async fn main() {
    hello().await;
}

async fn hello() {
    println!("Hello world");
}
