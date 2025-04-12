//! RISC-V 定时器相关功能

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use riscv::register::time;

/// 每秒的时钟滴答数

const TICKS_PER_SEC: usize = 100;

#[allow(dead_code)]
/// 每秒的毫秒数

const MSEC_PER_SEC: usize = 1000;

/// 每秒的微秒数
#[allow(dead_code)]

const MICRO_PER_SEC: usize = 1_000_000;

/// 获取当前时间（以时钟滴答计）

pub fn get_time() -> usize {

    time::read()
}

/// 获取当前时间（以毫秒计）
#[allow(dead_code)]

pub fn get_time_ms() -> usize {

    time::read() * MSEC_PER_SEC / CLOCK_FREQ
}

/// 获取当前时间（以微秒计）
#[allow(dead_code)]

pub fn get_time_us() -> usize {

    time::read() * MICRO_PER_SEC / CLOCK_FREQ
}

/// 设置下一次时钟中断

pub fn set_next_trigger() {

    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
