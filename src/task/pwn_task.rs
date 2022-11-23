use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::{
    boxed::Box,
    sync::{Arc},
    vec::Vec,
};
use tokio::{
    select, spawn,
    sync::{mpsc, oneshot},
};

#[derive(Copy, Clone)]
pub enum TaskStatus {
    PENDING,
    STARTING,
    RUNNING,
    STOPPING,
    TERMINAL,
}

#[async_trait]
pub trait Task {
    async fn spawn_task(self: &mut Self) -> Result<()>;
    async fn shutdown(self: &mut Self) -> Result<()>;
    fn task_status(self: &Self) -> TaskStatus;
}

pub struct TaskPool<T, F>
where
    T: 'static + Task + Send + Sync + Unpin,
    F: Fn() -> T + Send + Sync + Unpin + 'static,
{
    new_task: F,    
    pool_size: usize,
    // tasks: Option<Vec<Box<T>>>,
    // task_req_receiver: Option<mpsc::Receiver<oneshot::Sender<Box<T>>>>,
    task_req_sender: mpsc::Sender<oneshot::Sender<T>>,
    task_drop_sender: mpsc::Sender<T>,
    // task_drop_receiver: Option<mpsc::Receiver<Box<T>>>,
    max_running_task: usize,
}

impl<T, F> TaskPool<T, F>
where T: 'static + Task + Send + Sync + Unpin,
      F: Fn() -> T + Send + Sync + Unpin + 'static, 
{
    pub fn new(pool_size: usize, max_running: usize, new_task_func: F) -> Arc<TaskPool<T, F>> {
        let (req_tx, req_rx) = mpsc::channel(100);
        let (drop_tx, drop_rx) = mpsc::channel(100);

        let pool = TaskPool{new_task: new_task_func, 
            pool_size: pool_size, 
            // tasks: Some(Vec::new()), 
            // task_req_receiver: Some(req_rx),
            task_req_sender: req_tx,
            // task_drop_receiver: Some(drop_rx),
            task_drop_sender: drop_tx,
            max_running_task: max_running };

            let arc_pool = Arc::new(pool);
            let arc_pool2 = arc_pool.clone();

            spawn(async move {
                arc_pool2.task_pool_worker(req_rx, drop_rx).await;
            });

            arc_pool
    }

    pub async fn get_task(self: &Arc<Self>) -> Result<T>  {
        let (tx, rx) = oneshot::channel();
        if let Err(_e) = self.task_req_sender.clone().send(tx).await{
            return Err(anyhow!("can not send"));
        }
        Ok(rx.await?)
    }

    pub async fn drop_task(self: &Arc<Self>, mut task: T) -> Result<()> {
        task.shutdown().await;
        if let Err(_e) = self.task_drop_sender.clone().send(task).await{
            return Err(anyhow!("can not send"));
        }
        Ok(())
    }

    async fn task_spawned_loop(self: Arc<Self>, sender: mpsc::Sender<T>, mut signal_receiver: mpsc::Receiver<bool>) {
        loop {
            println!("in");
            signal_receiver.recv().await;
            println!("start");
            let re1 = self.new_spawned_task().await;
            match re1 {
                Ok(task) => {
                    sender.send(task).await;
                },
                Err(re) => println!("err new {}", re),
            }
        }
    }

    async fn task_pool_worker(self: Arc<Self>, mut req_receiver: mpsc::Receiver<oneshot::Sender<T>> , mut drop_receiver: mpsc::Receiver<T> ) {
        let mut tasks = Vec::new();
        let mut running_task: usize = 0;
        let mut task_spwaner_running = false;
        let (task_tx, mut task_rx) = mpsc::channel(100);
        let (sig_tx, sig_rx) = mpsc::channel(100);
        spawn(self.clone().task_spawned_loop(task_tx, sig_rx));
        sig_tx.send(true).await; // push one signal to start loop
        loop {
            select!{
                re1 = task_rx.recv() => {
                    match re1 {
                        Some(task) => {
                            tasks.push(task);
                            running_task += 1;
                            println!("run add {}", running_task);
                        },
                        None => println!("err new "),
                    }
                    task_spwaner_running = false;
                },
                re2 = req_receiver.recv(), if !tasks.is_empty() => {
                    match re2 {
                        Some(sender) => {
                            sender.send(tasks.pop().unwrap());
                        },
                        None => println!("err get"),
                    }
                },
                re3 = drop_receiver.recv() => {
                    match re3 {
                        Some(task) => {
                            running_task -= 1;
                            println!("run sub {}", running_task);
                        },
                        None => println!("err drop"),
                    }
                }
            }
            if tasks.len() < self.pool_size && running_task < self.max_running_task && !task_spwaner_running{
                sig_tx.send(true).await;
                task_spwaner_running = true;
            }
        }
    }

    async fn new_spawned_task(self: &Self) -> Result<T> {
        let func = &self.new_task;
        let mut t = func();
        t.spawn_task().await?;
        Ok(t)
    }

}