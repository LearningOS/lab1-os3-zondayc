//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see [`__switch`]. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use crate::syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_WRITE, SYSCALL_TASK_INFO, SYSCALL_YIELD};
use crate::timer::{get_time, get_time_ms};
use lazy_static::*;
pub use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus, CurTaskInfo};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {//变量与常量分离
    /// total number of tasks
    num_app: usize,//常量
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,//变量
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            task_info: CurTaskInfo::zero_init(),
        }; MAX_APP_NUM];
        for (i, t) in tasks.iter_mut().enumerate().take(num_app) {
            t.task_cx = TaskContext::goto_restore(init_app_cx(i));// 保存传入的 sp，并将 ra 设置为 __restore 的入口地址
            t.task_status = TaskStatus::Ready;//上面保存到ra之后再调用就可以通过ra直接到相应的上下文了
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        task0.task_info.begin_time=get_time_ms();
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();//获取inner来进行修改
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.tasks[next].task_info.begin_time=match inner.tasks[next].task_info.begin_time {
                a if a>0=>a,
                _=>get_time_ms(),
            };
            info!("time is {}",inner.tasks[next].task_info.begin_time);
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);//注意这里需要drop掉可变引用，不然inner不会被销毁，下一次就不能进行切换
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);//这里直接调用_switch切换就行了
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    fn get_current_process(&self)->TaskControlBlock{
        let inner=self.inner.exclusive_access();
        let current=inner.current_task;
        inner.tasks[current].clone()
    }

    fn update_info(&self, syscall_id:usize){
        let mut inner=self.inner.exclusive_access();
        let current=inner.current_task;
        match syscall_id {
            SYSCALL_EXIT=>inner.tasks[current].task_info.sys_exit+=1,
            SYSCALL_GET_TIME=>inner.tasks[current].task_info.sys_time+=1,
            SYSCALL_WRITE=>inner.tasks[current].task_info.sys_write+=1,
            SYSCALL_TASK_INFO=>inner.tasks[current].task_info.sys_info+=1,
            SYSCALL_YIELD=>inner.tasks[current].task_info.sys_yield+=1,
            _=>panic!("error syscall!"),
        }
    }
    // LAB1: Try to implement your function to update or get task info!
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn update_task_info(syscall_id:usize){
    TASK_MANAGER.update_info(syscall_id);
}

pub fn get_current_tcb()->TaskControlBlock{
    TASK_MANAGER.get_current_process()
}

// LAB1: Public functions implemented here provide interfaces.
// You may use TASK_MANAGER member functions to handle requests.