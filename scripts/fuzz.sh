#!/bin/bash

if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Usage: fuzz.h num_jobs path/to/config"
    exit
fi

if [ -z "$LOGS_DIR" ]; then
    LOGS_DIR=./logs
    mkdir -p $LOGS_DIR
fi

if [ -z "$PROJECT_ROOT" ]; then
    export PROJECT_ROOT="./"
fi

cp "$2" "$PROJECT_ROOT/agent/fuzz_config.h"

for i in $(seq "$1"); do
   if (( i % 3 == 0 )); then
        export FUZZ_TRACE_PC="TRACEPC-";
    else
        unset FUZZ_TRACE_PC
    fi
   
	if (( i % 2 == 0 )); then
        export MUTATE_SYSCALLS="1";
    else
        unset MUTATE_SYSCALLS
    fi

    if (( i % 6 == 0)); then
        export FUZZ_ABORT_ERRORS="ABORTERRORS-"
    else
        unset FUZZ_ABORT_ERRORS
    fi   

    tmpd=$(mktemp -d "$LOGS_DIR/kernel-fuzzer-$i-$FUZZ_TRACE_PC$FUZZ_ABORT_ERRORS$KASAN-XXXXX")
    export PORT=$(( i + 10100 ))
	"${BASH_SOURCE%/*}/run.sh" &> "$tmpd/output" &
 sleep 1;
done

echo "Initialized $1 Workers with Config: $2"
sleep 1
ls -tr "$LOGS_DIR"/*/output | tail -n"$1" | xargs tail -fq
wait
