// Implemention for pwn tasks
use tokio::{
    process::{Child, ChildStdin, ChildStdout, ChildStderr},
    time::timeout,
};
use std::{
    sync::{Arc, Mutex}, 
    result::{Result}, 
    error::{Error},
    io,
    time::Duration, 
    process::ExitStatus,
};
use async_trait::async_trait;

// there are two type of pwn challenge
// 1. use stdin/stdout/stderr io to community with challenger. we need spawn a task and forword stdio for than
// 2. use a independent port. should wrapping challenge with a docker and do port forwrod for that.

enum TaskStatus {
    PENDING,
    STARTING,
    RUNNING,
    STOPPING,
    TERMINAL,
}

struct PwnTask<T> 
where T: Task {
    child_process: Option<Arc<Mutex<Child>>>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    stdout: Option<Arc<Mutex<ChildStdout>>>,
    stderr: Option<Arc<Mutex<ChildStderr>>>,
    timeout: u32, // zero for no timeout 
    task: T,
}


#[async_trait]
trait Task {
    async fn spawn_task(self: &mut Self) -> Result<Arc<Mutex<Child>>, Box<dyn Error>>;
    async fn shutdown(self: &Self) -> Result<(), Box<dyn Error>>;
    fn task_status(self: &Self) -> TaskStatus;
}

impl<T> PwnTask<T>
where T: Task
{
    fn new(task: T, timeout: u32) -> PwnTask<T> {
        PwnTask {
            child_process: None,
            stdin: None,
            stdout: None,
            stderr: None,
            timeout: timeout,
            task: task,
        }
    }

    async fn spawn_task(self: &mut Self) -> Result<(), Box<dyn Error>> {
        match self.task.task_status() {
            TaskStatus::PENDING => (),
            _ => panic!("do not spawn task twice"),
        }
        
        self.child_process = Some(self.task.spawn_task().await?);
        
        let mut child_process = self.child_process.as_ref().unwrap().lock().unwrap();

        self.stdin = child_process.stdin.take().map(|s| Arc::new(Mutex::new(s)));
        self.stdout = child_process.stdout.take().map(|s| Arc::new(Mutex::new(s)));
        self.stderr = child_process.stderr.take().map(|s| Arc::new(Mutex::new(s)));

        Ok(())
    }

    // this task will lock child_porcess
    // cancel this when you wait shutdown
    async fn wait_for_stop(self: &Self) -> Result<ExitStatus, io::Error> {
        let result;
        {
            let mut process = self.child_process.as_ref().unwrap().lock().unwrap();
            let process_future = process.wait();
            result = if self.timeout == 0 {
                process_future.await
            }else {
                match timeout(Duration::from_secs(self.timeout as u64), process_future).await {
                    Err(_) => Err(io::Error::new( io::ErrorKind::TimedOut, "timeout!!")),
                    Ok(result) => result,
                }
            };
        } // unlock process, and call shutdown for exit
        self.task.shutdown().await;
        result
    }

    async fn shutdown(self: &Self) -> Result<(), Box<dyn Error>> {
        self.task.shutdown().await
    }

    fn task_status(self: &Self) -> TaskStatus {
        self.task.task_status()
    }

}