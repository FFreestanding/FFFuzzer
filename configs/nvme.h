char     *conf_name = "nvme";
fdconf   conf_fds[] = {{"/dev/nvme0n1", O_RDWR},{"/dev/nvme-fabrics", O_RDWR},{"/dev/nvme0", O_RDWR},{"/dev/ng0n1", O_RDWR}};
uint64_t conf_initscs[][6] = {{}};
scconf   conf_scs[] = {
    {.nr = __NR_ioctl, .args = 3, .mask_enabled = 1, .mask = {0xFFF, -1, -1, -1, -1, -1}},
    {.nr = __NR_mmap, .args = 6, .mask_enabled = 1, .mask = {0, 0xF000, PROT_READ | PROT_WRITE, MAP_SHARED|MAP_POPULATE, 0xFFFF, -1}},
    {.nr = __NR_close, .args = 1, .mask_enabled = 1, .mask = {0xFFF, -1, -1, -1, -1, -1}},
    {.nr = __NR_fstat, .args = 2, .mask_enabled = 1, .mask = {0xFFF, -1, -1, -1, -1, -1}},
    {.nr = __NR_read, .args = 3, .mask_enabled = 1, .mask = {0xFFF, -1, 0xFFFF, -1, -1, -1}},
    {.nr = __NR_write, .args = 3, .mask_enabled = 1, .mask = {0xFFF, -1, 0xFFFF, -1, -1, -1}},
};
