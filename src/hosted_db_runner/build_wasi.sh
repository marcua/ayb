#!/usr/bin/env bash

# Also: apt install llvm-dev libclang-dev clang

export WASI_VERSION=20
export WASI_VERSION_FULL=20.0
export WASI_SDK_PATH=`pwd`/wasi-sdk-${WASI_VERSION_FULL}
export WASI_LIB=`pwd`/wasi-sdk-${WASI_VERSION_FULL}/lib
export WASI_SYSROOT="${WASI_SDK_PATH}/share/wasi-sysroot"
export CC="${WASI_SDK_PATH}/bin/clang --sysroot=${WASI_SYSROOT}"
export CC_wasm32_wasi="${CC}"
export RUST_BACKTRACE=1

wget -nc "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_VERSION}/wasi-sdk-${WASI_VERSION_FULL}-linux.tar.gz"
tar xvf wasi-sdk-${WASI_VERSION_FULL}-linux.tar.gz

cargo build --target "wasm32-wasi"
