### 功能实现总结

- spawn
  - 主要利用已有的 TaskControlBlock::new 方法来创建新进程
  - 然后绑定父子关系，加入到待运行队列即可
- stride
  - priority 优先级，数字越大的越优先，需要保存在 tcb 上
  - 优先级越高，每次走的 pass 越少
  - 优先级高的，每次走的少，其 stride 数字小，会执行更多的次数

### 问答作业

stride 算法深入

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存
stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

- 实际情况是轮到 p1 执行吗？为什么？
  - p2.stride = 250 + 10 = 4（溢出）
  - 导致实际是 p2 继续执行，因为 p2 的数字更小

我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， **在不考虑溢出的情况下** , 在进程优先级全部 >= 2
的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

- 为什么？尝试简单说明（不要求严格证明）。

  - 因为如果按 P.pass = BigStride / P.priority
  - 那么在极端情况下优先级最小为 2，pass 最大步长为 BigStride / 2
  - 然后根据 stride 小的优先执行的情况下，初始情况下假设 2 者 stride 相等，
  - 随机挑选一个执行，执行之后 2 者最多相差 BigStride / 2
  - 这时候，必然会执行另外一个，2 者距离必然减小
  - 在有多个进程执行时，必然最小者移动，其也不能移动超过 BigStride / 2
  - 所以不会比其他的多 BigStride / 2 以上
  - 并且如果他超过了其他的，必然下一次不是他移动

- 已知以上结论，**考虑溢出的情况下**，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop
  方法能返回真正最小的 Stride。补全下列代码中的 `partial_cmp` 函数，假设两个 Stride 永远不会相等。

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = self.0.inner_exclusive_access().stride;
        let b = other.0.inner_exclusive_access().stride;
        // BinaryHeap 默认是大顶堆（最大堆），但我们需要最小的 stride。
        // 通过反转 Ordering 实现最小堆效果：stride 越小，cmp 返回 Ordering::Greater。
        // 计算环形距离 diff（支持回绕判断）
        let diff = b.wrapping_sub(a);
        if diff == 0 {
            Ordering::Equal
        } else if diff < BIG_STRIDE / 2 {
            // diff < BIG_STRIDE/2 表示从 a 到 b 的距离小于半圈，因此视作 a < b
            Ordering::Greater
        } else {
            // 否则表示 a 距离 b 超过半圈，视作 a > b
            Ordering::Less
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: `(125 < 255) == false`, `(129 < 255) == true`.
