use crate::task::pwn_task::{Task, TaskStatus};
use std::{sync::{Arc, Mutex}, vec::Vec};
use async_trait::async_trait;
use tokio::process::Command;
use anyhow::Result;
struct Vm {
    vm_name: String,
    vmx_path: String,
    ip: String,
}

pub struct VmTask {
    vm: Option<Vm>,
    status: TaskStatus,
}

const CMD_PATH: &str = "C:\\Program Files (x86)\\VMware\\VMware Workstation\\vmrun.exe";
static mut VM_POOL:Option<Arc<Mutex<Vec<Vm>>>> = None;

#[async_trait]
impl Task for VmTask {
    async fn spawn_task(self: &mut Self) -> Result<()> {
        self.status = TaskStatus::STARTING;

        let mut child = Command::new(CMD_PATH).arg("-T").arg("ws").arg("start").arg(&self.vm.as_ref().unwrap().vmx_path).spawn().expect("faild to run vmrun.exe");
        let status = child.wait().await;
        match status {
            Ok(status) => println!("start vmrun {}", status.success()),
            Err(er) => println!("vmrun error {}", er),
        };

        self.status = TaskStatus::RUNNING;
        Ok(())
    }

    async fn shutdown(self: &mut Self) -> Result<()>{
        self.status = TaskStatus::STOPPING;

        let mut child = Command::new(CMD_PATH).arg("-T").arg("ws").arg("revertToSnapshot").arg(&self.vm.as_ref().unwrap().vmx_path).arg("snapshoot1").spawn().expect("faild to run vmrun.exe");
        let status = child.wait().await;
        match status {
            Ok(status) => println!("shutdown vmrun {}", status.success()),
            Err(er) => println!("vmrun error {}", er),
        };

        self.status = TaskStatus::TERMINAL;

        let p = get_pool();
        let mut vm_vec = p.lock().unwrap();
        vm_vec.push(self.vm.take().unwrap());
        println!("push back, now len {}", vm_vec.len());

        Ok(())
    }
    fn task_status(self: &Self) -> TaskStatus {
        self.status
    }
}

impl VmTask {
    pub fn new() -> VmTask {
        let p = get_pool();
        let mut vm_vec = p.lock().unwrap();
        let task = VmTask{vm:Some(vm_vec.pop().unwrap()), status: TaskStatus::PENDING};
        println!("pop out, now len {}", vm_vec.len());
        task
    }

    pub fn get_ip(self: &Self) -> String {
        self.vm.as_ref().unwrap().ip.clone()
    }
}

unsafe fn pool_init() -> &'static Arc<Mutex<Vec<Vm>>> {
    let mut pool = Vec::new();
    for i in 1..=25 {
        let vm = Vm{vm_name: format!("ebs{}", i), vmx_path: format!("D:\\ebs{}\\ebs.vmx", i),ip: format!("192.168.145.{}", 128+i) };
        pool.push(vm);
    }
    VM_POOL = Some(Arc::new(Mutex::new(pool)));
    VM_POOL.as_ref().unwrap()
}

fn get_pool() -> Arc<Mutex<Vec<Vm>>> {
        // give back to pool
        let pool;
        unsafe {
            pool = match &VM_POOL {
                Some(p) => p,
                None => pool_init(),
            };
        }
        let p  = pool.clone();
        p
}