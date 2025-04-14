### 功能实现总结

主要利用了 MapArea 上已经存在的 map 方法和 unmap 来实现 sys_mmap 和 sys_munmap
使用 Map 来保存内存映射数据，方便查找删除
使用 PageTable 的 translate 方法来从用户页表获取数据，以实现 sys_trace
使用 translated_byte_buffer 来获取跨页表项的数据，以实现 sys_get_time
