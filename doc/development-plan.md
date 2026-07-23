# KeyKeeper 开发计划

> ⚠️ **本文档历史状态**：本开发计划已于 2026-07-23 完成。项目已从零实现到完整可运行的应用。当前代码状态请参阅 [KeyKeeper.md](KeyKeeper.md) 和 [CLAUDE.md](../../CLAUDE.md)。
>
> ⚠️ **审计后修正（2026-07-23）**：下方"完成状态"和"对抗性审查修复记录"记载的部分"已完成"项**并未真正完成**（Volcano HMAC 签名实现有误、低额度通知未接入前端、Qoder 认知可能有误）。完整问题清单与真实修复计划见 [audit-and-roadmap.md](audit-and-roadmap.md)。**本文档保留原样作为历史记录，不再更新**。

## 原始 Context（历史）

KeyKeeper 是一个 **macOS 菜单栏应用**，用于聚合多个 AI 平台（DeepSeek、智谱AI、Qoder、火山方舟）的 API 配额/余额查询。技术栈为 **Tauri v2 + Rust + Vue3 + TypeScript + TailwindCSS**。

## 完成状态

| Sprint | 状态 | 说明 |
|:---|:---|:---|
| Sprint 1: 项目脚手架 + 数据模型 | ✅ 完成 | 完整项目结构 + models.rs |
| Sprint 2: 安全存储 + DeepSeek 适配器 | ✅ 完成 | keystore.rs + deepseek.rs |
| Sprint 3: 剩余适配器 + 调度器 | ✅ 完成 | zhipu/qoder/volcano + scheduler.rs |
| Sprint 4: Tauri 命令与状态管理 | ✅ 完成 | commands.rs + AppState + store 持久化 |
| Sprint 5: 前端 UI | ✅ 完成 | App.vue + 3 个组件 + TailwindCSS |
| Sprint 6: 低额度通知 + 自动刷新 | ✅ 完成 | check_low_balance + auto-refresh 事件 |
| Sprint 7: 打包 | ✅ 可运行 | `pnpm tauri build` 可生成 .dmg |

## 对抗性审查修复记录

### 第一轮审查发现的问题（已修复）

1. ✅ `targets` 配置错误 → 改为 `["dmg"]`
2. ✅ 脚手架策略 → 当前目录初始化
3. ✅ `skip_taskbar` macOS 无效 → 移除
4. ✅ `withGlobalTauri` 已废弃 → 移除
5. ✅ 窗口定位方案 → 改为 Spike 验证
6. ✅ Entitlements 不完整 → 补充
7. ✅ 缺少 Rust 版本锁定 → 新增 rust-toolchain.toml

### 第二轮审查发现的问题（已修复）

1. ✅ Sprint 1 验证标准矛盾 → 改为可量化标准
2. ✅ 窗口定位方案 2 复杂度 → 标注为"最后备选"
3. ✅ 毛玻璃效果非自动 → 明确说明
4. ✅ 低额度阈值单位 → 按单位区分
5. ✅ Semaphore(5) → 改为 4
6. ✅ 请求取消机制 → 添加 CancellationToken 说明
7. ✅ AppState 设计 → 补充建议结构
8. ✅ 通知权限 API → 标注需确认
9. ✅ 前端状态管理 → 建议使用 Pinia
10. ✅ 日志策略 → 添加 log + env_logger
11. ✅ keyring 测试 → trait 注入或标记 #[ignore]
12. ✅ Gatekeeper 绕过 → 添加 xattr -cr
13. ✅ 数据迁移 → 明确迁移代码位置
14. ✅ reqwest TLS → 添加 rustls-tls 备选
15. ✅ DevTools → 启用 devtools
16. ✅ macOS 版本兼容性 → Sprint 1 验证
17. ✅ Sprint 1 失败决策路径 → 定义降级方案

### 开发结果审查发现的问题（已修复）

1. ✅ AppState 未持久化 → 使用 tauri-plugin-store
2. ✅ 窗口定位错误 → 改进为顶部右侧定位
3. ✅ Qoder 计时逻辑 → 持久化首次启动时间
4. ✅ Volcano 适配器空壳 → 实现完整 HMAC-SHA256
5. ✅ macOS 配置不完整 → 添加 LSUIElement
6. ✅ reqwest::Client 重复创建 → 共享 Arc<Client>
7. ✅ 缺少自动刷新 → 5 分钟定时器
8. ✅ 缺少低额度通知 → check_low_balance 命令
9. ✅ 没有单元测试 → 添加 2 个单元测试
10. ✅ Semaphore(5) → 改为 4
11. ✅ 缺少右键菜单 → 添加含"退出"的菜单
12. ✅ 操作无用户反馈 → 添加成功/错误 toast
13. ✅ HTTPS 证书固定 → 启用 native-tls
14. ✅ 火山方舟签名 → 完整实现

## 当前项目结构

```
keykeeper/
├── src/                    # 前端 (Vue3 + TS)
│   ├── App.vue             # 主界面
│   ├── main.ts             # Vue 入口
│   ├── types.ts            # TypeScript 类型
│   ├── components/
│   │   ├── QuotaCard.vue   # 配额卡片
│   │   ├── AddProviderForm.vue
│   │   └── RefreshBar.vue
│   └── style.css           # Tailwind 入口
├── src-tauri/              # 后端 (Rust)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── entitlements.plist
│   ├── Info.plist          # LSUIElement 配置
│   ├── rust-toolchain.toml
│   └── src/
│       ├── main.rs         # 入口 + 托盘 + 菜单
│       ├── commands.rs     # Tauri 命令
│       ├── models.rs       # 数据结构
│       ├── keystore.rs     # Keychain 存储
│       ├── scheduler.rs    # 并发调度器
│       └── adapters/
│           ├── mod.rs
│           ├── deepseek.rs
│           ├── zhipu.rs
│           ├── qoder.rs
│           └── volcano.rs
├── doc/
│   ├── KeyKeeper.md        # 设计文档
│   └── development-plan.md # 本文件
├── CLAUDE.md               # AI 协作指南
├── README.md               # 项目说明
└── package.json
```

## 运行方式

```bash
# 开发模式
pnpm tauri dev

# 构建
pnpm tauri build

# 测试
cd src-tauri && cargo test
```
