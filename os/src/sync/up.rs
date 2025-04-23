//! 单核处理器内部可变性原语。
use core::cell::{RefCell, RefMut};

/// 将静态数据结构包裹其中，使我们能够在不使用 `unsafe` 的情况下访问它。
///
/// 仅应在单核处理器中使用。
///
/// 若要获得内部数据的可变引用，请调用 `exclusive_access`。
pub struct UPSafeCell<T> {
    /// 内部数据。
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// # Safety
    /// 用户需保证内部结构体仅在单核处理器中使用。
    ///
    /// # 参数
    /// * `value` - 要包裹的值。
    ///
    /// # 返回值
    /// 新的 UPSafeCell。
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    /// 若数据已被借用则 panic。
    ///
    /// # 返回值
    /// 内部数据的可变引用。
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
