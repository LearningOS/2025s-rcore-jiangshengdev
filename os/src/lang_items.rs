//! panic 处理器。

use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
/// panic 处理器。
///
/// # 参数
/// * `info` - panic 信息。
///
/// # 返回
/// 永不返回（发散函数）。

fn panic(info: &PanicInfo) -> ! {

    if let Some(location) = info.location() {

        println!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {

        println!("[kernel] Panicked: {}", info.message().unwrap());
    }

    shutdown()
}
