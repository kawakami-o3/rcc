#!/bin/sh
set -x
#echo '1+2-3' | RUST_BACKTRACE=1 cargo run -q -- -a
#echo 'printf("a");3;' | RUST_BACKTRACE=1 cargo run -q -- > tmp.s

#echo 'int a=61;int *b=&a;*b;' | RUST_BACKTRACE=1 cargo run -q -- > tmp.s
#echo 'char *c="ab";*c;' | RUST_BACKTRACE=1 cargo run -q -- > tmp.s
#RUST_BACKTRACE=1 cargo run -q '5+20-4'
#RUST_BACKTRACE=1 cargo run -q 'a=2; return a;'
#RUST_BACKTRACE=1 cargo run -q 'a=2; b=3+2; return a*b;'
#RUST_BACKTRACE=1 cargo run -q 'if (0) return 2; return 3;'
RUST_BACKTRACE=1 cargo run -q 'if (0) return 2; else return 3;'
#RUST_BACKTRACE=1 cargo run -q 'return 2*3+4;'
#RUST_BACKTRACE=1 cargo run -q '0'

#cat tmp.s

#gcc -g3 -o tmp.out test/driver.c tmp.s -undefined dynamic_lookup

#gcc -g3 -o tmp.out test/driver.c tmp.s && ./tmp.out
