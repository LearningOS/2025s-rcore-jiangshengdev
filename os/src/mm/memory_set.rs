//! [`MapArea`] 和 [`MemorySet`] 的实现。
//!
//! 本模块实现了内存地址空间管理，包括：
//! - 地址空间 (MemorySet)，代表一个完整的地址空间，如内核空间或用户进程空间
//! - 映射区域 (MapArea)，表示地址空间中的一段连续虚拟内存区域
//! - 不同的映射类型和权限
//! - ELF 文件加载和地址空间创建

use super::{frame_alloc, print_area_mapping, print_mapped_page, FrameTracker};
use super::{PTEFlags, PageTable, PageTableEntry};
use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::{StepByOne, VPNRange};
use crate::config::{
    KERNEL_STACK_SIZE, MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT_BASE, USER_STACK_SIZE,
};
use crate::console::color;
use crate::sync::UPSafeCell;
use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::*;
use riscv::register::satp;

// 各个程序段的标记，从链接器脚本中导入
extern "C" {

    /// 代码段（.text）起始地址
    fn stext();

    /// 代码段（.text）结束地址
    fn etext();

    /// 只读数据段（.rodata）起始地址
    fn srodata();

    /// 只读数据段（.rodata）结束地址
    fn erodata();

    /// 数据段（.data）起始地址
    fn sdata();

    /// 数据段（.data）结束地址
    fn edata();

    /// 带栈的 BSS 段起始地址
    fn sbss_with_stack();

    /// BSS 段结束地址
    fn ebss();

    /// 整个内核结束地址
    fn ekernel();

    /// 跳板页起始地址
    fn strampoline();

}

lazy_static! {
    /// 内核的初始内存映射（内核地址空间）
    ///
    /// 通过 lazy_static! 延迟初始化，确保在第一次使用时才创建
    /// 使用 Arc 和 UPSafeCell 提供内部可变性和安全的共享访问
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

/// 地址空间
///
/// 表示一个完整的地址空间，包含页表和多个映射区域
/// 每个进程都有自己的 MemorySet 实例

pub struct MemorySet {
    /// 该地址空间的页表
    pub(crate) page_table: PageTable,
    /// 该地址空间中的所有映射区域
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// 创建一个新的空 `MemorySet`。
    ///
    /// 只包含一个空页表，没有任何映射区域
    /// 用作创建地址空间的起点

    pub fn new_bare() -> Self {

        // 创建一个包含空页表且没有映射区域的新地址空间
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// 获取页表令牌
    ///
    /// 返回可用于设置 satp 寄存器的值，使 MMU 使用该地址空间的页表进行地址转换
    ///
    /// # 返回值
    ///
    /// 页表令牌，可直接用于设置 satp 寄存器

    pub fn token(&self) -> usize {

        // 直接返回页表的令牌值
        self.page_table.token()
    }

    /// 在地址空间中插入一个帧映射区域
    ///
    /// # 参数
    ///
    /// * `start_va` - 起始虚拟地址
    /// * `end_va` - 结束虚拟地址（不含）
    /// * `permission` - 映射权限
    ///
    /// # 注意
    ///
    /// 假设新区域与现有区域不会冲突

    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {

        // 创建一个帧映射类型的映射区域，然后添加到地址空间中
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    /// 向地址空间添加一个映射区域
    ///
    /// # 参数
    ///
    /// * `map_area` - 要添加的映射区域
    /// * `data` - 可选的初始数据，将被复制到映射的物理内存中
    ///
    /// # 功能
    ///
    /// 1. 在页表中建立映射关系
    /// 2. 如果提供了数据，将数据复制到物理内存
    /// 3. 将映射区域添加到 areas 列表

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {

        // 获取页表的可变引用
        let page_table = &mut self.page_table;

        // 在页表中建立映射关系
        map_area.map(page_table);

        // 如果提供了数据，复制数据到对应的物理内存
        if let Some(data) = data {

            map_area.copy_data(page_table, data);
        }

        // 将区域添加到映射区域列表
        self.areas.push(map_area);
    }

    /// 将跳板页映射到地址空间
    ///
    /// 跳板页是一个特殊的页面，用于在用户空间和内核空间之间进行特权级切换
    /// 它被映射到虚拟地址空间的固定位置（由 TRAMPOLINE 常量定义）
    ///
    /// # 注意
    ///
    /// 跳板页不会被添加到 areas 集合中管理

    fn map_trampoline(&mut self) {

        // 获取跳板页的虚拟地址和物理地址
        let trampoline = TRAMPOLINE;

        let strampoline = strampoline as usize;

        // 转换为地址和页号
        let virt_addr = VirtAddr::from(trampoline);

        let phys_addr = PhysAddr::from(strampoline);

        let virt_page_num = virt_addr.into();

        let phys_page_num = phys_addr.into();

        // 设置跳板页为可读可执行权限，用于存放跳转代码
        let flags = PTEFlags::R | PTEFlags::X;

        // 在页表中建立映射
        self.page_table.map(virt_page_num, phys_page_num, flags);
    }

    /// 创建内核地址空间
    ///
    /// 创建并返回内核的地址空间，包含以下映射：
    /// 1. 跳板页 - 用于特权级切换
    /// 2. 内核 .text 段 - 包含内核代码
    /// 3. 内核 .rodata 段 - 包含只读数据
    /// 4. 内核 .data 段 - 包含可读写数据
    /// 5. 内核 .bss 段 - 包含零初始化数据和栈
    /// 6. 物理内存 - 从内核结束到内存结束的区域
    ///
    /// # 注意
    ///
    /// 不包含内核栈映射，内核栈由每个应用程序单独分配

    pub fn new_kernel() -> Self {

        // 创建一个空的地址空间
        let mut memory_set = Self::new_bare();

        // 映射跳板页
        memory_set.map_trampoline();

        // 打印跳板页映射信息
        print_mapped_page(
            &memory_set.page_table,
            VirtAddr::from(TRAMPOLINE),
            "Trampoline",
            color::GREEN,
        );

        // 获取各段的地址
        let stext = stext as usize;

        let etext = etext as usize;

        let srodata = srodata as usize;

        let erodata = erodata as usize;

        let sdata = sdata as usize;

        let edata = edata as usize;

        let sbss_with_stack = sbss_with_stack as usize;

        let ebss = ebss as usize;

        // 打印各段的地址范围信息
        info!(".text [{:#x}, {:#x})", stext, etext);

        info!(".rodata [{:#x}, {:#x})", srodata, erodata);

        info!(".data [{:#x}, {:#x})", sdata, edata);

        info!(".bss [{:#x}, {:#x})", sbss_with_stack, ebss);

        info!("mapping .text section");

        {

            // 映射代码段（.text）- 可读可执行
            let start_va = stext.into();

            let end_va = etext.into();

            let perm = MapPermission::R | MapPermission::X;

            // 为代码段使用恒等映射（虚拟地址等于物理地址）
            let area = MapArea::new(start_va, end_va, MapType::Identical, perm);

            memory_set.push(area, None);

            // 打印映射信息
            print_area_mapping(".text", &memory_set.page_table, stext, etext, color::CYAN);
        }

        info!("mapping .rodata section");

        {

            // 映射只读数据段（.rodata）- 只读
            let start_va = srodata.into();

            let end_va = erodata.into();

            let perm = MapPermission::R;

            let area = MapArea::new(start_va, end_va, MapType::Identical, perm);

            memory_set.push(area, None);

            print_area_mapping(
                ".rodata",
                &memory_set.page_table,
                srodata,
                erodata,
                color::YELLOW,
            );
        }

        info!("mapping .data section");

        {

            // 映射数据段（.data）- 可读可写
            let start_va = sdata.into();

            let end_va = edata.into();

            let perm = MapPermission::R | MapPermission::W;

            let area = MapArea::new(start_va, end_va, MapType::Identical, perm);

            memory_set.push(area, None);

            print_area_mapping(
                ".data",
                &memory_set.page_table,
                sdata,
                edata,
                color::MAGENTA,
            );
        }

        info!("mapping .bss section");

        {

            // 映射 BSS 段（.bss）- 可读可写
            let start_va = sbss_with_stack.into();

            let end_va = ebss.into();

            let perm = MapPermission::R | MapPermission::W;

            let area = MapArea::new(start_va, end_va, MapType::Identical, perm);

            memory_set.push(area, None);

            print_area_mapping(
                ".bss",
                &memory_set.page_table,
                sbss_with_stack,
                ebss,
                color::BLUE,
            );
        }

        info!("mapping physical memory");

        let ekernel = ekernel as usize;

        let memory_end = MEMORY_END;

        {

            // 映射剩余物理内存 - 可读可写
            // 从内核结束地址到物理内存结束地址
            let start_va = ekernel.into();

            let end_va = memory_end.into();

            let perm = MapPermission::R | MapPermission::W;

            let area = MapArea::new(start_va, end_va, MapType::Identical, perm);

            memory_set.push(area, None);

            print_area_mapping(
                "Physical memory",
                &memory_set.page_table,
                ekernel,
                memory_end,
                color::GREEN,
            );
        }

        memory_set
    }

    /// 从 ELF 文件创建用户进程地址空间
    ///
    /// 解析 ELF 文件，创建用户进程的地址空间，包括：
    /// 1. 程序各段（如 .text、.rodata、.data、.bss）
    /// 2. 用户栈
    /// 3. 用于 sbrk 系统调用的堆区域
    /// 4. TrapContext 区域
    /// 5. 跳板页
    ///
    /// # 参数
    ///
    /// * `elf_data` - ELF 文件的二进制数据
    ///
    /// # 返回值
    ///
    /// 返回一个元组，包含：
    /// 1. 创建的内存集（地址空间）
    /// 2. 用户栈顶指针
    /// 3. 程序入口点地址

    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {

        // 创建一个空的地址空间
        let mut memory_set = Self::new_bare();

        // 映射跳板页，用于特权级切换
        memory_set.map_trampoline();

        // 打印应用程序的跳板映射信息
        print_mapped_page(
            &memory_set.page_table,
            VirtAddr::from(TRAMPOLINE),
            "Trampoline (App)",
            color::GREEN,
        );

        // 解析 ELF 文件，映射程序各段
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();

        let elf_header = elf.header;

        // 检查 ELF 魔数是否正确
        let magic = elf_header.pt1.magic;

        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "无效的 ELF 文件！");

        // 获取程序头表数量
        let ph_count = elf_header.pt2.ph_count();

        // 记录最高的结束页号，用于确定用户栈的开始位置
        let mut max_end_vpn = VirtPageNum(0);

        // 遍历所有程序头
        for i in 0..ph_count {

            let ph = elf.program_header(i).unwrap();

            // 如果是可加载段 (LOAD)，则需要映射
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {

                // 计算段的虚拟地址范围
                let start_addr = ph.virtual_addr() as usize;

                let end_addr = (ph.virtual_addr() + ph.mem_size()) as usize;

                let start_va: VirtAddr = start_addr.into();

                let end_va: VirtAddr = end_addr.into();

                // 根据段的标志设置内存权限
                let mut map_perm = MapPermission::U; // 用户可访问
                let ph_flags = ph.flags();

                // 添加读、写、执行权限
                if ph_flags.is_read() {

                    map_perm |= MapPermission::R;
                }

                if ph_flags.is_write() {

                    map_perm |= MapPermission::W;
                }

                if ph_flags.is_execute() {

                    map_perm |= MapPermission::X;
                }

                // 创建映射区域 (帧映射类型)
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

                // 更新最高页号
                max_end_vpn = map_area.vpn_range.get_end();

                // 从 ELF 文件中提取段数据
                let start = ph.offset();

                let len = ph.file_size();

                let end = start + len;

                let data = Some(&elf.input[start as usize..end as usize]);

                // 添加区域并复制数据
                memory_set.push(map_area, data);

                // 确定当前段对应的 ELF 节区名称
                let mut section_names = Vec::new();

                for section in elf.section_iter() {

                    if let Ok(name) = section.get_name(&elf) {

                        // 只关注主要的几个节
                        if name == ".text" || name == ".rodata" || name == ".data" || name == ".bss"
                        {

                            let sec_addr = section.address();

                            // 检查节是否在当前段范围内
                            if sec_addr >= ph.virtual_addr()
                                && sec_addr < (ph.virtual_addr() + ph.mem_size())
                            {

                                section_names.push(name);
                            }
                        }
                    }
                }

                // 创建段名称字符串
                let seg_name = if section_names.is_empty() {

                    "User program segment".to_string()
                } else {

                    section_names.join(", ")
                };

                // 根据节名称选择颜色
                let mapping_color = if section_names.len() == 1 {

                    match section_names[0] {
                        ".text" => color::BRIGHT_RED,
                        ".rodata" => color::BRIGHT_YELLOW,
                        ".data" => color::BRIGHT_BLUE,
                        ".bss" => color::BRIGHT_MAGENTA,
                        _ => color::BRIGHT_GREEN,
                    }
                } else {

                    color::BRIGHT_GREEN
                };

                // 打印映射信息
                print_area_mapping(
                    &seg_name,
                    &memory_set.page_table,
                    start_addr,
                    end_addr,
                    mapping_color,
                );
            }
        }

        // 映射用户栈区域
        let max_end_va: VirtAddr = max_end_vpn.into();

        let mut user_stack_bottom: usize = max_end_va.into();

        // 添加一个页的间隙作为保护页，防止栈溢出到程序段
        user_stack_bottom += PAGE_SIZE;

        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;

        {

            // 为用户栈分配物理页，权限：用户可访问，可读可写
            let start_va = user_stack_bottom.into();

            let end_va = user_stack_top.into();

            let map_perm = MapPermission::R | MapPermission::W | MapPermission::U;

            let area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

            memory_set.push(area, None);

            // 打印用户栈信息
            print_area_mapping(
                "User stack",
                &memory_set.page_table,
                user_stack_bottom,
                user_stack_top,
                color::BRIGHT_YELLOW,
            );
        }

        // 映射用于 sbrk 系统调用的初始区域（初始大小为 0）
        {

            let start_va = user_stack_top.into();

            let end_va = user_stack_top.into();

            let map_perm = MapPermission::R | MapPermission::W | MapPermission::U;

            let area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

            memory_set.push(area, None);

            // 打印 sbrk 区域信息
            print_area_mapping(
                "User sbrk",
                &memory_set.page_table,
                user_stack_top,
                user_stack_top,
                color::BRIGHT_BLUE,
            );
        }

        // 映射陷阱上下文区域，用于保存进入内核时的用户状态
        {

            let start_va = TRAP_CONTEXT_BASE.into();

            let end_va = TRAMPOLINE.into();

            // 内核专用，用户态不可访问，可读可写
            let map_perm = MapPermission::R | MapPermission::W;

            let area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

            memory_set.push(area, None);

            // 打印陷阱上下文区域信息
            print_area_mapping(
                "TrapContext",
                &memory_set.page_table,
                TRAP_CONTEXT_BASE,
                TRAMPOLINE,
                color::BRIGHT_MAGENTA,
            );
        }

        // 获取程序入口点地址
        let entry_point = elf.header.pt2.entry_point() as usize;

        // 返回地址空间、用户栈顶地址和程序入口点
        (memory_set, user_stack_top, entry_point)
    }

    /// 通过写入 satp CSR 寄存器切换页表。
    ///
    /// 激活当前地址空间的页表，使 MMU 使用它进行地址转换
    /// 同时执行 sfence.vma 指令刷新 TLB，确保地址转换缓存失效

    pub fn activate(&self) {

        // 获取页表令牌值
        let satp = self.page_table.token();

        // 使用不安全代码块，直接操作 RISC-V 寄存器
        unsafe {

            // 写入 satp 寄存器，启用分页
            satp::write(satp);

            // 刷新 TLB（地址转换缓冲区）
            asm!("sfence.vma");
        }
    }

    /// 将虚拟页号转换为页表项
    ///
    /// 在当前地址空间中查找虚拟页号对应的页表项
    ///
    /// # 参数
    ///
    /// * `vpn` - 要查找的虚拟页号
    ///
    /// # 返回值
    ///
    /// 如果找到映射，返回对应的页表项
    /// 如果未找到映射，返回 None

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {

        // 查询页表，获取页表项
        self.page_table.translate(vpn)
    }

    /// 将区域缩小至新的结束边界
    ///
    /// 缩小指定的映射区域，释放不再使用的物理页帧
    ///
    /// # 参数
    ///
    /// * `start` - 区域的起始地址（用于标识区域）
    /// * `new_end` - 区域的新结束边界
    ///
    /// # 返回值
    ///
    /// 如果找到并成功缩小区域，返回 true
    /// 如果未找到指定区域，返回 false
    #[allow(unused)]

    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {

        // 查找起始地址匹配的区域
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {

            // 执行区域缩小操作
            area.shrink_to(&mut self.page_table, new_end.ceil());

            true
        } else {

            // 未找到匹配的区域
            false
        }
    }

    /// 将区域扩展至新的结束边界
    ///
    /// 扩展指定的映射区域，分配新的物理页帧
    ///
    /// # 参数
    ///
    /// * `start` - 区域的起始地址（用于标识区域）
    /// * `new_end` - 区域的新结束边界
    ///
    /// # 返回值
    ///
    /// 如果找到并成功扩展区域，返回 true
    /// 如果未找到指定区域，返回 false
    #[allow(unused)]

    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {

        // 查找起始地址匹配的区域
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {

            // 执行区域扩展操作
            area.append_to(&mut self.page_table, new_end.ceil());

            true
        } else {

            // 未找到匹配的区域
            false
        }
    }
}

/// 映射区域结构，控制一段连续的虚拟内存
///
/// 表示地址空间中连续的一段虚拟内存区域，包含映射类型、权限和对应的物理页帧

pub struct MapArea {
    /// 该区域覆盖的虚拟页号范围
    vpn_range: VPNRange,
    /// 该区域中帧映射类型的数据帧
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    /// 映射类型（恒等映射或帧映射）
    map_type: MapType,
    /// 映射权限
    map_perm: MapPermission,
}

impl MapArea {
    /// 创建一个新的映射区域
    ///
    /// # 参数
    ///
    /// * `start_va` - 起始虚拟地址
    /// * `end_va` - 结束虚拟地址（不含）
    /// * `map_type` - 映射类型（恒等映射或帧映射）
    /// * `map_perm` - 映射权限
    ///
    /// # 返回值
    ///
    /// 返回一个新的映射区域，但尚未建立实际的内存映射

    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {

        // 计算起始和结束的虚拟页号
        // floor() 向下取整，确保覆盖起始地址
        let start_vpn: VirtPageNum = start_va.floor();

        // ceil() 向上取整，确保覆盖终止地址前的所有内容
        let end_vpn: VirtPageNum = end_va.ceil();

        // 创建虚拟页号范围
        let vpn_range = VPNRange::new(start_vpn, end_vpn);

        // 创建空的数据帧映射
        let data_frames = BTreeMap::new();

        // 创建并返回映射区域
        Self {
            vpn_range,
            data_frames,
            map_type,
            map_perm,
        }
    }

    /// 映射区域中的单个虚拟页
    ///
    /// 根据映射类型，将单个虚拟页映射到物理页
    ///
    /// # 参数
    ///
    /// * `page_table` - 要操作的页表
    /// * `vpn` - 要映射的虚拟页号
    ///
    /// # 实现细节
    ///
    /// - 对于恒等映射类型，虚拟页号等于物理页号
    /// - 对于帧映射类型，会分配新的物理页帧

    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {

        // 根据映射类型确定物理页号
        let ppn: PhysPageNum;

        match self.map_type {
            MapType::Identical => {

                // 恒等映射：物理页号等于虚拟页号
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {

                // 帧映射：分配一个新的物理页帧
                let frame = frame_alloc().unwrap();

                ppn = frame.ppn;

                // 保存物理页帧的所有权，以便在取消映射时自动释放
                self.data_frames.insert(vpn, frame);
            }
        }

        // 将映射权限转换为页表项标志
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();

        // 在页表中建立映射
        page_table.map(vpn, ppn, pte_flags);
    }

    /// 解除区域中单个虚拟页的映射
    ///
    /// 从页表中移除虚拟页的映射，如果是帧映射类型，还会释放对应的物理页帧
    ///
    /// # 参数
    ///
    /// * `page_table` - 要操作的页表
    /// * `vpn` - 要解除映射的虚拟页号
    #[allow(unused)]

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {

        // 如果是帧映射类型，需要释放物理页帧
        if self.map_type == MapType::Framed {

            // 从数据帧映射中移除，会自动释放页帧（通过 Drop 特性）
            self.data_frames.remove(&vpn);
        }

        // 从页表中解除映射
        page_table.unmap(vpn);
    }

    /// 映射整个区域
    ///
    /// 将区域内的所有虚拟页映射到物理页
    ///
    /// # 参数
    ///
    /// * `page_table` - 要操作的页表

    pub fn map(&mut self, page_table: &mut PageTable) {

        // 遍历区域内的所有虚拟页号
        for vpn in self.vpn_range {

            // 映射每一个虚拟页
            self.map_one(page_table, vpn);
        }
    }

    /// 解除整个区域的映射
    ///
    /// 从页表中移除区域内所有虚拟页的映射
    ///
    /// # 参数
    ///
    /// * `page_table` - 要操作的页表
    #[allow(unused)]

    pub fn unmap(&mut self, page_table: &mut PageTable) {

        // 遍历区域内的所有虚拟页号
        for vpn in self.vpn_range {

            // 解除每一个虚拟页的映射
            self.unmap_one(page_table, vpn);
        }
    }

    /// 缩小区域至新的结束边界
    ///
    /// 将区域的结束点移动到较早的位置，释放多余的映射
    ///
    /// # 参数
    ///
    /// * `page_table` - 要操作的页表
    /// * `new_end` - 新的结束页号
    #[allow(unused)]

    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {

        // 遍历需要解除映射的虚拟页号范围（从新结束点到原始结束点）
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {

            // 解除每个页的映射
            self.unmap_one(page_table, vpn)
        }

        // 更新区域的虚拟页号范围
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    /// 扩展区域至新的结束边界
    ///
    /// 将区域的结束点移动到较晚的位置，创建新的映射
    ///
    /// # 参数
    ///
    /// * `page_table` - 要操作的页表
    /// * `new_end` - 新的结束页号
    #[allow(unused)]

    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {

        // 遍历需要新增映射的虚拟页号范围（从原始结束点到新结束点）
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {

            // 为每个页建立新的映射
            self.map_one(page_table, vpn)
        }

        // 更新区域的虚拟页号范围
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    /// 复制数据到映射的物理内存
    ///
    /// 将数据复制到区域映射的物理内存中，通常用于加载程序段
    ///
    /// # 参数
    ///
    /// * `page_table` - 当前使用的页表
    /// * `data` - 要复制的数据
    ///
    /// # 注意
    ///
    /// - 区域必须是帧映射类型
    /// - 数据长度可能小于整个区域的大小
    /// - 数据起始位置必须页对齐
    /// - 物理页帧之前已被清零

    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {

        // 确认这是帧映射类型的区域
        assert_eq!(self.map_type, MapType::Framed);

        // 起始偏移为 0
        let mut start: usize = 0;

        // 从区域的第一个虚拟页开始
        let mut current_vpn = self.vpn_range.get_start();

        // 数据总长度
        let len = data.len();

        // 循环复制数据到每个相关的物理页
        loop {

            // 计算当前页应该复制的数据长度（不超过一页或剩余数据长度）
            let end = len.min(start + PAGE_SIZE);

            // 获取要复制的数据片段
            let src = &data[start..end];

            // 通过页表获取当前虚拟页对应的页表项
            let pte = page_table.translate(current_vpn).unwrap();

            // 获取物理页号
            let ppn = pte.ppn();

            // 获取物理页的可变字节数组
            let bytes_array = ppn.get_bytes_array();

            // 获取目标内存区域的可变引用
            let dst = &mut bytes_array[..src.len()];

            // 复制数据
            dst.copy_from_slice(src);

            // 更新起始偏移
            start += PAGE_SIZE;

            // 如果所有数据都已复制完成，退出循环
            if start >= len {

                break;
            }

            // 移动到下一个虚拟页
            current_vpn.step();
        }
    }
}

/// 内存集的映射类型：恒等映射或帧映射
///
/// - 恒等映射：虚拟地址等于物理地址，主要用于内核空间
/// - 帧映射：为每个虚拟页分配新的物理页帧，主要用于用户空间
#[derive(Copy, Clone, PartialEq, Debug)]

pub enum MapType {
    /// 恒等映射，虚拟地址等于物理地址
    Identical,
    /// 帧映射，虚拟页与物理页之间建立映射关系
    Framed,
}

bitflags! {
    /// 映射权限，对应于 PTE 中的：`R W X U`
    ///
    /// 用于控制内存区域的访问权限
    pub struct MapPermission: u8 {
        /// 可读
        const R = 1 << 1;
        /// 可写
        const W = 1 << 2;
        /// 可执行
        const X = 1 << 3;
        /// 用户模式可访问
        const U = 1 << 4;
    }
}

/// 返回内核空间中内核栈的（底部，顶部）位置。
///
/// 每个应用程序都有自己的内核栈，用于处理从用户空间切换到内核空间时的上下文
///
/// # 参数
///
/// * `app_id` - 应用程序 ID
///
/// # 返回值
///
/// 返回一个元组，包含内核栈的底部地址和顶部地址

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {

    // 从 TRAMPOLINE 向下分配内核栈
    // 每个栈后面有一个保护页
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);

    // 底部是栈顶减去栈大小
    let bottom = top - KERNEL_STACK_SIZE;

    // 返回栈的底部和顶部地址
    (bottom, top)
}

/// 内核空间中的重映射测试
///
/// 验证内核地址空间映射是否正确配置了不同区域的权限：
/// - .text 段不可写
/// - .rodata 段不可写
/// - .data 段不可执行
#[allow(unused)]

pub fn remap_test() {

    // 获取内核空间的可变引用
    let mut kernel_space = KERNEL_SPACE.exclusive_access();

    // 获取三个关键段的中间地址
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();

    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();

    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();

    // 检查代码段不可写
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);

    // 检查只读数据段不可写
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);

    // 检查数据段不可执行
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);

    println!("重映射测试通过！");
}
