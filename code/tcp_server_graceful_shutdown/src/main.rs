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
