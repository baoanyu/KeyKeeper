//! 二进制入口。
//!
//! 所有模块与逻辑都在 lib.rs —— 保持单一模块树，避免 bin/lib 各自声明一遍
//! 导致整个后端编译两次（`cargo test` 也会跑两遍）。

fn main() {
    keykeeper::run();
}
