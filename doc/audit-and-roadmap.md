# KeyKeeper 审计与规划文档

> 生成时间：2026-07-23（最后更新：2026-07-27，含对抗性审查修正）
> 相关文档：[KeyKeeper.md](KeyKeeper.md)（设计）· [development-plan.md](development-plan.md)（历史）· [ux-improvements.md](ux-improvements.md)（易用性改进）· [../CLAUDE.md](../CLAUDE.md)
>
> 本文档汇总了 KeyKeeper 项目的**报错与 Bug 相关审计结果**、修复计划、功能路线图和方法论反思。
> 由三份原始文档（`fix-plan.md`、`roadmap.md`、`adversarial-review.md`）合并整理而成。
> 易用性 / UX 改进部分已独立到 `ux-improvements.md`。

> **对抗性审查结果（2026-07-27）**
>
> 本次审查把文档每个"事实断言"都对照当前代码重新验证，结论分三类：
>
> - ✅ **代码已修复**：文档描述的 bug 在当前代码里已不存在（文档过时）
> - ✅ **已实施修复**：本轮按文档建议完成了代码修复
> - ⛔ **依赖外部信息**：修复需要真实 API Key / 用户抓包 / 设计资源，无法在没凭据的情况下实施
>
> | 条目 | 状态 | 说明 |
> |:---|:---|:---|
> | P0-1 PlanType 序列化 | ✅ 代码已修复 | 当前已是 `snake_case`，文档断言错误 |
> | P0-2 DeepSeek 余额解析 | ⛔ 待 curl 验证 | 代码确实读错字段，但真实 API 结构需用户抓包确认 |
> | P0-3 Volcano 全面错误 | ⛔ 待用户抓 API | 代码确实错误，但正确实现依赖真实 API 端点 |
> | P0-4 Qoder 认知可能有误 | ⛔ 待用户抓 API | 同上 |
> | P0-5 并入 P0-3 | ⛔ 同上 | 数据模型 v2 是 breaking change，需 P0-3 落地后一起做 |
> | P0-6 托盘 Template Image | ⛔ 需设计资源 | 需要设计师提供 Template PNG |
> | P0-7 CLAUDE.md 虚假功能 | 📝 待修文档 | `check_low_balance` 已注册但前端未调用 |
> | P0-8 setTimeout 泄露 | ✅ 已实施修复 | 见 `src/App.vue` |
> | P1-1 启动重复刷新 | ✅ 已实施修复 | 见 `src-tauri/src/main.rs` |
> | P1-2 providers 竞态 | ✅ 已实施修复 | 见 `src-tauri/src/commands.rs` |
> | P1-3 错误响应泄露 Token | ✅ 已实施修复 | 见 `src-tauri/src/adapters/` |
> | P1-4 多余 shell 权限 | ✅ 已实施修复 | 已移除 shell 插件 |
> | P1-5 npm 而非 pnpm | ✅ 代码已修复 | 当前已是 pnpm，文档断言错误 |
> | P1-6 scheduler 吞 panic | ✅ 已实施修复 | 见 `src-tauri/src/scheduler.rs` |
> | P1-7 onUnmounted 死代码 | ✅ 已实施修复 | 见 `src/App.vue` |
> | P2-1 低额度通知 | 📝 待实施 | 消费 P0-3 告警门槛 |
> | P2-2 阈值分歧 | 📝 待实施 | |
> | P2-3 Qoder 时间持久化 | 📝 待实施 | |
> | P2-4 锁类型 + 跨 await | 📝 待实施 | 需引入 parking_lot |
> | P2-5 静默覆盖 Key | 📝 待实施 | |
> | P2-6 并入 P0-3 | ⛔ 同上 | |
> | P2-7 strong-typed | 📝 待实施 | 建议等 P0-3 数据模型 v2 一起做 |
> | P2-8 PlatformSpec | 📝 待实施 | |
> | P3-1 未使用依赖 | ✅ 已实施修复 | 已移除 thiserror、base64 |
> | P3-2 unwrap | 📝 待实施 | |
> | P3-3 默认图标 unwrap | 📝 待实施 | |
> | P3-4 load_providers 吞错 | 📝 待实施 | |
> | P3-5 lang=en | ✅ 代码已修复 | 当前已是 zh-CN，文档断言错误 |
> | P3-6 entitlements | 📝 待实施 | |
> | P3-7 .gitignore | ✅ 已实施修复 | 已移除 /doc/ 排除，但文档断言本身有误（原排除整个 /doc/ 而非仅 CLAUDE.md） |
> | P3-8 Cargo 版本约束 | 📝 待实施 | |
> | P3-9 Cargo.lock | 📝 待实施 | |
> | P3-10 bundle.category | 📝 待实施 | |
> | P3-11 Qoder 无 Key | 📝 待实施 | |
>
> **本轮未实施项的共同卡点**：P0-3 数据模型 v2 是 P0-2/P0-4/P0-5/P2-6/P2-7 的前置依赖，而 P0-3 的正确实现需要用户在真实环境抓 API（Sprint 0 前置调查）。建议用户完成 Sprint 0 后再启动 Sprint A。


---

## 目录

1. [证据说明与验证要求](#0-证据说明与验证要求)
2. [问题总览](#1-问题总览)
3. [严重问题（P0）](#2-严重问题-p0)
4. [高优先级（P1）](#3-高优先级-p1)
5. [中优先级（P2）](#4-中优先级-p2)
6. [低优先级 / 清理（P3）](#5-低优先级--清理-p3)
7. [功能路线图](#6-功能路线图)
8. [明确不做的事](#7-明确不做的事)
9. [实施顺序建议](#8-实施顺序建议)
10. [方法论反思](#9-方法论反思)

---

## 0. 证据说明与验证要求

本文档标注了每个"事实断言"的证据来源：

- ✅ **已验证**：读过项目源码 / 已 curl 过真实 API
- ⚠️ **推理未验证**：基于经验推断，实施前必须先 curl 官方 API 或查规范原文
- 📖 **待查规范**：需要查阅官方签名规范或文档

**任何标注 ⚠️ 的字段路径、端点 URL、签名细节，实施前必须先 curl 真实响应或查规范**。此前的审计里就有几处推理错误（详见第 9 节反思）。

---

## 1. 问题总览

| 分级 | 数量 | 说明 |
|:---|:---|:---|
| 🔴 P0 严重 | 8 项（P0-3 已重大重写，P0-5/P2-6 已并入 P0-3） | 数据模型 v2、Volcano 真实 API、多维度用量 |
| 🟠 P1 高 | 8 项 | 请求浪费、竞态、日志泄露、UX 隐蔽 bug |
| 🟡 P2 中 | 8 项 | 功能缺失、锁选型、UX 死角 |
| ⚪ P3 低 | 11 项 | 依赖清理、`unwrap`、发布配置 |
| 🌟 路线图 | 23 项 | 分五个 Phase |

**易用性 / UX 改进 25 项已独立到** [ux-improvements.md](ux-improvements.md)。

---

## 2. 严重问题（P0）

### P0-1. `PlanType` 序列化字符串错误

**现象**：前端 DeepSeek/ZhipuAI/Volcano 卡片显示 `payasyougo`，Qoder 显示 `codingplan`，标签映射失效。

**根因**：`src-tauri/src/models.rs:4` ✅

```rust
#[serde(rename_all = "lowercase")]  // 把 PayAsYouGo → "payasyougo"
pub enum PlanType {
    PayAsYouGo,
    CodingPlan,
    Subscription,
}
```

前端 `src/types.ts:1` 和 `src/components/QuotaCard.vue:23-27` 期望 `"pay_as_you_go"` / `"coding_plan"` / `"subscription"`。

**修复**：

```rust
// src-tauri/src/models.rs:4
#[serde(rename_all = "snake_case")]
```

**验证**：`pnpm tauri dev`，查看 QuotaCard 徽章显示"按量付费"、"编程套餐"、"订阅"。

---

### P0-2. DeepSeek 余额解析字段错误 —— 始终返回 0.00 元

**现象**：DeepSeek 卡片始终显示"0.00 元"且带成功标识。

**根因**：`src-tauri/src/adapters/deepseek.rs:35-40` ✅ 读取 `json["data"]["balance"]`，但 DeepSeek `/user/balance` 实际返回结构 ⚠️：

```json
{
  "is_available": true,
  "balance_infos": [
    { "currency": "CNY", "total_balance": "10.50", "granted_balance": "0.00", "topped_up_balance": "10.50" }
  ]
}
```

**修复**（关键：解析失败必须返回 `is_success: false`，不能沉默为 0）：

```rust
let balance_infos = json
    .get("balance_infos")
    .and_then(|v| v.as_array())
    .ok_or_else(|| anyhow::anyhow!("DeepSeek 响应缺少 balance_infos"))?;

let first = balance_infos.first()
    .ok_or_else(|| anyhow::anyhow!("DeepSeek balance_infos 为空"))?;

let balance = first
    .get("total_balance")
    .and_then(|v| v.as_str())
    .and_then(|s| s.parse::<f64>().ok())
    .ok_or_else(|| anyhow::anyhow!("total_balance 解析失败"))?;
```

**关键教训**：`unwrap_or(0.0)` 是 P0-2 类静默 bug 的根源，本项目所有 adapter 目前都是"解析失败→显示 0"模式，需系统性修（见 P2-7 strong-typed）。

**实施前**：先用真实 Key `curl -H "Authorization: Bearer sk-xxx" https://api.deepseek.com/user/balance` 拿到响应，比对字段。

---

### P0-3. Volcano 认知与实现全面错误（重大修正）

> ⚠️ **本条目是初审最严重的认知错误**：初审假设"Volcano 无余额 API 或需 HMAC 签名"，用户提供的火山方舟控制台截图**直接推翻了这个假设**。Volcano **有完整的官方套餐/用量查询 API**，且是**用户当前工作流的核心依赖**（`ark-code-latest` 模型即由此账户支撑），此 provider 的可靠性是 KeyKeeper 的最高优先级。

**用户提供的火山方舟控制台事实（screenshot 观测）** ✅：

- **套餐类型**：Lite 套餐（还有 Pro/其他套餐）
- **订阅状态**：生效中 / 已过期 / 已取消 等
- **有效期**：开始时间 + 结束时间 + 剩余天数（"33 天"）
- **计费模式**：包月（其他可能"包年"、"按量"）
- **自动续费**：开关状态
- **用量维度是多个并存**：
  - 当前会话：7%（`04时25分钟后刷新` — 类似 5 小时滚动窗口）
  - 近 1 周：8%（`3天06时37分钟后刷新` — 滚动窗口）
  - 近 1 月：99%（`1天06时37分钟后刷新` — 30 天滚动窗口）⚠️ **即将触顶**
- **续费入口**：控制台有"续费套餐 / 升级套餐"两个按钮

**现有代码的错误**（`src-tauri/src/adapters/volcano.rs`）✅：

| 层面 | 错误 |
|:---|:---|
| 鉴权 | 用 HMAC-SHA256（`AccessKey:SecretKey`）— 火山方舟推理服务用 Bearer Token，简单得多 📖 |
| 端点 | `open.volcengineapi.com/api/v3/quota/balance` — 端点存在性未验证，且不是查套餐用量的正确入口 📖 |
| 数据模型 | 假设一个 `remaining: f64`，实际有 3+ 个并存维度 |
| 语义 | `PayAsYouGo` — 实际用户是 `CodingPlan`（包月套餐） |
| 覆盖 | 只查"余额"，不查套餐信息、有效期、自动续费状态 |

**用户最紧迫的场景**：Lite 套餐 33 天剩余、月度用量 99% 即将触顶。**这正是 KeyKeeper 应该提前告警但完全没做的**——初审的 Bearer/HMAC 讨论完全绕开了真正的问题。

---

#### 修复行动 1：抓真实 API（在动任何代码前）

**必须先做的调查**：

1. 打开火山方舟控制台的"套餐信息"页面（用户提供截图那个页）
2. 打开浏览器 DevTools → Network 标签 → 过滤 XHR/fetch
3. 刷新页面，找到套餐信息和用量统计对应的请求
4. 记录以下内容到 `src-tauri/tests/fixtures/volcano_*.json`：
   - Request URL 完整路径
   - Request Method（GET/POST）
   - **所有 Request Headers**（重点：`Authorization` / `Cookie` / `X-*` 自定义头）
   - Request Body（若 POST）
   - **完整 Response Body**（每个用量维度的字段名、单位、时间戳字段格式）

**关键判断**：

- 若鉴权用**同一个 API Key**（能在火山方舟"API Key 管理"页拿到）→ 走 KeyKeeper 现有的 Keychain 存储路径，走通
- 若鉴权用**登录态 Cookie / 短期 Token** → 无法接入 KeyKeeper（需要用户走浏览器登录流程，超出范围），只能引导用户到控制台
- 若鉴权用**主账号 AK/SK**（火山账户根凭据）→ 有安全风险，需要用户明确同意

---

#### 修复行动 2：升级数据模型为多维度用量

现有 `QuotaInfo` 结构（一个 `total: f64` + 一个 `remaining: f64`）**根本无法表达火山方舟的真实用量**。原 P0-5 建议的 `total: Option<f64>` 也不够。真实模型：

```rust
// src-tauri/src/models.rs
pub struct QuotaMetric {
    pub label: String,               // "当前会话" / "近1周" / "近1月"
    pub used_percent: Option<f64>,   // 0..100，来自 API 直接给的百分比
    pub used: Option<f64>,           // 已使用绝对量（可选）
    pub total: Option<f64>,          // 上限（可选，包月套餐可能不给具体数字）
    pub remaining: Option<f64>,      // 剩余（可选）
    pub unit: QuotaUnit,
    pub refresh_at: Option<u64>,     // Unix timestamp: 下次刷新时间
}

pub struct Subscription {
    pub plan_name: String,           // "Lite 套餐" / "Pro 套餐"
    pub billing_mode: String,        // "包月" / "包年" / "按量"
    pub status: String,              // "生效中" / "已过期"
    pub started_at: u64,             // 开始时间
    pub expires_at: u64,             // 结束时间
    pub auto_renew: bool,            // 自动续费
}

pub struct QuotaInfo {
    pub provider_name: String,
    pub status: QuotaStatus,         // Ok / Estimated / Failed（见 P2-6）
    pub metrics: Vec<QuotaMetric>,   // ⚠️ 数组 — 一个 provider 可有 N 个维度
    pub subscription: Option<Subscription>,
    pub message: Option<String>,
    pub console_url: Option<String>, // 充值/管理入口，用于前端"打开控制台"按钮
}
```

**这是一次 breaking change，一次做完所有 adapter**（不要分批），否则前端 UI 会来回改。清单：

- [ ] `src-tauri/src/models.rs`
- [ ] `src-tauri/src/adapters/{deepseek,zhipu,qoder,volcano}.rs`
- [ ] `src/types.ts`
- [ ] `src/components/QuotaCard.vue`（重设计：多进度条 + 套餐信息卡）
- [ ] `src-tauri/tests/fixtures/*.json`

**该 breaking change 与原 P0-5、P2-6 合并为一次**——它们指向同一问题。

---

#### 修复行动 3：实现 Volcano 适配器 v2

抓到真实 API 后按以下伪代码实现：

```rust
// src-tauri/src/adapters/volcano.rs（v2 重写）

#[derive(Deserialize)]
struct VolcanoPlanResponse {
    plan_name: String,          // ⚠️ 真实字段名待抓取确认
    plan_type: String,          // "lite" / "pro"
    started_at: String,         // ISO 时间戳
    expires_at: String,
    billing_mode: String,
    auto_renew: bool,
    status: String,
}

#[derive(Deserialize)]
struct VolcanoUsageResponse {
    metrics: Vec<VolcanoMetric>, // ⚠️ 真实字段名待抓取确认
}

#[derive(Deserialize)]
struct VolcanoMetric {
    label: String,               // "当前会话" / "近1周" / "近1月"
    used_percent: f64,
    refresh_at: Option<String>,  // ISO 时间戳
}

impl VolcanoFetcher {
    async fn fetch_quota(&self, api_key: &str) -> Result<QuotaInfo> {
        // 1. 并发拉套餐信息 + 用量
        let (plan, usage) = tokio::try_join!(
            self.fetch_plan(api_key),
            self.fetch_usage(api_key),
        )?;

        // 2. 组装
        Ok(QuotaInfo {
            provider_name: "Volcano".into(),
            status: QuotaStatus::Ok,
            metrics: usage.metrics.into_iter().map(convert_metric).collect(),
            subscription: Some(convert_subscription(plan)),
            message: None,
            console_url: Some("https://console.volcengine.com/ark/region:ark+cn-beijing/subscription".into()),
        })
    }
}
```

---

#### 修复行动 4：告警门槛

用户当前场景（月度 99%、剩余 33 天套餐）**就是应该触发多种告警的场景**：

- 月度用量 ≥ 80% → 黄色警告
- 月度用量 ≥ 95% → 红色严重
- 套餐剩余 ≤ 7 天 → 桔色提醒
- 自动续费关闭 且 剩余 ≤ 3 天 → 红色严重
- （所有告警的实际触发在 P2-1 低额度通知里做，此处只声明门槛）

告警门槛在数据模型 v2 落地后立即可用。

---

**优先级说明**：因为 Volcano 是用户当前工作流的核心（`ark-code-latest`），**此 P0-3 应作为 Sprint A 中最优先项**，甚至比 P0-1 更紧迫——但推进有依赖顺序：

```
1. 抓真实 API（用户手动做，1 小时）
      ↓
2. 数据模型 v2 定稿（P0-3 + P0-5 + P2-6 一次到位，半天）
      ↓
3. Volcano 适配器重写 + 前端 QuotaCard 重设计（1 天）
      ↓
4. 其他 adapter 迁移到 v2 模型（半天）
```

**在数据模型 v2 落地前，其他 P0/P1 修复不要引用 `QuotaInfo` 的旧字段结构**，否则会返工。

---

### P0-4. Qoder 认知同样可能有误 — 需要重新调查

**现有代码**：`src-tauri/src/adapters/qoder.rs:35` ✅

```rust
let remaining = (CODING_PLAN_DURATION_SECS - elapsed).max(0.0);
```

现有实现基于 CLAUDE.md 的说法 *"Qoder 无公开余额接口，本地估算 5 小时"*。**但受 P0-3 Volcano 认知修正的启发，这个假设需要重新验证**——CLAUDE.md 里的"无公开接口"很可能是初期实现者没找到而非事实。

**必须先做的调查**：

1. 打开 Qoder 用量/套餐页
2. DevTools → Network，捕获真实请求
3. 判断：
   - **有官方 API 且鉴权可复用** → 走 P0-3 同样的数据模型 v2 路径，替代本地估算
   - **只有登录态 Cookie** → 保留本地估算，但至少修正循环重置逻辑
   - **确实无接口** → 至少修正现在"归零永不重置"的 bug

**若确认必须继续本地估算**，则原修复仍然适用：

```rust
let cycle_elapsed = elapsed % CODING_PLAN_DURATION_SECS;
let remaining = CODING_PLAN_DURATION_SECS - cycle_elapsed;
```

同时前端 QuotaCard 应显示"距下次重置还有 X 小时"而不是"剩余 X 秒"。

**但更可能的正确结果**：Qoder 有官方 API，本地估算方案整个删除，改用数据模型 v2 的 `Vec<QuotaMetric>` 表示。

---

### P0-5. `QuotaInfo` 数据模型无法表达真实用量 → 并入 P0-3

**已并入 P0-3 修复行动 2**。原本建议 `total: Option<f64>` 是**不够的**——Volcano/Qoder 都是"多维度用量并存"，一个 `total + remaining` 无论怎么改都表达不出来。

正确解法：`QuotaInfo::metrics: Vec<QuotaMetric>` + 独立的 `subscription` 字段。详见 P0-3 修复行动 2 的数据模型定义。

**智谱进度条永远 0% 的问题**（`zhipu.rs:45` `total: remaining` ✅）在 v2 模型中自然消失：如果 API 不返回 total，`QuotaMetric.total = None`，前端就不渲染进度条而只显示"剩余 X"。

---

### P0-6. 托盘图标不是 macOS Template Image

**现象**：`src-tauri/src/main.rs:49` ✅ 使用 `app.default_window_icon()`（彩色应用图标）作为托盘图标。macOS 菜单栏的正确做法是**单色描线 + 模板图片**，系统自动反色适配深浅模式。

**表现**：
- 深色菜单栏：彩色图标看起来"脏"
- 视网膜屏：像素化
- 与其他菜单栏应用视觉不一致

**修复**：
1. 新增单色 PNG：`src-tauri/icons/tray-icon-Template@2x.png`（尺寸 22×22 @ 2x = 44×44）
2. 命名后缀 `Template` 是关键——macOS 依此识别为模板图片
3. `main.rs::TrayIconBuilder::icon()` 换成加载这个新图标

**这是 macOS 菜单栏应用的入门坑，也是路线图 R-1（动态状态图标）的前置条件**。

---

### P0-7. CLAUDE.md / README 声明了未接入的功能

**现象**：CLAUDE.md 和 README 都写了"低额度自动通知提醒"，但实际 `check_low_balance` 命令**从未被前端调用**（`src/` 中无 `invoke('check_low_balance', ...)`）✅。

**影响**：用户读 CLAUDE.md 会认为该功能可用，然后疑惑"为什么没通知"——比"没有该功能"更伤信任。

**修复**：
- 短期：把 CLAUDE.md 和 README 中的"低额度通知"标注为**"计划中"**或删除
- 长期：完成 P2-1（真正接入通知）

---

### P0-8. `showError`/`showSuccess` setTimeout 泄露

**现象**：`src/App.vue:18-26` ✅

```ts
function showSuccess(msg: string) {
  success.value = msg;
  setTimeout(() => { success.value = ''; }, 3000);
}
```

**Bug**：连续两次调用 `showSuccess`，第一次的 timer 会在 3s 后触发，把第二次的消息也清掉。用户看到消息突然消失。

**修复**：

```ts
let successTimer: number | null = null;
function showSuccess(msg: string) {
  success.value = msg;
  if (successTimer) clearTimeout(successTimer);
  successTimer = window.setTimeout(() => { success.value = ''; }, 3000);
}
```

`showError` 同理。

---

## 3. 高优先级（P1）

### P1-1. 启动时重复刷新

**位置**：`src-tauri/src/main.rs:83-89` ✅

`tokio::time::interval` 第一次 `tick()` 立即触发，而 `App.vue::onMounted` 已经调用了一次 `refresh()`，导致启动时连续两次拉取所有平台配额。

**修复**：

```rust
let mut interval = tokio::time::interval(Duration::from_secs(AUTO_REFRESH_INTERVAL_SECS));
interval.tick().await; // 消耗第一次立即触发
loop {
    interval.tick().await;
    let _ = app_handle.emit("auto-refresh", ());
}
```

---

### P1-2. Provider 列表加载竞态

**位置**：`src-tauri/src/commands.rs:61-72, 154-165` ✅

`get_all_quotas` 和 `get_saved_providers` 用"加锁 → 检查空 → drop → load → 再加锁 → 写入"模式，两次并发刷新可能相互覆盖。

**修复**：抽出一个持锁完成整个流程的辅助函数

```rust
async fn ensure_providers_loaded(state: &AppState, app: &tauri::AppHandle) -> Vec<String> {
    let mut guard = state.providers.lock().await;
    if guard.is_empty() {
        *guard = load_providers(app).await;
    }
    guard.clone()
}
```

---

### P1-3. 错误响应体可能泄露 Bearer Token

**位置**：`src-tauri/src/adapters/{deepseek,zhipu,volcano}.rs` ✅

非 2xx 响应时 `resp.text().await.unwrap_or_default()` 完整拼进 `QuotaInfo::error_msg` 返回前端。如果平台在错误响应中回显 Request 头（部分 API 网关会），可能显示 Authorization 头部分内容。

**修复**（无依赖版本，不引入 regex）：

```rust
fn sanitize_error(body: &str) -> String {
    let truncated: String = body.chars().take(200).collect();
    truncated.split_whitespace()
        .map(|w| {
            if w.starts_with("sk-") || w.starts_with("Bearer") || w.contains("=sk-") {
                "***"
            } else { w }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
```

---

### P1-4. 移除不必要的 `shell:allow-open` 权限

**位置**：`src-tauri/capabilities/default.json:13`、`src-tauri/src/main.rs:35`、`package.json:18` ✅

`shell.open` 从未在前端使用（`grep` 无命中），但赋予 Web 层调用任意 URL/应用的能力，违背最小权限。

**修复**：
1. `capabilities/default.json`：删除 `"shell:allow-open"`
2. `main.rs`：删除 `.plugin(tauri_plugin_shell::init())`
3. `Cargo.toml`：删除 `tauri-plugin-shell` 依赖
4. `package.json`：删除 `@tauri-apps/plugin-shell`

---

### P1-5. `tauri.conf.json` 使用 npm 而非 pnpm

**位置**：`src-tauri/tauri.conf.json:7,9` ✅

项目用 pnpm（有 `pnpm-lock.yaml`、`.npmrc`），但 Tauri 配置写的 `npm run`：

```json
"beforeDevCommand": "pnpm dev",
"beforeBuildCommand": "pnpm build"
```

---

### P1-6. `scheduler.rs` 吞掉 tokio spawn panic

**位置**：`src-tauri/src/scheduler.rs:39` ✅

```rust
results.into_iter()
    .filter_map(|r| r.ok())  // ← spawn panic 后 JoinHandle 返回 Err，这里直接丢弃
    .collect()
```

**Bug**：如果某个 adapter panic，对应的卡片会**从 UI 里消失**——用户完全不知道为什么"添加了但看不到"。

**修复**：保留 provider 名单，panic 时返回错误 QuotaInfo

```rust
// 提前记录 provider 名到 Vec<String>
let provider_names: Vec<String> = tasks.iter().map(|(name, _, _)| name.clone()).collect();

// ...
results.into_iter()
    .enumerate()
    .map(|(i, r)| match r {
        Ok(quota) => quota,
        Err(join_err) => QuotaInfo::error(
            &provider_names[i],
            &format!("Internal panic: {}", join_err),
        ),
    })
    .collect()
```

---

### P1-7. Vue `onUnmounted` 死代码

**位置**：`src/App.vue:72-76` ✅

菜单栏应用主窗口是 hide 不是 destroy，Vue 组件从不真正 unmount，`onUnmounted` cleanup 永远不执行。

**修复**：删掉 `onUnmounted` 块，避免代码给"我已处理清理"的假象。

---

### P1-8. `check_low_balance` 未接入前端

**位置**：`src-tauri/src/main.rs:100, 138-171`（已注册但前端无调用）✅

**决策**：
- 短期：见 P0-7，文档标注为"计划中"
- 长期：见 P2-1，接入通知

---

## 4. 中优先级（P2）

### P2-1. 接入低额度通知（消除 P0-7 假声明）

**方案**：`src/App.vue::refresh()` 完成后调用 `check_low_balance` 并触发通知

```ts
import { sendNotification } from '@tauri-apps/plugin-notification';

async function refresh() {
  // ... 现有逻辑
  const lowProviders = await invoke<string[]>('check_low_balance', { quotas: quotas.value });
  if (lowProviders.length > 0) {
    await sendNotification({
      title: 'KeyKeeper 提醒',
      body: `以下平台余额不足：${lowProviders.join('、')}`,
    });
  }
}
```

同时统一阈值（见 P2-2）。

---

### P2-2. 低额度阈值前后端分歧

**位置**：Rust `main.rs:158-163`（秒 `< 600`，token `< 1000`，CNY `< 10`）vs 前端 `QuotaCard.vue:18-21`（非 CNY 一律 `< 1000`，CNY `< 10`）✅

**修复**：把阈值抽到常量模块，前端通过命令读取或复制常量。

---

### P2-3. Qoder 首次启动时间持久化失败静默降级

**位置**：`src-tauri/src/commands.rs:83` ✅

`let _ = set_qoder_first_launch(&app, now).await;` 忽略错误。若 store 写失败，每次刷新都会重置为"当前时间"，5 小时倒计时始终显示接近 5h。

**修复**：改为 `if let Err(e) = ... { log::error!(...); }`。

---

### P2-4. Mutex 持有跨 await + 类型选错

**问题一（跨 await）**：`src-tauri/src/commands.rs:127-131, 143-146, 175-179` ✅

`save_provider_key` / `delete_provider` / `add_provider` 在持有 `providers` 锁的同时 await store 写入，阻塞其他命令。

**问题二（锁类型过度）**：`commands.rs:16` ✅ 用 `tokio::sync::Mutex<Vec<String>>`，但里面全是同步操作，没有跨 await 持锁的正当理由。

**联合修复**：换 `parking_lot::Mutex` + clone 后释放锁再 await

```rust
pub providers: parking_lot::Mutex<Vec<String>>,

// 使用处：
let cloned = {
    let mut providers = state.providers.lock();  // 同步锁
    // 修改 providers
    providers.clone()
};
let _ = save_providers(&app, &cloned).await;  // 释放锁后 await
```

---

### P2-5. `save_provider_key` 静默覆盖已存在平台的 Key

**位置**：`src-tauri/src/commands.rs:118-133` ✅

用户重复"添加"同一平台会静默覆盖 Keychain 里旧 Key，UI 却提示"已添加"。

**修复**：Rust 端返回 `SaveResult::{Added, Updated}`，前端按结果给不同 toast。

---

### P2-6. `error_msg` 语义混乱 → 已并入 P0-3 数据模型 v2

**问题**：`qoder.rs:44` ✅ 明明 `is_success: true` 却填了 `error_msg: Some("本地估算...")`——`error_msg` 被当作"info/hint"用，语义分裂。

**修复**：**已并入 P0-3 修复行动 2**。P0-3 的数据模型 v2 中已包含：

```rust
pub enum QuotaStatus {
    Ok,          // 数字精确
    Estimated,   // 本地估算 or 只验了 Key
    Failed,      // 请求失败
}

pub struct QuotaInfo {
    pub status: QuotaStatus,
    pub message: Option<String>,  // 统一说明（任何状态都可填）
    // ...
}
```

一次性解决：`error_msg` 泄露、Qoder 语义混乱、Volcano 多状态、智谱进度条 0% 四个问题。

---

### P2-7. 所有 adapter 弱类型解析 → strong-typed response struct

**问题**：所有 adapter 用 `resp.json::<serde_json::Value>()` + `.get(...).and_then(...)`。P0-2（DeepSeek 字段错）就是这样爬进来的。

**改法**：每个 adapter 定义 strong-typed response struct

```rust
#[derive(Deserialize)]
struct DeepSeekBalanceResponse {
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,  // 注意是字符串，需要 parse
    #[serde(default)]
    granted_balance: Option<String>,
}

let parsed: DeepSeekBalanceResponse = resp.json().await?;
```

**收益**：加 adapter 测试（路线图 R-9）时用 strong-typed struct 反序列化 fixture，字段变了编译器立刻报错。

---

### P2-8. Provider 元数据硬编码分散在 5 处 → 集中化

**问题**：新增一个平台要改 5 个文件：`adapters/xxx.rs`、`adapters/mod.rs`、`commands.rs`、`types.ts`、`AddProviderForm.vue`。

**修复**：抽 PlatformSpec

```rust
// src-tauri/src/adapters/registry.rs
pub struct PlatformSpec {
    pub id: &'static str,           // "deepseek"
    pub display_name: &'static str, // "DeepSeek"
    pub key_hint: &'static str,     // "以 sk- 开头，54 字符"
    pub color: &'static str,        // 品牌色
    pub build: fn(Arc<Client>) -> Box<dyn QuotaFetcher>,
}

pub const PLATFORMS: &[PlatformSpec] = &[ /* ... */ ];
```

前端通过一个 `get_platform_specs` Tauri 命令读取——真正的单一数据源。

---

## 5. 低优先级 / 清理（P3）

| # | 问题 | 位置 | 修复 |
|:---|:---|:---|:---|
| P3-1 | 未使用的 Cargo 依赖 `thiserror`、`base64` | `Cargo.toml:23,32` | 删除 |
| P3-2 | `main.rs:41` `unwrap()` 可能 panic | `main.rs:41` | 改为 `if let Some(w) = ...` |
| P3-3 | `main.rs:49` 默认图标 `unwrap()` 可能 panic | `main.rs:49` | 同上 |
| P3-4 | `load_providers` 静默吞掉 store 错误 | `commands.rs:23-34` | 至少 `log::warn!` |
| P3-5 | `index.html` `lang="en"` 但 UI 中文 | `index.html:2` | 改为 `lang="zh-CN"` |
| P3-6 | `entitlements.plist` 授予不必要的 JIT 权限 | `entitlements.plist:8-11` | 删除 `allow-jit` 和 `allow-unsigned-executable-memory` |
| P3-7 | `.gitignore` 排除 `/CLAUDE.md` 是错的 | `.gitignore` 末行 | 删除该行，CLAUDE.md 必须进版本控制 |
| P3-8 | Cargo 依赖版本约束松散（`version = "2"`） | `Cargo.toml` 全部 | 至少 pin 到 minor：`version = "2.1"` |
| P3-9 | 确认 `Cargo.lock` 已提交（binary crate 必须） | `src-tauri/Cargo.lock` | `git ls-files` 验证 |
| P3-10 | `tauri.conf.json` 缺 `bundle.category` | tauri.conf.json | 加 `"category": "public.app-category.developer-tools"` |
| P3-11 | Qoder 适配器忽略 API Key，用户被迫填假 Key | `qoder.rs` + `AddProviderForm.vue` | Qoder 时隐藏 Key 输入框 |

---

## 6. 功能路线图

修 bug 之外的可行性建议。按投入产出比排序。

### 6.1 🌟 强推（P0/P1 修完后立刻做）

#### R-1. 菜单栏图标做状态指示（前置 P0-6）

**动机**：菜单栏应用的核心价值是"不打开窗口就能看到状态"。

**方案**：动态切换 `TrayIcon`

| 状态 | 图标 |
|:---|:---|
| 正常 | 默认模板图 |
| 有平台余额低 | 图标 + 红色小圆点 |
| 有平台请求失败 | 图标 + 黄色警告 |
| 全部刷新中 | 图标转灰（可选） |

**实现要点**：准备三套 Template PNG，Rust 端持有 `TrayIconHandle`，在 `get_all_quotas` 返回后调用 `tray.set_icon(...)`。

**工作量**：半天（假设 P0-6 已完成）。

---

#### R-2. 缓存上次成功结果 + "上次更新时间"

**动机**：网络失败时，用户可能只想看"上次查到多少还有大概概念"。

**方案**：
1. 成功返回的 `QuotaInfo` 序列化到 `tauri-plugin-store::quota_cache.json`
2. 启动时先渲染缓存（`stale: true`），再异步刷新覆盖
3. 每张卡右上角加"5 分钟前"（用 `dayjs.fromNow()`）
4. 失败时显示上次数据 + 灰化 + 时间戳

**工作量**：半天。

---

#### R-3. 平台生态扩展（覆盖 80% 中文开发者）

**优先级**（⚠️ **所有端点均需 curl 验证**，不要相信下表字段路径）：

| 平台 | 鉴权 | 余额 API（待验证） | 用户面 | 工作量 |
|:---|:---|:---|:---|:---|
| 月之暗面 Kimi | Bearer | ⚠️ 待查文档 | ⭐⭐⭐⭐⭐ | 2h |
| SiliconFlow | Bearer | ⚠️ 待查文档 | ⭐⭐⭐⭐ | 2h |
| OpenRouter | Bearer | `GET /api/v1/credits` ⚠️ | ⭐⭐⭐ | 2h |
| 阿里 DashScope | Bearer + AK | 无直接 API | ⭐⭐⭐ | 4h |
| OpenAI | Bearer | ❌ **credit_grants API 已在 2024 年下线** | ⭐⭐⭐⭐ | 无解 |
| Anthropic | - | 无余额 API | ⭐ | 跳过 |

**建议先接**：Kimi + SiliconFlow。接完覆盖大部分中文开发者。

**新增平台清单**（配合 P2-8 registry 后，改动会集中在 1 个文件）：
1. `src-tauri/src/adapters/xxx.rs` — 实现 `QuotaFetcher`
2. `src-tauri/src/adapters/registry.rs` — 加一条 PlatformSpec

**关键铁律**：**每个平台接入前必须先 curl 官方 API 拿到真实响应**，存到 `src-tauri/tests/fixtures/` 里再写 parse。

---

### 6.2 💡 值得做（中等成本）

#### R-4. 单张卡片手动刷新
- Rust 加命令 `get_quota_by_provider(provider: String) -> QuotaInfo`
- `QuotaCard.vue` 右上角加 🔄 图标
- 工作量：2 小时

#### R-5. 消耗速率与预测
- 引入 `tauri-plugin-sql`（SQLite）
- 表：`balance_snapshots(provider TEXT, remaining REAL, ts INTEGER)`
- 每次刷新写一行，保留最近 30 天
- 前端算：过去 7d 均值 → 展示 `≈ 12 天` 剩余
- 工作量：半天

#### R-6. i18n（至少加英文）
- `vue-i18n` + `zh-CN.json` + `en-US.json`
- 语言检测：`navigator.language` 自动切换
- 想在 GitHub 传播必须做
- 工作量：半天

#### R-7. 全局快捷键 toggle 窗口
- `tauri-plugin-global-shortcut`
- 默认 `⌥ Space` 或 `⌘⇧ K`
- 菜单栏应用标配交互
- 工作量：2 小时

---

### 6.3 🔧 工程质量（长期回报）

#### R-8. Adapter 单元测试 + 响应快照

**动机**：P0-2（DeepSeek 字段错）之所以潜伏，是因为**根本没有测试**。

**方案**：

```
src-tauri/tests/
├── fixtures/
│   ├── deepseek_balance.json
│   ├── deepseek_error_401.json
│   ├── zhipu_balance.json
│   └── ...
└── adapter_tests.rs
```

用 `wiremock` 起 mock server 或直接把 fixture 传给内部 parse 函数。每个 adapter 最少两个用例：`test_parse_success()` + `test_parse_401_error()`。

**依赖 P2-7 strong-typed struct**。**投入 1 天，长期回报极高**。

---

#### R-9. `tracing` 替代 `env_logger` + 日志落盘
- 换 `tracing` + `tracing-subscriber` + `tracing-appender`
- 或用 `tauri-plugin-log`
- 输出到 `~/Library/Logs/KeyKeeper/keykeeper.log`，按天分文件保留 7 天
- 右键菜单加"打开日志目录"
- 工作量：半天

---

#### R-10. GitHub Actions 自动发布
- `.github/workflows/release.yml`
- 触发：`push: tags: v*`
- 步骤：装 Rust + pnpm → `pnpm tauri build` → 上传 dmg 到 Release
- 后续：加 macOS 签名 + Notarization（需 Apple Developer 证书）
- 工作量：半天（不含签名申请）

---

#### R-11. Homebrew Cask 发布
- 建自己的 tap 仓库（`homebrew-tap`）加 `Casks/keykeeper.rb`
- 每次 Release 更新版本号（可脚本化）
- Mac 开发者分发标配
- 工作量：2 小时

---

#### R-12. macOSPrivateApi 决策

**问题**：`tauri.conf.json:29` `"macOSPrivateApi": true` 启用了透明毛玻璃私有 API。

**取舍**：

| 目标 | 决策 |
|:---|:---|
| Homebrew Cask 分发 | 保留 |
| 上 App Store | 必须替换为 `NSVisualEffectView`（通过 `tauri-plugin-decorum` 或公开 API） |

**至少在文档里明确记录这个决策**。

---

### 6.4 更多补充建议

| # | 项 | 备注 |
|:---|:---|:---|
| R-13 | 暗色模式 | Tailwind `dark:` 变体成本极低 |
| R-14 | 键盘导航 + aria-label | A11y 基本要求 |
| R-15 | 最小刷新间隔保护 | 前端 debounce + Rust 记录上次时间，防狂点 rate limit |
| R-16 | Add Provider 后 debounce 100ms 再 refresh | 批量添加不浪费请求 |
| R-17 | Add Provider 表单根据平台显示 Key 格式提示 | `sk-xxx` 之类的 placeholder |
| R-18 | 通知点击行为 | 打开对应平台充值页（配合 R-1 状态图标） |
| R-19 | 冷启动前端 stale-while-revalidate | 秒开缓存版本，再异步刷新（配合 R-2） |
| R-20 | `PlanType::CodingPlan` 命名过窄 | 未来加"包月订阅"会重名，改 `TimedSession` |
| R-21 | README 改造 | 顶部截图 + 三行价值主张 + 平台图标网格 + 安装方式 + FAQ |
| R-22 | CONTRIBUTING + issue/PR 模板 | 开源仓库标配 |
| R-23 | `rust-toolchain.toml` 需 pin 到具体版本 | 可复现构建 |

---

## 7. 明确不做的事

以下想法看起来"高级"但会让 KeyKeeper 定位失焦，**明确劝阻**：

| 想法 | 为什么不做 |
|:---|:---|
| 多 Key 轮换 / 多账户管理 | 变成 Key 管理器，与"查余额"定位冲突。用户需要这个应该用 1Password / Bitwarden |
| 接入 LLM 分析用量 | 花哨但对"查余额"没帮助，且会引入模型调用成本 |
| 跨平台支持 Windows / Linux | Windows 没真正的"菜单栏"，Linux 生态碎片。先把 macOS 做到极致 |
| 加密存储 config.json 本身 | Key 已经在 Keychain，其他都是元数据，加密无意义还徒增复杂度 |
| 自动化余额充值（跳转支付页） | 合规风险 + 收益低 |
| 内建 AI 助手分析账单 | 定位失焦 |

**原则**：**先把 4 个平台的菜单栏查余额做到 100 分，比铺 10 个平台每个 60 分强**。

---

## 8. 实施顺序建议

> ⚠️ **顺序调整（Volcano 修正后）**：数据模型 v2（P0-3 修复行动 2）**必须先于其他 P0/P1 完成**，否则依赖旧 `QuotaInfo` 结构的修复会返工。

### Sprint 0（半天，前置调查 — 用户手动）

1. 火山方舟控制台 DevTools 抓 API（P0-3 修复行动 1）
2. Qoder 用量页 DevTools 抓 API（P0-4）
3. DeepSeek `/user/balance` curl（P0-2）
4. 智谱 `/api/paas/v4/balance` curl（P0-5 智谱部分）
5. 所有 fixture 存到 `src-tauri/tests/fixtures/`

**此步骤完成之前，Sprint A 不要开始**。

### Sprint A（1-2 天，数据模型 v2 + Volcano 上线）

**目标**：Volcano 从"完全不可用"到"可显示套餐/多维度用量/告警"，是 KeyKeeper 当前的主要价值。

1. P0-3 修复行动 2：定稿 `QuotaInfo` v2 数据模型（`Vec<QuotaMetric>` + `Subscription` + `QuotaStatus`）
2. P0-3 修复行动 3：Volcano 适配器 v2 重写
3. P0-4：Qoder 按调查结果落地（API 版 or 本地估算循环版）
4. P0-2：DeepSeek 迁移到 v2 + 修余额解析
5. P0-5：智谱迁移到 v2（进度条问题自然消失）
6. P0-1：`PlanType` 序列化（顺手）
7. 前端 QuotaCard v2 重设计：套餐信息卡 + 多进度条 + 状态标签

### Sprint B（1 天，用户可见 bug 收尾）

8. P0-3 修复行动 4：告警门槛落地（月度 ≥80% / ≥95% / 套餐 ≤7 天 / 自动续费 off + ≤3 天）
9. P0-6 托盘 Template Image
10. P0-7 CLAUDE.md/README 移除虚假功能
11. P0-8 setTimeout 泄露
12. P1-1 启动重复刷新
13. P1-5 npm → pnpm
14. P3-5 `lang="zh-CN"`

### Sprint C（1-2 天，架构清理）

15. P1-2 providers 竞态
16. P1-3 错误响应脱敏
17. P1-4 移除 shell 权限
18. P1-6 scheduler panic 兜底
19. P1-7 删掉死代码 onUnmounted
20. P2-4 锁类型 + 跨 await
21. P2-7 strong-typed response struct（配合 v2 数据模型）
22. P3-1 清理未使用依赖

### Sprint D（1 天，功能补齐）

23. P1-8 + P2-1 低额度通知接入（消费 Sprint B 的告警门槛）
24. P2-2 阈值统一
25. P2-3 Qoder 时间持久化错误处理（若仍需本地估算）
26. P2-5 更新 Key 语义
27. P2-8 PlatformSpec 集中化
28. P3-11 Qoder 无 Key 流程（若仍需本地估算）
29. R-1 托盘状态图标（依赖 P0-6）
30. R-2 缓存 + 时间戳

### Sprint E（生态扩展）

31. R-3 Kimi + SiliconFlow（curl 验证 → fixture → 实现）
32. R-4 单卡刷新
33. R-7 全局快捷键
34. R-15 最小刷新间隔

### Sprint F（1.0 发布前）

35. R-8 adapter 测试补齐
36. R-9 日志落盘
37. R-6 i18n（英文）
38. R-13/14 暗色模式 + a11y
39. R-21 README 改造
40. P3-6/7/8/9/10 发布配置收敛
41. R-10 GitHub Actions
42. R-12 macOSPrivateApi 决策落地
43. R-11 Homebrew Cask

### Sprint G（长期）

44. R-5 消耗速率
45. R-3 更多平台
46. R-18/19 通知交互 + 冷启动优化
47. 参考 [ux-improvements.md](ux-improvements.md) 中 U-1~U-25 按需消费

---

## 9. 方法论反思

本次审计前后经历了四轮：初审 → 路线图 → 对抗性复审 → 用户提供 Volcano 控制台截图后再修正。**第四轮修正暴露了最严重的错误**，全部记录如下：

### 9.1 已发现的方法论错误

1. **未查文档就给结论**（**最严重**）：Volcano 我推理"没有余额 API"、"应该改 Bearer"，用户截图直接推翻——Volcano **有完整的官方套餐/多维度用量 API**。类似的错误：假设 Qoder"无公开接口"（可能也是错的）、CLAUDE.md 说啥我信啥。**教训：读代码 ≠ 读文档 ≠ 抓 API，三件事都要做**。

2. **模式匹配 ≠ 事实**：看到 Volcano 用 HMAC-SHA256 就自动套 AWS SigV4 心智模型，得出"应该没有 `VOLC` 前缀"——但我从未查过火山官方签名规范。同类错误：断言 DeepSeek/Kimi/SiliconFlow 的字段路径。

3. **修复代码里带同样的 anti-pattern**：初审给的 DeepSeek 修复代码依然用 `unwrap_or(0.0)`，保留了原 bug 的核心症状（解析失败还是显示 0）。**审 bug 时没审自己的修复方案**。

4. **表面症状 vs 根源**：初审修的是"Volcano 3 个签名 bug"，第二轮说"改用 Bearer"，第三轮说"加 `Option<f64>`"——**每次都在打补丁**，真正的根源是"`QuotaInfo` 单值模型无法表达真实业务"。截图之后才明白：Volcano 是多维度用量 + 套餐信息 + 自动续费 + 有效期，一个 `total + remaining` 怎么改都不够。**教训：如果补丁越打越丑，很可能数据模型本身就是错的**。

5. **只审 API 层，没审 UI 语义层 + 业务语义层**：只审 API 是否报错、字段是否解析对，没审"用户看到的数字和进度条对不对"，更没审"用户查这个数字是为了做什么决策"。用户实际关心的是"下个月要不要续费 / 现在能不能撑到月底"，而不是"remaining 是多少"。

6. **过时的记忆当作事实**：断言"OpenAI credit_grants API 可用"，实际该 API 2024 年已下线。

### 9.2 修正后的审查方法

以后写"修复计划"和"路线图"时，除了列改动，还要：

1. **每个"事实断言"标注证据来源**：`✅ 已 curl 验证` / `📖 官方文档链接` / `⚠️ 推理未验证`。**未验证的断言必须走 Sprint 0 抓 API 环节，不允许直接进入实施**。
2. **修复代码本身跑一遍代码审查**：不能只审原代码。审查自己写的修复时，仍然要问"这段代码有没有沉默失败？"、"有没有跨 await 持锁？"
3. **走一遍用户视角 walkthrough**：从"点击托盘图标"开始，每步问"用户看到什么？和预期一致吗？他会做什么？"
4. **走一遍业务视角 walkthrough**：问"用户为什么装这个 App？他要做什么决策？现有信息够不够？"
5. **区分 API 层 bug、数据模型层 bug、业务模型层 bug**：字段解析错是 API 层；`total: f64` 而非 `Option<f64>` 是数据模型层；`Vec<QuotaMetric>` 而非单值是业务模型层。越往深越会波及大范围代码，越晚发现越贵。
6. **审查文档-实现一致性**：CLAUDE.md 说的功能是否真接入了？README 承诺的能力是否可用？CLAUDE.md 的"事实"是否本身是错的（如"Volcano 无公开 API"）？

### 9.3 保留原则

- **接入任何平台前必须先 curl 或抓 XHR**，把响应存 `tests/fixtures/`
- **静默失败是 Bug**：`unwrap_or(0.0)` / `let _ = ...` / `filter_map(|r| r.ok())` 是三种同类型的静默失败模式，除非明确注释理由，否则默认视为 bug
- **改数据模型是 breaking change，一次做完**：`QuotaInfo` 每次结构变更必须同步 `models.rs` + 所有 adapter + `types.ts` + `QuotaCard.vue`，不要分批
- **CLAUDE.md 里的"事实"不等于事实**：CLAUDE.md 可能是初期实现者的假设记录，不是真理。质疑之。

---

## 变更记录

| 日期 | 变更 |
|:---|:---|
| 2026-07-23 | 初版（合并自 fix-plan.md、roadmap.md、adversarial-review.md 三份文档） |
| 2026-07-27 | **对抗性审查修正**：逐条验证文档断言 vs 当前代码，发现 P0-1/P1-5/P3-5 三项文档断言错误（代码已修复）；实施 P0-8/P1-1/P1-2/P1-3/P1-4/P1-6/P1-7/P3-1/P3-7 共 9 项修复；P0-2/P0-3/P0-4/P0-5/P0-6/P2-6 共 6 项因依赖真实 API Key / 设计资源未实施，需用户完成 Sprint 0 抓包后继续。 |
| 2026-07-23 | **Volcano 认知修正**：用户提供火山方舟控制台截图，推翻 P0-3 的"无 API"假设。P0-3 重写为"抓真实 API + 数据模型 v2 + 多维度用量"；P0-5 并入 P0-3；P2-6 并入 P0-3；Sprint 顺序重排，新增 Sprint 0 前置调查阶段；方法论反思增加第 1 条"未查文档就给结论"。易用性建议独立到 `ux-improvements.md`。 |
