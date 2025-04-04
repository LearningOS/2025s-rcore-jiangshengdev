//! batch subsystem

use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::*;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        let len = KERNEL_STACK_SIZE;
        let addr = self.data.as_ptr() as usize;

        addr + len
    }

    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        let sp = self.get_sp();
        let size = core::mem::size_of::<TrapContext>();

        let cx_ptr = (sp - size) as *mut TrapContext;

        unsafe {
            *cx_ptr = cx;
        }

        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    pub fn print_app_info(&self) {
        println!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            println!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            println!("All applications completed!");
            use crate::board::QEMUExit;
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }

        println!("[kernel] Loading app_{}", app_id);

        let start = APP_BASE_ADDRESS as *mut u8;
        let limit = APP_SIZE_LIMIT;

        // clear app area
        let text = core::slice::from_raw_parts_mut(start, limit);
        text.fill(0);

        let data = self.app_start[app_id] as *const u8;
        let len = self.app_start[app_id + 1] - self.app_start[app_id];
        let app_src = core::slice::from_raw_parts(data, len);

        let size = app_src.len();
        let app_dst = core::slice::from_raw_parts_mut(start, size);
        app_dst.copy_from_slice(app_src);

        // Memory fence about fetching the instruction memory
        // It is guaranteed that a subsequent instruction fetch must
        // observes all previous writes to the instruction memory.
        // Therefore, fence.i must be executed after we have loaded
        // the code of the next app into the instruction memory.
        // See also: riscv non-priv spec chapter 3, 'Zifencei' extension.
        asm!("fence.i");
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();
            }

            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();

            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];

            let data = num_app_ptr.add(1);
            let len = num_app + 1;

            let app_start_raw: &[usize] = core::slice::from_raw_parts(data, len);

            app_start[..=num_app].copy_from_slice(app_start_raw);

            AppManager {
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

/// init batch subsystem
pub fn init() {
    print_app_info();
}

/// print apps info
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// run next app
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();

    unsafe {
        app_manager.load_app(current_app);
    }

    app_manager.move_to_next_app();
    drop(app_manager);

    // before this we have to drop local variables related to resources manually
    // and release the resources
    extern "C" {
        fn __restore(cx_addr: usize);
    }

    unsafe {
        let entry = APP_BASE_ADDRESS;
        let sp = USER_STACK.get_sp();

        let context = TrapContext::app_init_context(entry, sp);

        let cx_addr = KERNEL_STACK.push_context(context) as *const _ as usize;

        __restore(cx_addr);
    }

    panic!("Unreachable in batch::run_current_app!");
}
