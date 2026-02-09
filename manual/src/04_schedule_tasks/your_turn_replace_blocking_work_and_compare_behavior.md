# Your Turn: Replace blocking work and compare behavior

> Working example code path: `code/blocking_work_compare`

What you want to try and accomplish:

1. Create a small example with an async `looper(n)` function that prints 3 iterations, and use `futures::future::join_all` to run `looper(0..3)` together.
2. In `looper`, use `std::thread::sleep` first. Run it and look at the ordering of output.
3. Replace `std::thread::sleep` with `tokio::time::sleep(...).await`. Run it again and compare the output ordering.
4. Switch your runtime to `#[tokio::main(flavor = "current_thread")]` and run the async-sleep version again.
5. Switch back to `#[tokio::main]` (multi-thread runtime) and run once more.
6. Write down what changed between each run:
   * Did output stay grouped by task, or interleave?
   * Which versions blocked progress for other tasks?
   * Which versions allowed fair progress across tasks?
7. If you finish quickly, add one intentionally blocking operation (like CPU-heavy work or another `std::thread::sleep`) and then move it into `tokio::task::spawn_blocking` to compare behavior.

The goal is to make the blocking-vs-non-blocking difference obvious in your own output, not just in mine.

Good luck! I'll be here to help.

---

## Scroll for answers

### What you should have seen

* `std::thread::sleep` inside async code tends to bunch output by task, because worker threads get blocked.
* `tokio::time::sleep(...).await` interleaves output much more cleanly, because the runtime can schedule other ready tasks.
* `spawn_blocking` keeps blocking work off the async scheduler, so async tasks keep making progress.
* On `current_thread`, blocking in async code is especially obvious: one blocked task blocks *everything* else on that runtime thread.

### Full working example

The ready-to-run version is in `code/blocking_work_compare`.

From the `code` directory:

```bash
cargo run -p blocking_work_compare
```

The example does all four runs for you:

1. Multi-thread runtime + bad blocking (`std::thread::sleep` in async task).
2. Multi-thread runtime + good non-blocking sleep (`tokio::time::sleep().await`).
3. Multi-thread runtime + blocking moved into `tokio::task::spawn_blocking`.
4. Current-thread runtime, repeating the same comparisons.

```rust
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
```
