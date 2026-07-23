# KeyKeeper

macOS 菜单栏应用，聚合多个 AI 平台的 API 配额/余额查询。

> ⚠️ **当前状态**：早期开发中。审计已发现多个 P0 级问题（Volcano 适配器不可用、数据模型无法表达真实用量、多处 UI 语义错误）。修复计划见 [doc/audit-and-roadmap.md](doc/audit-and-roadmap.md)。

## 功能

- 聚合查询 DeepSeek、智谱AI、Qoder、火山方舟四个平台（Volcano/Qoder 修复中）
- 一键刷新查看所有平台余额
- 安全存储 API Key（macOS Keychain）
- 低额度通知（计划中，见 audit-and-roadmap.md P2-1）

## 技术栈

- **前端**: Vue 3 + TypeScript + TailwindCSS
- **后端**: Rust + Tauri v2
- **构建**: Vite + pnpm

## 开发

```bash
# 安装依赖
pnpm install

# 开发模式（前端 + Rust）
pnpm tauri dev

# 仅前端开发
pnpm dev

# 构建
pnpm build

# 打包 macOS .dmg
pnpm tauri build

# 运行测试
cd src-tauri && cargo test
```

## 项目结构

```
src/              # 前端 (Vue3 + TS)
src-tauri/        # 后端 (Rust)
  src/
    main.rs       # 入口 + 托盘 + 菜单
    commands.rs   # Tauri 命令
    models.rs     # 数据结构
    keystore.rs   # Keychain 存储
    scheduler.rs  # 并发调度器
    adapters/     # 平台适配器
doc/              # 设计与审计文档（见下）
```

## 文档

- [doc/KeyKeeper.md](doc/KeyKeeper.md) — 项目设计与架构
- [doc/audit-and-roadmap.md](doc/audit-and-roadmap.md) — 审计报告、Bug 修复计划、路线图
- [doc/ux-improvements.md](doc/ux-improvements.md) — 易用性改进建议

## 许可证

MIT
