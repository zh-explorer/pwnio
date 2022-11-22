use bytes::BytesMut;
use std::{io::Result, result};
use tokio::{
    io::{copy, split, AsyncReadExt, AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
    process::{Command, Child}
};
use std::process::{Stdio};

const LISTEN_PORT: u16 = 12345;
const LISTEN_IP: &str = "0.0.0.0";

// async fn process_connect(stream: TcpStream) -> Result<()> {
//     let (mut reader, mut writer) = split(stream);
//     copy(&mut reader, &mut writer).await?;

    // let mut buffer = BytesMut::new();
    // loop {
    //     stream.read_buf(&mut buffer).await?;
    //     if buffer.is_empty() {
    //         break;
    //     }
    //     stream.write_all_buf(&mut buffer).await?;
    //     buffer.clear();
    // }
//     Ok(())
// }

fn spawn_process() -> Child {
    let mut cmd = Command::new("sh").stdout(Stdio::piped()).stdin(Stdio::piped()).kill_on_drop(true).spawn().unwrap();
    cmd
}

async fn process_connect(stream: TcpStream) -> Result<()> {
    let mut child_process = spawn_process();
    let mut stdout = child_process.stdout.take().unwrap();
    let mut stdin = child_process.stdin.take().unwrap();
    let (mut reader, mut writer) = split(stream);

    let channel1 = async move {
        copy(&mut stdout, &mut writer).await;
        writer.shutdown().await;
    };

    let channel2 = async move {
        copy(&mut reader, &mut stdin).await;
        stdin.shutdown().await;
        
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // !!!!! WORNING!! for some pwn challenge that not well handle of stdio close, uncomment this line !!!!!!
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // child_process.start_kill();
    };

    tokio::join!(channel1, channel2);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind((LISTEN_IP, LISTEN_PORT)).await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("get connnection from {}", addr);
        tokio::spawn(process_connect(socket));
    }
}
