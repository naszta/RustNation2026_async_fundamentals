# Your Turn: Add graceful shutdown and error handling

> Working example code path: `code/tcp_server_graceful_shutdown`

What you want to try and accomplish:

1. Start with your async TCP server from the previous section.
2. Remove `unwrap()` from network operations (`bind`, `accept`, `read`, `write_all`) and handle errors explicitly.
3. Add a shutdown signal (for example with `tokio::sync::broadcast`).
4. In your accept loop, use `tokio::select!` so the server can react to:
   * A new connection.
   * A shutdown signal.
5. For each client connection, spawn a task and pass it a shutdown receiver (`resubscribe` works well here).
6. In the connection handler, use `tokio::select!` so it can react to:
   * Incoming bytes from the socket.
   * The shutdown signal.
7. When shutdown is requested:
   * Stop accepting new connections.
   * Let existing connection tasks exit cleanly.
   * Wait for spawned tasks to complete.
8. Bonus: put a timeout around writes so a stuck client doesn't hang shutdown forever.

The goal is to keep the server responsive while also shutting down predictably and safely.

Good luck! I'll be here to help.

---

## Scroll for answers

### What you should have seen

* The server exits cleanly after receiving a shutdown signal.
* Existing connections finish without panicking.
* Error paths are visible in logs instead of crashing the whole process.
* A slow or stuck write path doesn't block forever when timeout is applied.

### Full working example

The ready-to-run version is in `code/tcp_server_graceful_shutdown`.

From the `code` directory:

```bash
cargo run -p tcp_server_graceful_shutdown
```

```rust
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio::time::{Duration, timeout};

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = "127.0.0.1:3011";
    let listener = TcpListener::bind(addr).await?;
    println!("[main] listening on {addr}");

    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(16);
    let server_task = tokio::spawn(run_server(listener, shutdown_rx));

    tokio::time::sleep(Duration::from_millis(150)).await;

    run_client("client-1", addr, b"hello from client 1").await?;
    run_client("client-2", addr, b"hello from client 2").await?;

    println!("[main] sending shutdown signal");
    let _ = shutdown_tx.send(());

    match server_task.await {
        Ok(Ok(())) => println!("[main] server exited cleanly"),
        Ok(Err(e)) => eprintln!("[main] server returned error: {e}"),
        Err(e) => eprintln!("[main] server task join error: {e}"),
    }

    Ok(())
}

async fn run_server(
    listener: TcpListener,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> io::Result<()> {
    let mut connections = JoinSet::new();

    loop {
        tokio::select! {
            recv = shutdown_rx.recv() => {
                match recv {
                    Ok(()) => {
                        println!("[server] shutdown requested");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        eprintln!("[server] shutdown receiver lagged, skipped {skipped} signal(s)");
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        println!("[server] shutdown channel closed");
                        break;
                    }
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((socket, peer_addr)) => {
                        println!("[server] accepted {peer_addr}");
                        let conn_shutdown = shutdown_rx.resubscribe();
                        connections.spawn(async move {
                            if let Err(e) = handle_connection(socket, conn_shutdown).await {
                                eprintln!("[server] connection {peer_addr} error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[server] accept error: {e}");
                    }
                }
            }
        }
    }

    println!("[server] waiting for active connections to finish");
    while let Some(joined) = connections.join_next().await {
        if let Err(e) = joined {
            eprintln!("[server] connection task join error: {e}");
        }
    }
    println!("[server] all connection tasks finished");

    Ok(())
}

async fn handle_connection(
    mut socket: TcpStream,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> io::Result<()> {
    let mut buf = [0_u8; 1024];

    loop {
        tokio::select! {
            recv = shutdown_rx.recv() => {
                match recv {
                    Ok(()) => {
                        socket.write_all(b"server shutting down\n").await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) | Err(broadcast::error::RecvError::Closed) => {}
                }
                return Ok(());
            }
            read_result = socket.read(&mut buf) => {
                match read_result {
                    Ok(0) => return Ok(()),
                    Ok(n) => {
                        if let Err(e) = timeout(Duration::from_secs(2), socket.write_all(&buf[..n])).await {
                            return Err(io::Error::new(io::ErrorKind::TimedOut, format!("write timeout: {e}")));
                        }
                    }
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::ConnectionReset, format!("read failed: {e}")));
                    }
                }
            }
        }
    }
}

async fn run_client(name: &str, addr: &str, msg: &[u8]) -> io::Result<()> {
    let mut socket = TcpStream::connect(addr).await?;
    socket.write_all(msg).await?;

    let mut buf = [0_u8; 1024];
    let n = socket.read(&mut buf).await?;
    println!("[{name}] received: {}", String::from_utf8_lossy(&buf[..n]));

    Ok(())
}
```
