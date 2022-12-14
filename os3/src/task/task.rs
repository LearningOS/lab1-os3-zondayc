//! Types related to task management

use super::TaskContext;

#[derive(Clone, Copy)]
pub struct CurTaskInfo{
    pub sys_write:  u32,
    pub sys_exit:   u32,
    pub sys_info:   u32,
    pub sys_time:   u32,
    pub sys_yield:  u32,
    pub begin_time: usize,
}

#[derive(Copy, Clone)]
/// task control block structure
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    // LAB1: Add whatever you need about the Task.
    pub task_info: CurTaskInfo,
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

impl CurTaskInfo{
    pub fn zero_init() -> Self{
        CurTaskInfo { sys_write: 0, sys_exit: 0, sys_info: 0, sys_time: 0, sys_yield: 0, begin_time: 0 }
    }
}