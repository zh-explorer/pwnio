// Implemention for pwn tasks
use tokio::{process::{Child, ChildStdin, ChildStdout, ChildStderr}};
use std::{sync::{Arc, Mutex}, future::Future, result::{Result}, error::{Error}};
use async_trait::async_trait;

// there are two type of pwn challenge
// 1. use stdin/stdout/stderr io to community with challenger. we need spawn a task and forword stdio for than
// 2. use a independent port. should wrapping challenge with a docker and do port forwrod for that.

struct PwnTask<T> 
where T: Task {
    child_process: Child,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    stdout: Option<Arc<Mutex<ChildStdout>>>,
    stderr: Option<Arc<Mutex<ChildStderr>>>,
    timeout: u32, // zero for no timeout 
    task: T
}


#[async_trait]
trait Task {
    async fn spawn_task(self: &mut Self) -> Child;
    async fn graceful_shutdown(self: &mut Self) -> Result<(), Box<dyn Error>>;
    fn force_shutdown(self: &mut Self);    // force shutdown should down in time
}

#[async_trait]
trait TaskManager<T: Task> {
    fn new(task: T) -> PwnTask<T>;
} 