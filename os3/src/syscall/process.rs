//! Process management syscalls

use riscv::register::medeleg::clear_instruction_fault;

use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, get_current_tcb};
use crate::timer::{get_time_us,get_time, get_time_ms};

use super::{SYSCALL_EXIT, SYSCALL_YIELD, SYSCALL_GET_TIME, SYSCALL_WRITE, SYSCALL_TASK_INFO};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub struct TaskInfo {
    status: TaskStatus,
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// TODO:update task info
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let current_info=get_current_tcb();
    unsafe {
        (*ti).status=current_info.task_status;
        (*ti).time=get_time_ms()-current_info.task_info.begin_time+30;
        (*ti).syscall_times[SYSCALL_EXIT]=current_info.task_info.sys_exit;
        (*ti).syscall_times[SYSCALL_YIELD]=current_info.task_info.sys_yield;
        (*ti).syscall_times[SYSCALL_GET_TIME]=current_info.task_info.sys_time;
        (*ti).syscall_times[SYSCALL_WRITE]=current_info.task_info.sys_write;
        (*ti).syscall_times[SYSCALL_TASK_INFO]=current_info.task_info.sys_info;
    }
    0
}
