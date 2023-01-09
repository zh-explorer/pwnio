use anyhow::Result;
use rand::Rng;
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
    sync::oneshot::{channel, Receiver, Sender},
    spawn,select,
};

pub struct Forward {
    pub port: u16,
    target_addr: String,
    server: TcpListener,
    stop_rx: Option<Receiver<bool>>,
}

impl Forward {
    pub async fn new(target: String) -> (Forward, Sender<bool>) {
        let mut port: u16;
        let server;
        loop {
            {
            let mut rng = rand::thread_rng();
                port = rng.gen_range(30000..40000);
            }
            match TcpListener::bind(("0.0.0.0", port)).await {
                Ok(listener) => {
                    server = listener;
                    break;
                }
                _ => (),
            }
        }
        let (tx, rx) = channel();
        (Forward {
            port: port,
            target_addr: target,
            server: server,
            stop_rx: Some(rx),
        }, tx)
    }

    pub async fn start_server(mut self: Self) -> Result<()> {
        let mut rx = self.stop_rx.take().unwrap();
        loop {
            select!{
                Ok((socket, _)) = self.server.accept() => {
                    spawn(Forward::forward_addr(socket, self.target_addr.clone()));
                },
                _ = &mut rx => break,
            }
        }
        Ok(())
    }

    async fn forward_addr(mut stream: TcpStream, addr: String) -> Result<()> {
        let mut target = TcpStream::connect(addr).await?;
        copy_bidirectional(&mut stream, &mut target).await?;
        Ok(())
    }
    
    pub fn stop(stop_signal: Sender<bool>) {
        stop_signal.send(true);
    }

}

