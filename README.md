# KeyKeeper

macOS 独立窗口应用：AI 平台**额度与订阅管理台**。一眼看清各平台「还剩几天到期」+ 余额，替代手写的到期记录。

> ⚠️ **当前状态**：改造进行中。执行计划与决策记录见 [doc/refactor-plan-v2.md](doc/refactor-plan-v2.md)。

## 功能

- **到期管理**：手动录入各平台额度包的到期日，按自然日倒计时
- **临期标记**：到期 ≤3 天红色置顶、≤7 天橙色、低额度橙色（应用内标记，无系统通知）
- **多额度包**：同一平台可挂多个独立到期日（如「超算 DeepSeek」的 `0.1` 与 `10M 体验`）
- **API 自动查询**：有公开 API 的平台自动拉取余额（DeepSeek / 智谱 / 火山方舟）
- **手动录入**：无 API 的平台手动维护到期日（超算 DeepSeek / MiMo / 豆包工作 / LongCat / Qoder）
- **快捷续期**：「+30 天」「+7 天」一键顺延到期日
- **安全存储**：API Key 存 macOS Keychain，永不写入普通文件或 store

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

# 测试 / Lint（在 src-tauri/ 下）
cargo test
cargo clippy --all-targets -- -D warnings
```

## 项目结构

```
src/                  # 前端 (Vue3 + TS)
  components/         # 卡片 / 录入表单 / 刷新栏
  utils.ts            # 日期语义（本地时区 23:59:59）+ 到期标记规则
src-tauri/            # 后端 (Rust)
  src/
    main.rs           # 入口
    commands.rs       # Tauri 命令
    models.rs         # 数据模型 + PLATFORM_SPECS（平台元数据唯一来源）
    keystore.rs       # Keychain 存储
    scheduler.rs      # 并发调度器
    adapters/         # Api 平台适配器
doc/                  # 设计与审计文档（见下）
```

## 文档

- [doc/refactor-plan-v2.md](doc/refactor-plan-v2.md) — 改造计划、数据模型 v2、决策记录（唯一事实来源）
- [doc/requirements-backlog.md](doc/requirements-backlog.md) — 待办需求、Bug 修复计划、UX 改进

## 许可证

MIT
