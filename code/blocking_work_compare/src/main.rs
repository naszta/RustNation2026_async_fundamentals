use futures::future::join_all;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    run_multithread_runtime();
    run_current_thread_runtime();
}

async fn run_blocking_sleep(label: &str) {
    let start = Instant::now();
    let tasks: Vec<_> = (0..3).map(|n| blocking_looper(n, start, label)).collect();
    join_all(tasks).await;
}

async fn run_async_sleep(label: &str) {
    let start = Instant::now();
    let tasks: Vec<_> = (0..3).map(|n| async_looper(n, start, label)).collect();
    join_all(tasks).await;
}

async fn run_spawn_blocking(label: &str) {
    let start = Instant::now();
    let tasks: Vec<_> = (0..3).map(|n| looper_with_spawn_blocking(n, start, label)).collect();
    join_all(tasks).await;
}

async fn blocking_looper(n: u8, start: Instant, label: &str) {
    for i in 0..3 {
        println!(
            "[{label}] +{:>4}ms task {n} iteration {i} (before std::thread::sleep)",
            start.elapsed().as_millis()
        );
        thread::sleep(Duration::from_millis(60));
    }
}

async fn async_looper(n: u8, start: Instant, label: &str) {
    for i in 0..3 {
        println!(
            "[{label}] +{:>4}ms task {n} iteration {i} (before tokio::time::sleep)",
            start.elapsed().as_millis()
        );
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
}

async fn looper_with_spawn_blocking(n: u8, start: Instant, label: &str) {
    for i in 0..3 {
        println!(
            "[{label}] +{:>4}ms task {n} iteration {i} (before spawn_blocking)",
            start.elapsed().as_millis()
        );

        tokio::task::spawn_blocking(|| {
            thread::sleep(Duration::from_millis(60));
        })
        .await
        .expect("spawn_blocking task panicked");
    }
}

fn run_multithread_runtime() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Failed to build multi-thread runtime");

    runtime.block_on(async {
        println!("=== RUN 1: BAD - std::thread::sleep in async code ===");
        run_blocking_sleep("multithread").await;

        println!("\n=== RUN 2: GOOD - tokio::time::sleep().await ===");
        run_async_sleep("multithread").await;

        println!("\n=== RUN 3: GOOD - move blocking work to spawn_blocking ===");
        run_spawn_blocking("multithread").await;
    });
}

fn run_current_thread_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build current_thread runtime");

    runtime.block_on(async {
        println!("\n=== RUN 4: current_thread runtime comparison ===");
        println!("-- current_thread + std::thread::sleep (bad) --");
        run_blocking_sleep("current_thread").await;

        println!("\n-- current_thread + tokio::time::sleep (good) --");
        run_async_sleep("current_thread").await;

        println!("\n-- current_thread + spawn_blocking (good) --");
        run_spawn_blocking("current_thread").await;
    });
}
