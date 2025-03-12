#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

#define PAGEMAP_LENGTH              8

//# 虚拟地址转化成物理地址
size_t virt_to_phys(void* vaddr){
        unsigned long paddr = 0;
        int page_size, page_shift = -1;
        //# /proc/self/pagemap 记录当前进程的虚拟地址到物理地址的映射
        FILE *pagemap = fopen("/proc/self/pagemap", "rb");
        //# 使用 sysconf(_SC_PAGESIZE) 获取系统的页面大小
        page_size = sysconf(_SC_PAGESIZE);
        //# 每个页面条目占 PAGEMAP_LENGTH 字节
        size_t offset = ((size_t)vaddr / page_size) * PAGEMAP_LENGTH;
        fseek(pagemap, (unsigned long)offset, SEEK_SET);
        if (fread(&paddr, 1, (PAGEMAP_LENGTH-1), pagemap) < (PAGEMAP_LENGTH-1)) {
                perror("fread fails. ");
                exit(0);
        }
        paddr = paddr & 0x7fffffffffffff;
        /* printf("physical frame address is 0x%lx\n", paddr); */

        offset = (size_t)vaddr % page_size;

        /* PAGE_SIZE = 1U << PAGE_SHIFT */
        while (!((1UL << ++page_shift) & page_size));

        paddr = (unsigned long)((unsigned long)paddr << page_shift) + offset;
        //printf("physical address is 0x%lx\n", paddr);
        fclose(pagemap);
        return paddr;
}
