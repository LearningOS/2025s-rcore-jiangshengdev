//! 文件和文件系统相关的系统调用

use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;

const FD_STDOUT: usize = 1;

/// 将长度为 `len` 的 buf 写入具有 `fd` 的文件

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {

    trace!("kernel: sys_write");

    match fd {
        FD_STDOUT => {

            let buffers = translated_byte_buffer(current_user_token(), buf, len);

            for buffer in buffers {

                print!("{}", core::str::from_utf8(buffer).unwrap());
            }

            len as isize
        }
        _ => {

            panic!("sys_write 中不支持的 fd！");
        }
    }
}
