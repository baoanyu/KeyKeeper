# KeyKeeper 待办需求 Backlog

> 生成时间：2026-08-07 · 最后更新：2026-09-23
> 来源：合并自原 `audit-and-roadmap.md`、`ux-improvements.md`、`KeyKeeper.md`、`development-plan.md` 中**未实施**的需求。
> 本文档只列未实施项 + 必要上下文，已实施项不迁移。

> ⚠️ **2026-09-23 注记**：`refactor-plan-v2.md` 已开始执行，本文档**不再是未实施项的权威清单**——多项条目被吸收 / 取代 / 作废（完整清单见 refactor-plan-v2.md §4 末尾）。排查待办请以 refactor-plan-v2.md §5 执行阶段为准，本文档保留作历史上下文与证据链。

> ⚠️ **2026-08-31 实施审计注记**：原 backlog 中多项声称"未实施"的条目实际已在之前提交中完成。本次逐项核对代码后已修正。已完成项标记 ✅ 并注明日期。
>
> **2026-08-31 对抗性审查修复**：全量代码审查后发现 15 个真实问题（排除 3 个误报），本次修复 10 个。详见文末"对抗性审查已修复"清单。

---

## ⚠️ 审计警示（上下文保留）

以下关键陈述已被 2026-07-23 审计推翻，**实施前必须重新验证**：

| 原陈述 | 审计后真实情况 | 现行状态（2026-09-23） |
|:---|:---|:---|
| 火山方舟用 HMAC-SHA256 签名 | 鉴权/端点/数据模型全错。真实是多维度用量 + 套餐信息，需用户 DevTools 抓包 | ✅ 已按官方签名 Demo 重写 V4 签名器 + `QueryBalanceAcct`；billing region 待真实 AK/SK 终验 |
| Qoder 无公开余额接口 | 认知可能有误，需 DevTools 验证 | ⬜ 未验证，Qoder 维持 `Manual` |
| DeepSeek 解析 `data.balance` | 实际是 `balance_infos[].total_balance`，当前代码始终返回 0.00 元 | ✅ 已按官方文档改为解析**顶层** `balance_infos[]` |
| `QuotaInfo` 单值 `total + remaining` 够用 | 无法表达多维度用量，需升级为 `Vec<QuotaMetric>` + `Subscription` | ✅ 已升级为 `Vec<Entitlement>`（**未采用**本表原本设想的 `QuotaMetric` / `Subscription` 三类型方案，见 refactor-plan-v2.md §2.2 及其 v4 修订） |

> **铁律**：任何标注 ⚠️ 的字段路径、端点 URL、签名细节，实施前必须先 curl 真实响应或查规范。
> 📌 **该铁律尚未完全满足**：三条自动查询的契约来自官方文档 / 实证实现 / 官方签名 Demo，**均未经真实 Key 终验**（终验清单见 [code-review-2026-09-remediation.md](./code-review-2026-09-remediation.md) §5）。

---

## 依赖关系图（关键路径）

```
┌─────────────────────────────────────────────────────────────┐
│  Sprint 0：用户手动抓 API（外部阻塞，无法绕开）               │
│  · 火山方舟控制台 DevTools 抓套餐/用量 API（P0-3）           │
│  · Qoder 用量页 DevTools 抓 API（P0-4）                      │
│  · DeepSeek /user/balance curl（P0-2）                       │
│  · 智谱 /api/paas/v4/balance curl                           │
│  · 所有 fixture 存到 src-tauri/tests/fixtures/               │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  P0-3 数据模型 v2（一次 breaking change，一次做完）          │
│  QuotaInfo → Vec<QuotaMetric> + Subscription + QuotaStatus   │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
        ┌──────────────────┼──────────────────┐
        ▼                  ▼                  ▼
   adapter 重写        前端 UX 重设计        文档更新
   (P0-2/P0-4/P2-7)   (U-3/U-4/U-5/         (KeyKeeper.md
                       U-11/U-13)            正文重写)
```

**不依赖 Sprint 0、可立即实施的项目**见第 8 节「低垂果实」。

---

## 1. 🔴 P0 严重（未实施）

### P0-2. DeepSeek 余额解析字段错误 — 始终返回 0.00 元

> ✅ **已修复**（2026-09-23，提交 `04f611b`）：改为按官方文档解析顶层 `balance_infos[]`，fixture 驱动的解析测试入库。**真实 Key 终验仍待做**。以下保留原始描述作证据链。

**位置**：`src-tauri/src/adapters/deepseek.rs`（原 35-40 行）

**现状**：代码读 `json["data"]["balance"]`，实际 DeepSeek `/user/balance` 返回 `balance_infos[].total_balance`。

**修复关键**：解析失败必须返回 `is_success: false`，不能沉默为 0。`unwrap_or(0.0)` 是静默 bug 的根源。

**实施前**：先用真实 Key `curl -H "Authorization: Bearer sk-xxx" https://api.deepseek.com/user/balance` 拿到响应，比对字段。

---

### P0-3. Volcano 认知与实现全面错误（重大修正）

> 🟡 **部分修复**（2026-09-23，提交 `04f611b`）：签名器已按官方 Demo 重写（`VOLC` 前缀 / region / URI / Action 四处错误已修），并实现 `QueryBalanceAcct` 账户余额接口；**billing region 待真实 AK/SK 终验**，方舟**套餐到期**预计无 API Key 可鉴权接口 → 维持 `Manual` 兜底（code-review §5 Q3/Q4）。
> ⚠️ 下文「修复行动 2」的 `QuotaMetric` / `Subscription` / `QuotaStatus` 三类型方案**最终未被采纳**——实际落地为 `Entitlement` 加 `used_percent` 一个字段（理由见 refactor-plan-v2.md §2.2 v2.1 说明）。以下保留原始描述作证据链。

> ⚠️ 初审最严重的认知错误。Volcano **有完整的官方套餐/用量查询 API**，且是用户工作流核心依赖。

**现有代码错误**：

| 层面 | 错误 |
|:---|:---|
| 鉴权 | 用 HMAC-SHA256 — 实际鉴权方式待抓取确认 |
| 端点 | `open.volcengineapi.com/api/v3/quota/balance` — 存在性未验证 |
| 数据模型 | 假设一个 `remaining: f64`，实际有 3+ 个并存维度 |
| 语义 | `PayAsYouGo` — 实际用户是包月套餐 |

#### 修复行动 1：抓真实 API（用户手动，在动任何代码前）

1. 打开火山方舟控制台"套餐信息"页面
2. DevTools → Network → 过滤 XHR/fetch
3. 记录到 `src-tauri/tests/fixtures/volcano_*.json`：完整 Request URL / Method / Headers / Body / Response Body

**关键判断**：
- 若鉴权用**同一个 API Key** → 走现有 Keychain 路径
- 若鉴权用**登录态 Cookie** → 无法接入，只能引导用户到控制台
- 若鉴权用**主账号 AK/SK** → 有安全风险，需用户明确同意

#### 修复行动 2：升级数据模型为多维度用量（breaking change）

```rust
pub struct QuotaMetric {
    pub label: String,               // "当前会话" / "近1周" / "近1月"
    pub used_percent: Option<f64>,
    pub used: Option<f64>,
    pub total: Option<f64>,
    pub remaining: Option<f64>,
    pub unit: QuotaUnit,
    pub refresh_at: Option<u64>,
}

pub struct Subscription {
    pub plan_name: String,
    pub billing_mode: String,
    pub status: String,
    pub started_at: u64,
    pub expires_at: u64,
    pub auto_renew: bool,
}

pub enum QuotaStatus { Ok, Estimated, Failed }

pub struct QuotaInfo {
    pub provider_name: String,
    pub status: QuotaStatus,
    pub metrics: Vec<QuotaMetric>,
    pub subscription: Option<Subscription>,
    pub message: Option<String>,
    pub console_url: Option<String>,
}
```

**一次做完的清单**：
- [ ] `src-tauri/src/models.rs`
- [ ] `src-tauri/src/adapters/{deepseek,zhipu,qoder,volcano}.rs`
- [ ] `src/types.ts`
- [ ] `src/components/QuotaCard.vue`（多进度条 + 套餐信息卡）
- [ ] `src-tauri/tests/fixtures/*.json`

> ⚠️ 在数据模型 v2 落地前，其他修复不要引用旧 `QuotaInfo` 字段，否则会返工。

#### 修复行动 3：实现 Volcano 适配器 v2

抓到真实 API 后按伪代码实现（并发拉套餐信息 + 用量，组装 v2 结构）。

#### 修复行动 4：告警门槛

- 月度用量 ≥ 80% → 黄色警告
- 月度用量 ≥ 95% → 红色严重
- 套餐剩余 ≤ 7 天 → 桔色提醒
- 自动续费关闭 且 剩余 ≤ 3 天 → 红色严重

---

### P0-4. Qoder 认知同样可能有误

**现状**：`src-tauri/src/adapters/qoder.rs:35` 基于"Qoder 无公开接口"假设做本地估算。

**必须先做的调查**：
- 有官方 API 且鉴权可复用 → 走数据模型 v2 路径，替代本地估算
- 只有登录态 Cookie → 保留本地估算，修正循环重置逻辑
- 确实无接口 → 至少修"归零永不重置"的 bug

**若确认必须继续本地估算**：
```rust
let cycle_elapsed = elapsed % CODING_PLAN_DURATION_SECS;
let remaining = CODING_PLAN_DURATION_SECS - cycle_elapsed;
```

---

### P0-6. 托盘图标不是 macOS Template Image

**位置**：`src-tauri/src/main.rs:49`

**现状**：使用 `app.default_window_icon()`（彩色应用图标）作为托盘图标。macOS 菜单栏正确做法是**单色描线 + 模板图片**，系统自动反色适配深浅模式。

**修复**：
1. 新增单色 PNG：`src-tauri/icons/tray-icon-Template@2x.png`（22×22 @ 2x = 44×44）
2. 命名后缀 `Template` 是关键
3. `main.rs::TrayIconBuilder::icon()` 换成加载新图标

**阻塞**：需设计师提供 Template PNG。这也是路线图 R-1（动态状态图标）的前置条件。

---

### P0-7. CLAUDE.md / README 声明了未接入的功能

**现状**：~~CLAUDE.md 和 README 写了"低额度自动通知提醒"，但 `check_low_balance` 命令从未被前端调用。~~

**修复**：
- ~~短期：CLAUDE.md 和 README 中标注为"计划中"或删除~~ ✅ 已修复：README 死链（指向不存在的 audit-and-roadmap.md）已更正为 requirements-backlog.md
- ~~长期：完成 P2-1~~ ✅ 已完成：见 P2-1

> **实施状态**：✅ 已完成（2026-08-31）

---

## 2. 🟠 P1 高优先级（未实施）

| # | 问题 | 位置 | 修复要点 |
|:---|:---|:---|:---|
| P1-8 | `check_low_balance` 未接入前端 | `main.rs:100,138-171` | 见 P0-7 短期 / P2-1 长期 |

> P1-1 ~ P1-7 已实施（启动重复刷新、providers 竞态、错误响应脱敏、移除 shell 权限、scheduler panic 兜底、删 onUnmounted 死代码）。

---

## 3. 🟡 P2 中优先级（未实施）

### P2-1. 接入低额度通知

~~`src/App.vue::refresh()` 完成后调用 `check_low_balance` 并触发系统通知。~~

✅ **已完成**（2026-08-31）：`App.vue::refresh()` 后调用 `checkAndNotify()`，内部 invoke `check_low_balance` 并 `sendNotification`。通知失败被 try/catch 隔离，不影响刷新。

原方案（已实施）：
```ts
import { sendNotification } from '@tauri-apps/plugin-notification';

const lowProviders = await invoke<string[]>('check_low_balance', { quotas: quotas.value });
if (lowProviders.length > 0) {
  await sendNotification({
    title: 'KeyKeeper 提醒',
    body: `以下平台余额不足：${lowProviders.join('、')}`,
  });
}
```

### P2-2. 低额度阈值前后端分歧

Rust 端（秒 `< 600`，token `< 1000`，CNY `< 10`）vs 前端（非 CNY 一律 `< 1000`，CNY `< 10`）。修复：抽到常量模块。

### P2-3. Qoder 时间持久化失败静默降级

`commands.rs:83` 的 `let _ = set_qoder_first_launch(...)` 忽略错误。若 store 写失败，每次刷新都重置为"当前时间"。修复：改为 `if let Err(e) = ... { log::error!(...); }`。

### P2-4. Mutex 持有跨 await + 类型选错

`save_provider_key` / `delete_provider` / `add_provider` 在持有锁的同时 await store 写入。修复：换 `parking_lot::Mutex` + clone 后释放锁再 await。

### P2-5. `save_provider_key` 静默覆盖已存在平台的 Key

用户重复"添加"同一平台会静默覆盖 Keychain 旧 Key，UI 却提示"已添加"。修复：Rust 端返回 `SaveResult::{Added, Updated}`，前端按结果给不同 toast。

### P2-7. 所有 adapter 弱类型解析 → strong-typed response struct

每个 adapter 定义 strong-typed response struct（而非 `serde_json::Value` + `.get().and_then()`）。P0-2（DeepSeek 字段错）就是这样爬进来的。

**收益**：加 adapter 测试时字段变了编译器立刻报错。

### P2-8. Provider 元数据硬编码分散在 5 处 → 集中化

新增一个平台要改 5 个文件。修复：抽 `PlatformSpec`，前端通过 `get_platform_specs` Tauri 命令读取。

```rust
pub struct PlatformSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub key_hint: &'static str,
    pub color: &'static str,
    pub console_url: &'static str,
    pub key_docs_url: &'static str,
    pub build: fn(Arc<Client>) -> Box<dyn QuotaFetcher>,
}
```

---

## 4. ⚪ P3 低优先级 / 清理（未实施）

| # | 问题 | 位置 | 修复 |
|:---|:---|:---|:---|
| P3-2 | `unwrap()` 可能 panic | `main.rs:41` | 改为 `if let Some(w) = ...` |
| P3-3 | 默认图标 `unwrap()` 可能 panic | `main.rs:49` | 同上 |
| P3-4 | `load_providers` 静默吞掉 store 错误 | `commands.rs:23-34` | 至少 `log::warn!` |
| P3-6 | `entitlements.plist` 授予不必要的 JIT 权限 | `entitlements.plist:8-11` | ✅ 已删除 `allow-jit` 和 `allow-unsigned-executable-memory`（2026-08-31） |
| P3-8 | Cargo 依赖版本约束松散 | `Cargo.toml` | ✅ 已 pin 到 minor（2026-08-31） |
| P3-9 | 确认 `Cargo.lock` 已提交 | `src-tauri/Cargo.lock` | `git ls-files` 验证 |
| P3-10 | `tauri.conf.json` 缺 `bundle.category` | `tauri.conf.json` | 加 `"category": "public.app-category.developer-tools"` |
| P3-11 | Qoder 适配器忽略 API Key，用户被迫填假 Key | `qoder.rs` + `AddProviderForm.vue` | Qoder 时隐藏 Key 输入框 |

> P3-1（未使用依赖）、P3-5（lang=en）、P3-7（.gitignore）已实施。

---

## 5. 📝 文档更新（未实施）

> ✅ **已由后续工作取代**（2026-09-23）：`KeyKeeper.md` 已不存在（本文件即其合并产物），面向读者的正文由 `README.md` 承接并在提交 `3a963ec` / `04f611b` 重写；"数据结构 v2 多维度" 由 refactor-plan-v2.md §2.2 定案；火山签名的错误前提已按官方 Demo 修正。以下保留原始描述作证据链。

### KeyKeeper.md 正文重写

文档顶部已加审计警示，但正文仍是错误版本。**依赖 Sprint 0 抓包完成**后有权威事实可写。

需修正点：
- §"各平台实现"表格（火山/Qoder/DeepSeek 全错）
- §"数据结构"（单值 → v2 多维度）
- §"避坑指南"（火山签名整个前提被推翻）
- § Cargo.toml 依赖（清理已移除的 `hmac/sha2/base64/thiserror`）

---

## 6. 🌟 功能路线图（全部未实施）

### 6.1 强推（P0/P1 修完后立刻做）

| # | 项 | 依赖 | 工作量 |
|:---|:---|:---|:---|
| ~~R-1~~ | ~~菜单栏图标做状态指示（动态切换托盘图标：正常/余额低/请求失败）~~ **已作废**：依赖 P0-6，而菜单栏形态已改为独立窗口（refactor-plan-v2.md §1.1 / §7），托盘不复存在；状态指示改由应用内标记承担（§2.7） | ~~P0-6~~ | — |
| R-2 | 缓存上次成功结果 + "上次更新时间" | — | 半天 |
| R-3 | 平台扩展（Kimi / SiliconFlow / OpenRouter / DashScope） | P2-8 后改动集中到 1 文件 | 2h/平台 |

### 6.2 值得做（中等成本）

| # | 项 | 依赖 | 工作量 |
|:---|:---|:---|:---|
| R-4 | 单张卡片手动刷新 | — | 2h |
| R-5 | 消耗速率与预测（SQLite 存快照，7d 均值预测） | — | 半天 |
| R-6 | i18n（至少加英文） | — | 半天 |
| R-7 | 全局快捷键 toggle 窗口 | — | 2h |

### 6.3 工程质量（长期回报）

| # | 项 | 依赖 | 工作量 |
|:---|:---|:---|:---|
| R-8 | Adapter 单元测试 + 响应快照 | P2-7 strong-typed | 1 天 |
| R-9 | `tracing` 替代 `env_logger` + 日志落盘 | — | 半天 |
| R-10 | GitHub Actions 自动发布 | — | 半天 |
| R-11 | Homebrew Cask 发布 | R-10 | 2h |
| R-12 | macOSPrivateApi 决策（保留 or 替换为公开 API） | — | 文档决策 |

### 6.4 更多补充

| # | 项 | 备注 |
|:---|:---|:---|
| R-13 | 暗色模式 | Tailwind `dark:` 变体成本极低 |
| R-14 | 键盘导航 + aria-label | A11y 基本要求 |
| R-15 | 最小刷新间隔保护 | 前端 debounce + Rust 记录上次时间 |
| R-16 | Add Provider 后 debounce 100ms 再 refresh | 批量添加不浪费请求 |
| R-17 | Add Provider 表单根据平台显示 Key 格式提示 | placeholder |
| R-18 | 通知点击行为 | 打开对应平台充值页 |
| R-19 | 冷启动 stale-while-revalidate | 配合 R-2 |
| R-20 | `PlanType::CodingPlan` 命名过窄 | 未来加"包月订阅"会重名，改 `TimedSession` |
| R-21 | README 改造 | 顶部截图 + 价值主张 + 平台图标网格 + FAQ |
| R-22 | CONTRIBUTING + issue/PR 模板 | 开源仓库标配 |
| R-23 | `rust-toolchain.toml` 需 pin 到具体版本 | 可复现构建 |

---

## 7. 🎨 UX 改进（未实施）

> 已实施：U-15 Key 引导、U-16 Key 校验、U-17 隐私说明、U-20 401 恢复、U-22 Key 预检。U-21 单卡重试部分实施。

### 信息传达（依赖数据模型 v2）

| # | 项 | 依赖 |
|:---|:---|:---|
| U-1 | 状态化（红/黄/绿信号 + 状态条） | U-11 阈值 |
| U-3 | 多维度用量（一个 provider 多条 metric 堆叠） | **数据模型 v2** |
| U-4 | 刷新时机（显示 refresh_at 相对时间） | 数据模型 v2 |
| U-5 | 套餐信息（剩余天数、自动续费状态） | 数据模型 v2 |

### 信息传达（可独立做）

| # | 项 | 依赖 |
|:---|:---|:---|
| U-2 | 相对时间预测（"按 7 日均值可用 12 天"） | R-5 SQLite |
| U-6 | 信任标注（精确/缓存/估算 小图标） | QuotaStatus + R-2 |

### 决策路径

| # | 项 | 依赖 |
|:---|:---|:---|
| U-7 | 低余额时"立即充值"按钮 | **可用 `window.open` 解耦 shell，可立即做** |
| U-8 | 点击卡片标题打开控制台 | PlatformSpec 加 `console_url` |
| U-9 | 顶部"决策提示"栏 | 需单价数据 |
| U-10 | 右上角 sparkline 7 天趋势 | R-5 SQLite |

### 主动性

| # | 项 | 依赖 |
|:---|:---|:---|
| U-11 | 分级告警（80/95/99 + 套餐 ≤7天/≤3天） | 数据模型 v2 + P2-1 |
| U-12 | 用量突增告警（24h > 7d 日均 × 3） | R-5 SQLite |
| U-13 | 套餐到期告警 | 数据模型 v2 |
| U-14 | 预测告警（按速率预测触顶时间 < 3 天） | R-5 SQLite |

### 添加流程

| # | 项 | 依赖 |
|:---|:---|:---|
| U-18 | .env 批量导入 | — |
| U-19 | 首次使用 3 步引导 | 1.0 之前做 |

### 快捷操作

| # | 项 | 依赖 |
|:---|:---|:---|
| U-23 | 右键菜单（刷新/打开控制台/充值/复制/历史/隐藏/删除） | — |
| U-24 | 托盘 hover tooltip 显示最紧张平台 | — |
| U-25 | 键盘快捷键（⌘R / ⌘⇧K / Esc） | 部分依赖 R-7 |

---

## 8. 🍇 低垂果实（不依赖 Sprint 0，可立即实施）

以下项目**无外部阻塞**，可以马上动手：

| 优先级 | 项 | 状态 |
|:---|:---|:---|
| 🔴 | U-7 立即充值按钮 | ✅ 已完成（QuotaCard.vue:139-145） |
| 🔴 | P0-7 CLAUDE.md/README 标注 | ✅ 已完成（2026-08-31） |
| 🟡 | U-18 .env 批量导入 | ⬜ 未实施 |
| 🟡 | U-23 右键菜单 | ⬜ 未实施 |
| 🟡 | U-24 hover tooltip | ⬜ 未实施（main.rs 有 `_tray` 占位但未用） |
| 🟡 | P2-3 Qoder 时间持久化加 `log::error!` | ✅ 已完成（commands.rs:97-99） |
| 🟡 | P3-4 `load_providers` 加 `log::warn!` | ✅ 已完成（commands.rs:27-28, 37-38） |
| ⚪ | P3-2/P3-3 `unwrap()` → `if let` | ✅ 已完成（main.rs:41-44, 51-54） |
| ⚪ | P3-10 加 `bundle.category` | ✅ 已完成（tauri.conf.json:22） |
| ⚪ | P3-11 Qoder 时隐藏 Key 输入框 | ✅ 已完成（AddProviderForm.vue:75, 109） |

---

## 9. 实施顺序建议

### Phase 1：Sprint 0（用户手动，半天）
- 抓真实 API（火山/Qoder/DeepSeek/智谱）
- fixture 存 `src-tauri/tests/fixtures/`

### Phase 2：数据模型 v2 + Volcano 上线（1-2 天）
- P0-3 修复行动 2/3/4（定稿 v2 → Volcano 重写 → 告警门槛）
- P0-2 / P0-4 / P0-5（adapter 迁移）
- 前端 QuotaCard v2 重设计
- KeyKeeper.md 正文重写

### Phase 3：用户可见 bug 收尾（1 天）
- P0-6 托盘 Template Image
- P0-7 文档标注
- P2-1 低额度通知接入
- P2-2 阈值统一

### Phase 4：架构清理（1-2 天）
- P2-4 锁类型
- P2-7 strong-typed struct
- P2-8 PlatformSpec 集中化
- P3 清理项

### Phase 5：功能补齐（1 天）
- R-1 托盘状态图标
- R-2 缓存 + 时间戳
- U-11 分级告警

### Phase 6：生态扩展
- R-3 Kimi + SiliconFlow
- R-4 单卡刷新
- R-7 全局快捷键

### Phase 7：1.0 发布前
- R-8 adapter 测试
- R-9 日志落盘
- R-6 i18n（英文）
- R-13/R-14 暗色模式 + a11y
- R-21 README 改造
- R-10 GitHub Actions
- R-12 macOSPrivateApi 决策
- R-11 Homebrew Cask
- P3-6/P3-8/P3-9 发布配置收敛

---

## 10. ✅ 对抗性审查已修复（2026-08-31）

全量代码审查发现 18 个问题，元审查排除 3 个误报，确认 15 个真实问题。本次修复 10 个：

| # | 严重度 | 问题 | 修复位置 |
|:---|:---|:---|:---|
| 1 | 🔴 | `save_providers` 错误静默吞掉 → 数据丢失 | `commands.rs` — 3 处 `let _ =` 改为错误传播 |
| 2 | 🔴 | `retryProvider` 不触发通知 → 行为不一致 | `App.vue` — 添加 `checkAndNotify` 调用 |
| 3 | 🔴 | Qoder 写入空 Key 到 Keychain | `App.vue` — 新增 `addProviderWithoutKey`；`keystore.rs` — 新增 `has_key` |
| 4 | 🟠 | `check_low_balance` 重复 `unwrap_or` | `main.rs` — 改用 `matches!` 模式匹配 |
| 5 | 🟠 | 刷新无防抖 → 并发请求风暴 | `App.vue` — `loading.value` 守卫 |
| 6 | 🟠 | `tauri-build` 未 pin 到 minor | `Cargo.toml` — `version = "2.0"` |
| 7 | 🟠 | `add_provider` 可添加无 Key 的 provider | `commands.rs` — 加 `has_key` 前置校验 |
| 9 | 🟡 | `retryProvider` 参数无意义 | `App.vue` — 添加注释说明 |
| 10 | 🟡 | 无 `rust-toolchain.toml` | 新增 `rust-toolchain.toml` (channel 1.97) |
| — | 🟡 | 删除死代码 `addProvider` | `App.vue` — 因 #3 变为未使用，移除 |

**不修复项（含理由）**：
- `window.open` 在 Tauri v2 无效 → 误报，实际可用
- `sem.acquire().unwrap()` panic → 被 `join_all` 兜底
- `ensure_providers_loaded` TOCTOU → 幂等无危害
- Volcano `.expect()` → Sprint 0 后重写
- `check_low_balance` 弱类型 → 已在 P2-7 规划
- mutex 跨 await → 留待架构清理阶段
- scheduler `unwrap()` → 被 join_all 兜底
- Info.plist 缺字段 → 上架时再补