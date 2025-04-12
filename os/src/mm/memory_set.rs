//! [`MapArea`] 和 [`MemorySet`] 的实现。

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

extern "C" {

    fn stext();

    fn etext();

    fn srodata();

    fn erodata();

    fn sdata();

    fn edata();

    fn sbss_with_stack();

    fn ebss();

    fn ekernel();

    fn strampoline();

}

lazy_static! {
    /// 内核的初始内存映射（内核地址空间）
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

/// 地址空间

pub struct MemorySet {
    pub(crate) page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// 创建一个新的空 `MemorySet`。

    pub fn new_bare() -> Self {

        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// 获取页表令牌

    pub fn token(&self) -> usize {

        self.page_table.token()
    }

    /// 假设没有冲突。

    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {

        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {

        let page_table = &mut self.page_table;

        map_area.map(page_table);

        if let Some(data) = data {

            map_area.copy_data(page_table, data);
        }

        self.areas.push(map_area);
    }

    /// 注意，跳板页不被 areas 收集。

    fn map_trampoline(&mut self) {

        let trampoline = TRAMPOLINE;

        let strampoline = strampoline as usize;

        let virt_addr = VirtAddr::from(trampoline);

        let phys_addr = PhysAddr::from(strampoline);

        let virt_page_num = virt_addr.into();

        let phys_page_num = phys_addr.into();

        let flags = PTEFlags::R | PTEFlags::X;

        self.page_table.map(virt_page_num, phys_page_num, flags);
    }

    /// 不包含内核栈。

    pub fn new_kernel() -> Self {

        let mut memory_set = Self::new_bare();

        // 映射跳板页
        memory_set.map_trampoline();

        // 调用 debug 模块中的打印函数
        print_mapped_page(
            &memory_set.page_table,
            VirtAddr::from(TRAMPOLINE),
            "Trampoline",
            color::GREEN,
        );

        let stext = stext as usize;

        let etext = etext as usize;

        let srodata = srodata as usize;

        let erodata = erodata as usize;

        let sdata = sdata as usize;

        let edata = edata as usize;

        let sbss_with_stack = sbss_with_stack as usize;

        let ebss = ebss as usize;

        // 映射内核各段
        info!(".text [{:#x}, {:#x})", stext, etext);

        info!(".rodata [{:#x}, {:#x})", srodata, erodata);

        info!(".data [{:#x}, {:#x})", sdata, edata);

        info!(".bss [{:#x}, {:#x})", sbss_with_stack, ebss);

        info!("mapping .text section");

        {

            let start_va = stext.into();

            let end_va = etext.into();

            let perm = MapPermission::R | MapPermission::X;

            let area = MapArea::new(start_va, end_va, MapType::Identical, perm);

            memory_set.push(area, None);

            print_area_mapping(".text", &memory_set.page_table, stext, etext, color::CYAN);
        }

        info!("mapping .rodata section");

        {

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

    /// 包括 ELF 中的各段、跳板页、TrapContext 和用户栈，
    /// 同时返回用户栈顶指针和入口点。

    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {

        let mut memory_set = Self::new_bare();

        // 映射跳板页
        memory_set.map_trampoline();

        // 新增：打印应用程序的跳板映射
        print_mapped_page(
            &memory_set.page_table,
            VirtAddr::from(TRAMPOLINE),
            "Trampoline (App)",
            color::GREEN,
        );

        // 映射 ELF 的程序头，带有 U 标志
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();

        let elf_header = elf.header;

        let magic = elf_header.pt1.magic;

        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "无效的 ELF 文件！");

        let ph_count = elf_header.pt2.ph_count();

        let mut max_end_vpn = VirtPageNum(0);

        for i in 0..ph_count {

            let ph = elf.program_header(i).unwrap();

            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {

                // 新增：计算起止地址用于打印
                let start_addr = ph.virtual_addr() as usize;

                let end_addr = (ph.virtual_addr() + ph.mem_size()) as usize;

                let start_va: VirtAddr = start_addr.into();

                let end_va: VirtAddr = end_addr.into();

                let mut map_perm = MapPermission::U;

                let ph_flags = ph.flags();

                if ph_flags.is_read() {

                    map_perm |= MapPermission::R;
                }

                if ph_flags.is_write() {

                    map_perm |= MapPermission::W;
                }

                if ph_flags.is_execute() {

                    map_perm |= MapPermission::X;
                }

                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

                max_end_vpn = map_area.vpn_range.get_end();

                let start = ph.offset();

                let len = ph.file_size();

                let end = start + len;

                let data = Some(&elf.input[start as usize..end as usize]);

                memory_set.push(map_area, data);

                // 新增：根据当前加载段地址范围获取对应的 ELF section 名称，只保留 .text, .rodata, .data, .bss
                let mut section_names = Vec::new();

                for section in elf.section_iter() {

                    if let Ok(name) = section.get_name(&elf) {

                        if name == ".text" || name == ".rodata" || name == ".data" || name == ".bss"
                        {

                            let sec_addr = section.address();

                            if sec_addr >= ph.virtual_addr()
                                && sec_addr < (ph.virtual_addr() + ph.mem_size())
                            {

                                section_names.push(name);
                            }
                        }
                    }
                }

                let seg_name = if section_names.is_empty() {

                    "User program segment".to_string()
                } else {

                    section_names.join(", ")
                };

                // 新增：为不同的段名称使用不同颜色
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

                // 修改：用户程序段使用计算出的 mapping_color 刻画，并打印对应 ELF segment 的 section name
                print_area_mapping(
                    &seg_name,
                    &memory_set.page_table,
                    start_addr,
                    end_addr,
                    mapping_color,
                );
            }
        }

        // 映射用户栈，带有 U 标志
        let max_end_va: VirtAddr = max_end_vpn.into();

        let mut user_stack_bottom: usize = max_end_va.into();

        // 保护页
        user_stack_bottom += PAGE_SIZE;

        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;

        {

            let start_va = user_stack_bottom.into();

            let end_va = user_stack_top.into();

            let map_perm = MapPermission::R | MapPermission::W | MapPermission::U;

            let area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

            memory_set.push(area, None);

            // 修改：用户栈使用 BRIGHT_YELLOW 刻画
            print_area_mapping(
                "User stack",
                &memory_set.page_table,
                user_stack_bottom,
                user_stack_top,
                color::BRIGHT_YELLOW,
            );
        }

        // 用于 sbrk
        {

            let start_va = user_stack_top.into();

            let end_va = user_stack_top.into();

            let map_perm = MapPermission::R | MapPermission::W | MapPermission::U;

            let area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

            memory_set.push(area, None);

            // 修改：sbrk 区使用 BRIGHT_BLUE 刻画
            print_area_mapping(
                "User sbrk",
                &memory_set.page_table,
                user_stack_top,
                user_stack_top,
                color::BRIGHT_BLUE,
            );
        }

        // 映射 TrapContext
        {

            let start_va = TRAP_CONTEXT_BASE.into();

            let end_va = TRAMPOLINE.into();

            let map_perm = MapPermission::R | MapPermission::W;

            let area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);

            memory_set.push(area, None);

            // 修改：TrapContext 使用 BRIGHT_MAGENTA 刻画
            print_area_mapping(
                "TrapContext",
                &memory_set.page_table,
                TRAP_CONTEXT_BASE,
                TRAMPOLINE,
                color::BRIGHT_MAGENTA,
            );
        }

        let entry_point = elf.header.pt2.entry_point() as usize;

        (memory_set, user_stack_top, entry_point)
    }

    /// 通过写入 satp CSR 寄存器切换页表。

    pub fn activate(&self) {

        let satp = self.page_table.token();

        unsafe {

            satp::write(satp);

            asm!("sfence.vma");
        }
    }

    /// 将虚拟页号转换为页表项

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {

        self.page_table.translate(vpn)
    }

    /// 将区域缩小至新的结束边界
    #[allow(unused)]

    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {

        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {

            area.shrink_to(&mut self.page_table, new_end.ceil());

            true
        } else {

            false
        }
    }

    /// 将区域扩展至新的结束边界
    #[allow(unused)]

    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {

        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {

            area.append_to(&mut self.page_table, new_end.ceil());

            true
        } else {

            false
        }
    }
}

/// 映射区域结构，控制一段连续的虚拟内存

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {

        let start_vpn: VirtPageNum = start_va.floor();

        let end_vpn: VirtPageNum = end_va.ceil();

        let vpn_range = VPNRange::new(start_vpn, end_vpn);

        let data_frames = BTreeMap::new();

        Self {
            vpn_range,
            data_frames,
            map_type,
            map_perm,
        }
    }

    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {

        let ppn: PhysPageNum;

        match self.map_type {
            MapType::Identical => {

                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {

                let frame = frame_alloc().unwrap();

                ppn = frame.ppn;

                self.data_frames.insert(vpn, frame);
            }
        }

        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();

        page_table.map(vpn, ppn, pte_flags);
    }

    #[allow(unused)]

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {

        if self.map_type == MapType::Framed {

            self.data_frames.remove(&vpn);
        }

        page_table.unmap(vpn);
    }

    pub fn map(&mut self, page_table: &mut PageTable) {

        for vpn in self.vpn_range {

            self.map_one(page_table, vpn);
        }
    }

    #[allow(unused)]

    pub fn unmap(&mut self, page_table: &mut PageTable) {

        for vpn in self.vpn_range {

            self.unmap_one(page_table, vpn);
        }
    }

    #[allow(unused)]

    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {

        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {

            self.unmap_one(page_table, vpn)
        }

        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    #[allow(unused)]

    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {

        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {

            self.map_one(page_table, vpn)
        }

        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    /// 数据：起始对齐但可能长度较短
    /// 假设所有帧在之前已被清零

    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {

        assert_eq!(self.map_type, MapType::Framed);

        let mut start: usize = 0;

        let mut current_vpn = self.vpn_range.get_start();

        let len = data.len();

        loop {

            let end = len.min(start + PAGE_SIZE);

            let src = &data[start..end];

            let pte = page_table.translate(current_vpn).unwrap();

            let ppn = pte.ppn();

            let bytes_array = ppn.get_bytes_array();

            let dst = &mut bytes_array[..src.len()];

            dst.copy_from_slice(src);

            start += PAGE_SIZE;

            if start >= len {

                break;
            }

            current_vpn.step();
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// 内存集的映射类型：恒等映射或帧映射

pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    /// 映射权限，对应于 PTE 中的：`R W X U`
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

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {

    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);

    let bottom = top - KERNEL_STACK_SIZE;

    (bottom, top)
}

/// 内核空间中的重映射测试
#[allow(unused)]

pub fn remap_test() {

    let mut kernel_space = KERNEL_SPACE.exclusive_access();

    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();

    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();

    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();

    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);

    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);

    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);

    println!("重映射测试通过！");
}
