mod task;
mod util;
mod io;

use std::{sync::{Arc}};
use tokio::{
    io::{ AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{timeout, Duration},
    sync::{mpsc::{Receiver}, Mutex},
    spawn,
};
use bytes::BytesMut;
use task::{pwn_task::{TaskPool}, vm_task::VmTask};
use util::pwn_util::{token_checker, ChallQueue};
use io::io_forward::Forward;
use anyhow::Result;

const LISTEN_PORT: u16 = 9999;
const LISTEN_IP: &str = "0.0.0.0";
const TIMEOUT: u64 = 300;
const POOL_SIZE: usize = 12;
const MAX_TASK: usize = 24;

async fn wait_queue(stream:&mut TcpStream,mut recevier:Receiver<u64>) -> Result<()>{

    loop {
        let count = recevier.recv().await.unwrap();
        if count == 0 {
            stream.write_all(b"thx for wait, not you can do challenge\n").await;
            break;
        }else {
            let message = format!("There are {} challengers are waitting for do challenge, plz wait\n", count-1);
            stream.write_all(message.as_bytes()).await;
        }
    }
    Ok(())
}

async fn wait_for_gone(stream:&mut TcpStream, msg: String) {
    loop {
        let mut buffer = BytesMut::with_capacity(128);
        let re = stream.read_buf(&mut buffer).await;
        match re {
            Ok(count) => {
                if count == 0 {
                    break;
                }else {
                    stream.write_all(b"talk to this port is use less.\n").await;
                    stream.write_all(msg.as_bytes()).await;
                    buffer.clear();
                }
            },
            Err(er) => break,
        }
    }
}

async fn process_connect<F>(mut stream: TcpStream, pool: Arc<TaskPool<VmTask, F>>, queue: Arc<Mutex<ChallQueue>>, tokens: Arc<Mutex<Vec<String>>>) -> Result<()>
where
    F: Fn() -> VmTask + Send + Sync + Unpin + 'static,
{
    stream.write_all(b"1. tcp port 8000 will be forwarded to another port\n2. Only one instance is allowd for each team at a time\n3. Please input your team token now:\n").await?;

    let mut team_token = Vec::new();

    loop {
        let c = stream.read_u8().await?;
        if c == 10 {
            break;
        }
        team_token.push(c);
    }
    
    let str_team_token = String::from_utf8_lossy(&team_token).to_string().trim().to_string();
    println!("get token: {}", str_team_token.clone());
    let result = token_checker(str_team_token.clone()).await?;
    if !result {
        stream.write_all(b"team token error\n").await?;
        stream.shutdown().await?;
        return Ok(())
    }

    let re;

    {
        let mut token_vec = tokens.lock().await;
        let mut token_iter = token_vec.iter();
        if let Some(_idx) = token_iter.position(|x| x == &str_team_token) {
            stream.write_all(b"every team can only have one instance\n").await?;
            stream.shutdown().await?;
            return Ok(())
        }
        token_vec.push(str_team_token.clone());

        re = queue.lock().await.queue_in().await;
    }

    if let Some(recevier) = re {
        wait_queue(&mut stream, recevier).await;
    }

    if let Err(_er) = stream.write_all(b"starting challenge server\nThis should take some times\n").await {
        // this one is gone, release and return
        {
            queue.lock().await.queue_out().await;
        }
        {
            let mut token_vec = tokens.lock().await;
            let mut token_iter = token_vec.iter();
            if let Some(idx) = token_iter.position(|x| x == &str_team_token) {
                token_vec.remove(idx);
            }else {
                println!("no way!\n");
            }
        }
        return Ok(());
    }

    let task = pool.get_task().await?;
    let mut address = task.get_ip();
    address.push_str(":8000");

    let (forward, stop_signal) = Forward::new(address).await;

    let message = format!("you have {}sec, And you server is at 47.90.215.212:{}\nchange you host apps.example.com to this ip and access http://apps.example.com:{}/OA_HTML/AppsLocalLogin.jsp\nplz do not close this connection unless you want to release all resource and quit", TIMEOUT, forward.port, forward.port);
    stream.write_all(message.clone().as_bytes()).await;

    spawn(forward.start_server());

    let res = timeout(Duration::from_secs(TIMEOUT), wait_for_gone(&mut stream, message)).await;
    if res.is_err() {
        stream.write_all(b"timeout!! bye\n").await?;
    }
    stream.shutdown().await?;
    Forward::stop(stop_signal);
    pool.drop_task(task).await?;    // TODO: raise
    {
        queue.lock().await.queue_out().await;
    }
    {
        let mut token_vec = tokens.lock().await;
        let mut token_iter = token_vec.iter();
        if let Some(idx) = token_iter.position(|x| x == &str_team_token) {
            token_vec.remove(idx);
        }else {
            println!("no way!\n");
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

    let pool = TaskPool::new(POOL_SIZE, MAX_TASK, VmTask::new);
    let queue = Arc::new(Mutex::new(ChallQueue::new(MAX_TASK)));
    let team_tokens = Arc::new(Mutex::new(Vec::<String>::new()));

    let listener = TcpListener::bind((LISTEN_IP, LISTEN_PORT)).await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("get connnection from {}", addr);

        let queue = queue.clone();
        let team_tokens = team_tokens.clone();
        let pool = pool.clone();
        tokio::spawn(async move{
            if let Err(re) = tokio::spawn(process_connect(socket, pool.clone(), queue.clone(), team_tokens.clone())).await {
                println!("{}", re);
            }
        });
        // 
    }
}