#!/bin/sh
LIBFUZZER_SRC_DIR=$(dirname $0)
CXX="${CXX:-clang}"
for f in $LIBFUZZER_SRC_DIR/*.cpp; do
  $CXX -O2 -fno-omit-frame-pointer -std=c++11 -fPIE $f -c &
done
wait
rm -f libFuzzer.a
ar ru libFuzzer.a Fuzzer*.o
rm -f Fuzzer*.o

