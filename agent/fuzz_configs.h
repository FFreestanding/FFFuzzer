#ifndef FUZZ_CONFIGS_H
#define FUZZ_CONFIGS_H

#include <stdint.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/eventfd.h>
#include <arpa/inet.h>


typedef struct fdconf {
    const char *path;
    int flags;
} fdconf;

typedef struct scconf {
    int nr;//# 系统调用号
    int args;//# 参数个数
    int mask_enabled;
    uint64_t mask[6];//# 与运算，过滤
    int min_enabled;
    uint64_t min[6];
    int identity_arg;
} scconf;

#include "fuzz_config.h"


#endif
