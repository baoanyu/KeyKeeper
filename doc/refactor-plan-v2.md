# KeyKeeper v2 改造计划

> 制定时间：2026-09 · 状态：**基本完成**（2026-09-23：Phase 0 / 1 / 2 / 4 已完成并提交；Phase 3 已按官方文档 / 实证契约落地三条自动查询，**唯一剩余阻塞是批次 C「真实 Key 终验」**——见 [code-review-2026-09-remediation.md](./code-review-2026-09-remediation.md) §5）
> 本文档整合三件事：① 产品形态改造 ② 本次全量代码审查发现的问题 ③ 平台扩展。
> 相关文档：[requirements-backlog.md](./requirements-backlog.md)（历史 backlog，本文档实施后会吸收其中多项）

### 修订记录

| 版本 | 变更 |
|:---|:---|
| v1 | 初稿 |
| v2 | 对抗性审查后修订：<br>· **F-1** §3.1「30 天」结论降级（证据不足，存在幸存者偏差）<br>· **F-2** 修正 §3.2 与 Phase 2 关于 Volcano 的自相矛盾<br>· **F-3** 新增 §1.4「后台行为」——原缺失，且阻塞形态改造<br>· **F-4** 超算 / 豆包工作标注为待确认，修正 §6 的「已解决」误述<br>· **F-5** 数据模型精简：5 个类型 → 2 个<br>· **F-6** Phase 重排：核心价值优先于外壳改造<br>· **F-7~F-14** 补工作量估算、编辑交互、到期语义、Phase 2 预案等 |
| v3 | 用户决策落地：<br>· **§1.4 定案「关窗退出」** → **系统通知整体移除**，改为应用内到期标记（§2.7）<br>· **P1-3 改为「改造消解」** —— 通知删除后节流问题不存在<br>· **§3.2 更正**：「超算 DeepSeek」是独立产品，**不是** scnet.cn（我此前的归类是错的）<br>· 「豆包工作」按用户指示采用推荐值（仍未验证）<br>· 新增 §1.4 备选：「黄灯最小化时继续提醒」待确认，默认不实现 |
| v4 | 全量代码评审后同步（提交 `3a963ec` / `04f611b`）：<br>· **§2.2 模型 v2.1**：`Entitlement` 增加 `used_percent`（配额窗口型平台），补回 §2.5 展示规则<br>· **§2.5 新增「已用 X%」卡片形态**（智谱 / 方舟类配额窗口接口）<br>· **§5 Phase 3 落地方式变更**：不再依赖用户抓包，改由官方文档（DeepSeek）/ 实证实现（智谱）/ 官方签名 Demo（火山）驱动，fixture 已入库<br>· **§5 Phase 4 完成**；**§6 待确认项收敛**（原 Q2/Q4 部分解决）<br>· 新增评审文档 [code-review-2026-09-remediation.md](./code-review-2026-09-remediation.md) 为问题清单与终验清单的权威来源 |

---

## 0. 背景与决策记录

### 0.1 产品定位的重新确认

当前实现把自己定位成「**余额查询器**」：`QuotaInfo { remaining: f64 }` + 进度条。

但用户实际记录的是这个：

```
超算 DeepSeek 0.1：   9.23-10.23
超算 DeepSeek 10M 体验：9.23-10.23
小米 mimo：           9.22-10.22
豆包工作：             9.25-10.25
longcat：             9.10-10.10
```

**是有效期区间，不是余额。** 用户真正想知道的是「**还剩几天到期**」。

→ 产品定位修正为：**AI 平台额度与订阅管理台**。余额只是"额度"的一种形态。

> ⚠️ 注意这个结论的**直接证据只有一条**：用户就是这么记录的。不需要外推到"平台普遍如何"——见 §3.1 的证据分级。

### 0.2 本次审查的核心结论

> **四个平台（DeepSeek / 智谱 / Qoder / Volcano）的 API 响应，一次都没有被真实验证过。**
> 本机 Keychain 四个平台全空、store 文件不存在 —— 该应用从未跑通一次完整流程。

所有适配器都是在"猜字段、猜端点"的前提下写的。这决定了本计划的排序：**先建不依赖 API 的地基，再逐个验证 API**。

### 0.3 已确认的决策

| 决策项 | 结论 |
|:---|:---|
| 窗口形态 | **纯独立窗口应用**，去掉菜单栏托盘 |
| 新平台数据源 | **混合模式**：能自动的自动查，其余手动录入 |
| 豆包工作 | **无外部 API** → `Manual` 模式（手动录入到期日） |
| 火山方舟 | **有 API**，应走自动查询 |
| 真实响应验证 | **用户提供**真实响应样本，据此修正适配器 |

---

## 1. 目标形态

### 1.1 菜单栏应用 → 独立窗口应用

| 文件 | 改动 |
|:---|:---|
| `src-tauri/tauri.conf.json` | 移除 `alwaysOnTop`；`decorations: false` → `true`；`visible: false` → `true`；新增 `resizable: true` + `minWidth/minHeight`；`transparent` 保留（毛玻璃） |
| `src-tauri/Info.plist` | **删除 `LSUIElement`**（否则无 Dock 图标、⌘Tab 无法切换，不符合"独立应用"预期） |
| `src-tauri/src/main.rs` | 删除托盘相关（`TrayIconBuilder` / `Menu` / `MenuItem`，约 40 行）、失焦自动隐藏（`WindowEvent::Focused(false)`）、`toggle_window()`、`position_window_below_tray()` |
| `src/App.vue` | 顶栏加拖拽区（`data-tauri-drag-region`） |
| `src/style.css` | 移除 `width: 400px; height: 600px` 硬编码，改为响应式 |

**验证标准**：`pnpm tauri dev` 启动后 —— 有 Dock 图标、窗口有标题栏可拖动、可缩放、⌘W 关闭、⌘Tab 可切换。

### 1.2 数据模型：余额 → 额度 + 订阅

见 §2。

### 1.3 混合数据源

```
PlatformSpec.mode:
  Api      → 有公开 API，走 Keychain + 适配器自动查询
  Manual   → 无 API，用户手动录入到期日，应用负责倒计时 + 临期标记
```

> v1 曾列出第三种 `LocalEstimate`（Qoder 用）。**已删除** —— Qoder 在 §3.3 被降级为 `Manual`，该模式当前没有使用者。等真需要再加，避免 speculative 设计。

**关键收益**：`Manual` 模式让"新增平台"**不再依赖对方有没有 API**。

### 1.4 后台行为（已决策）

> **决策（用户拍板）：关闭窗口即退出应用。**

**这个选择的硬后果：系统通知失效。** 通知需要进程活着才能发出 —— 关窗即退出后没有后台进程，`check_low_balance` + `sendNotification` 这套机制**整体作废**。

也就是说，记录到期日的目的是"别忘"，但应用**不会再主动提醒**，需要用户自己想起来打开。

#### 决策落地

| # | 决策项 | 结论 |
|:---|:---|:---|
| a | 关闭窗口 = 退出应用吗？ | **是**。`WindowEvent::CloseRequested` → `app.exit(0)` |
| b | 关窗后还自动刷新吗？ | **否** —— 进程已退出，无从刷新 |
| c | 系统通知 | **移除**。改为**应用内到期标记**（见下） |
| d | 唤回窗口 | 点 Dock 图标重新启动 |

#### 替代方案：应用内到期标记

不常驻后台 ≠ 不需要知道快到期了。改为：**每次打开应用时，临期条目在列表顶部高亮/置顶**。

| 条件 | 应用内表现 |
|:---|:---|
| 到期 ≤ 3 天 | 🔴 置顶 + 红色标记 |
| 到期 ≤ 7 天 | 🟡 橙色标记 |
| 其他 | 按到期日排序 |

核心价值（"不忘记"）保留，只是从**推送**变成**拉取**。

> ⚠️ **备选（待你确认）**：macOS 上「点黄灯最小化」与「点红灯关闭」是两个不同动作 —— 最小化时进程仍在，**可以继续提醒**。若需要"平时不占资源、最小化时能提醒"，可保留该行为。**默认不实现，等你确认。**
>
> ⚠️ 若改为"关窗隐藏 + 后台常驻"，则通知机制需整体恢复 —— 这是二选一，没有中间态。

**验证标准**：点窗口关闭按钮后 `ps aux | grep keykeeper` **无残留进程**。

---

## 2. 数据模型 v2（breaking change，一次做完）

### 2.1 为什么必须是 v2

当前 `QuotaInfo` 表达不了三件真实存在的事：

1. **一个平台多个额度包** —— 图里 `超算 DeepSeek` 同时有 `0.1` 和 `10M 体验` 两个独立到期日
2. **到期时间** —— 当前模型完全没有时间维度（`Qoder` 是硬塞进 `remaining` 的秒数）
3. **数据来源** —— 无法区分"API 精确值 / 手工录入"

### 2.2 新模型（已按 F-5 精简）

> v1 设计了 5 个类型（`DataSource` / `Trust` / `Entitlement` / `QuotaMetric` / `PlatformStatus`）。
> 对抗性审查后砍到 2 个 —— `Trust` 与 `DataSource` 语义重叠、`resets_at` 当前零用户、`used` 可由 `total - remaining` 推导。

```rust
/// 数据来源。合并了原 DataSource 与 Trust：
/// "是否估算" 和 "是否手动" 是同一个维度的两种取值，不需要两个枚举。
pub enum Source {
    Api,       // 自动查询，精确值
    Manual,    // 用户手动录入
}

/// 一个额度包（对应图里的一行记录）
pub struct Entitlement {
    pub label: String,             // "0.1" / "10M 体验" / "余额"
    pub expires_at: Option<i64>,   // 到期时间戳；used_percent 存在时语义为「重置时刻」
    pub unit: QuotaUnit,
    pub total: Option<f64>,        // 有 total 才画进度条
    pub remaining: Option<f64>,
    pub note: Option<String>,      // 估算说明 / 错误信息 / 备注，一个字段覆盖
    // ↓ v2.1 追加（见下方说明）
    pub used_percent: Option<f64>, // 已用百分比 0–100
}
```

> **v2.1 追加 `used_percent` 的理由**（2026-09-23，代码评审 R-5）：
> 智谱 `monitor/usage/quota/limit` 与火山方舟类接口返回的是**配额窗口**（「已用 42%，X 时刻重置」），
> **没有** `total` / `remaining` 绝对值 —— 强行套 `total/remaining` 只能伪造数字。
> 故按真实契约加一个字段：`used_percent.is_some()` 时卡片主文案为「已用 X%」，`expires_at` 解释为重置时刻。
> v1 精简时砍掉的 `QuotaMetric.used`（理由是 `used = total - remaining` 可推导）在**没有 total 的平台不成立**，此处是按新证据的修正，不是回退。

```rust
pub struct PlatformStatus {
    pub id: String,                // 稳定标识 "deepseek"
    pub display_name: String,      // "DeepSeek"
    pub source: Source,
    pub entitlements: Vec<Entitlement>,
    pub console_url: Option<String>,
    pub updated_at: i64,
}
```

**被砍掉的字段及理由**：

| 字段 | 砍掉的理由 |
|:---|:---|
| `Trust` 枚举 | 与 `Source` 语义重叠（`Trust::Manual` ≡ `Source::Manual`）；`Failed` 用 `note.is_some()` 表达 |
| `QuotaMetric.resets_at` | 只服务于 Qoder 的 5 小时窗口，而 Qoder 已降级为 `Manual` → **当前零用户**（配额窗口型平台的重置时刻复用 `expires_at`，见 v2.1 说明） |
| ~~`QuotaMetric.used`~~ | ~~`used = total - remaining`，可推导~~ —— **v2.1 部分推翻**：该推导在无 `total` 的配额窗口平台上不成立，故以 `used_percent` 形式恢复 |
| `PlatformStatus.message` | 合并进 `Entitlement.note` |

> 恢复条件：若 Qoder 未来确认真实接口并走自动查询，再引入 `resets_at`。

### 2.3 手动录入的持久化

用现有的 `tauri-plugin-store`（JSON），**不引入 SQLite**。

理由：手动录入条目量小（<20 平台）、结构简单、已有依赖。SQLite 是为 R-5（历史快照/趋势预测）准备的，等真要做趋势时再引入。

**迁移**：无需迁移。现有 store 只存 `providers_list: Vec<String>`，且本机 store 为空、应用从未发布 —— 新结构直接覆盖写入即可。**这一点明确写出，避免实施时纠结。**

### 2.4 到期日的时刻语义（新增）

"10.23 到期"必须有明确含义，否则"还剩 N 天"算不准：

- **存储**：存**日期**（`YYYY-MM-DD` 语义），落库为**本地时区当天 23:59:59** 的时间戳 → 到期日当天仍可用
- **展示**：按**自然日**计算剩余天数（`到期日 - 今天`），不做小时级倒计时
- **标记判断**：以自然日为单位，在渲染列表时计算

### 2.5 前端展示规则

| 情况 | 卡片主信息 | 副信息 |
|:---|:---|:---|
| 有 `expires_at` | **还剩 N 天** | 到期日 + 额度包名 |
| 只有 `remaining` | **¥X / N tokens** | 进度条（`total` 已知时） |
| 两者都有 | 还剩 N 天 | 余额 + 进度条 |
| **有 `used_percent`** | **已用 X%** | 重置时刻（即 `expires_at`）+ 进度条 |
| `source = Manual` | 同上 | 加"手动"标记 + **编辑入口** |

### 2.6 手动录入的交互（新增 · 原为空白）

> v1 只写了两个字"可编辑"。但这是**每天要用的核心功能**，交互必须定清楚。

| 场景 | 交互 |
|:---|:---|
| **新增** | 表单：平台（下拉，含预置清单）+ 额度包名（可选）+ 到期日（date picker） |
| **编辑** | 卡片上点"编辑"→ 就地展开表单，或弹出小编辑层 |
| **续费** | 直接改到期日（**不新增条目**）。若确实新增了额度包，才走"新增" |
| **快捷录入** | 提供「+30 天」「+7 天」按钮 —— 因为多数额度包是月度周期（但见 §3.1，周期需你确认） |
| **删除** | 卡片上的 ✕（复用现有交互） |

### 2.7 到期标记规则（原「提醒规则」）

> 因 §1.4 决策"关窗退出"，**系统通知已移除**。以下规则改为**应用内标记**，不再是推送。

| 条件 | 应用内表现 |
|:---|:---|
| 到期 ≤ 3 天 | 🔴 置顶 + 红色 |
| 到期 ≤ 7 天 | 🟡 橙色 |
| 余额 < 单位阈值 | 🟠 橙色 |
| `used_percent` ≥ 95 | 🔴 进度条红色 |
| `used_percent` ≥ 80 | 🟠 进度条橙色 |

**不需要节流** —— 应用内标记是**状态**而非**事件**，用户打开就看到，不存在"重复打扰"。原 P1-3 的节流问题随之消失（见 §4）。

---

## 3. 平台清单

> 调研方式说明：本环境的 `WebFetch` 工具被网络策略阻断（连 example.com 都无法访问），调研经 `curl` + Bing 完成。
> **下表按证据强度分级**：✅ 已确证 · ⚠️ 部分证据 · ❓ 待确认

### 3.1 关于「30 天」——一个需要撤回的过度推断

> **v1 曾断言"全部是 30 天有效期型"，并据此推导平台普遍模式。这个推断证据不足，现予降级。**

用户记录的五条跨度确实都是 30 天：

| 记录 | 跨度 |
|:---|:---|
| 超算 DeepSeek 0.1：9.23-10.23 | 30 天 |
| 超算 DeepSeek 10M 体验：9.23-10.23 | 30 天 |
| 小米 mimo：9.22-10.22 | 30 天 |
| 豆包工作：9.25-10.25 | 30 天 |
| longcat：9.10-10.10 | 30 天 |

**但"5 条都是 30 天"有更平庸的解释**——看开始日期：`9.23 / 9.23 / 9.22 / 9.25 / 9.10`。前四条集中在 **9.22–9.25 三天内**，这更像**你在那几天集中买了一波月付套餐**，而非"平台普遍 30 天制"。

从单个用户的一次行为推广到平台模式，是典型的幸存者偏差。

**证据分级**：

| 论断 | 强度 | 依据 |
|:---|:---|:---|
| LongCat Token 资源包 30 天有效 | ✅ | 官方文档原文："购买后 30 天内有效" |
| MiMo 有 Token Plan + "有效期内"概念 | ⚠️ | 前端 bundle 提取，**未提 30 天** |
| 火山有 Agent/Coding Plan 订阅 | ⚠️ | 文档导航文本，**未提 30 天** |
| 超算 / 豆包工作是 30 天周期 | ❓ | **无证据** |

**保留的结论**：用户关心的是**有效期**而非余额 —— 这是直接证据（你就是这么记的），成立。
**撤回的结论**：平台普遍是 30 天制 —— 不成立，待验证。

→ **对设计的影响**：卡片主信息仍是「还剩 N 天」（因为记录里就是这个），但 §2.6 的「+30 天」快捷按钮**只是便利，不代表系统假设周期是 30 天**。

### 3.2 平台调研结果

| id | 全称 | 控制台 | 自动查询 | 证据 |
|:---|:---|:---|:---|:---|
| `volcano` | 火山方舟（火山引擎） | console.volcengine.com/ark | ✅ 有 API（余额接口待抓包定位） | ⚠️ 用户确认 + 文档导航 |
| `deepseek` | DeepSeek 开放平台 | platform.deepseek.com | ⚠️ 字段存疑，待验证 | ⚠️ |
| `zhipu` | 智谱 AI 开放平台 | open.bigmodel.cn | ⚠️ 端点待验证 | ⚠️ |
| `longcat` | LongCat API 开放平台（美团） | longcat.chat | 🔍 有对话 API（`api.longcat.chat`），**文档未提供余额/资源包查询接口** | ✅ 文档已抓 |
| `xiaomi-mimo` | Xiaomi MiMo 开放平台 | platform.xiaomimimo.com | 🔍 有 `/console/balance`、`/tokenPlan/usage`，疑似登录态 | ⚠️ bundle 提取 |
| `chaosuan-deepseek` | 超算 DeepSeek | —（独立产品，非 scnet.cn） | ❌ `Manual` | ✅ 用户确认"是一个产品" |
| `doubao-work` | 豆包工作 | doubao.com | ❌ 无外部 API（你已确认） | ⚠️ 按推荐值采用，**未验证** |
| `qoder` | Qoder | qoder.dev | ❌ 无公开接口 | ⚠️ |

> ⚠️ 两项说明：
> - **`chaosuan-deepseek`**：用户确认"超算 DeepSeek 是一个产品"，**不是** scnet.cn（国家超算互联网）。我此前把它归到 scnet.cn 是错的。作为独立 `Manual` 条目处理，控制台 URL 留空待补。
> - **`doubao-work`**：我搜"豆包工作"被中文词典结果污染，**从未验证**。`doubao.com` 是按用户"按推荐值"的指示采用，仍属未验证值 —— 若后续发现不对，只改 `PlatformSpec` 一处即可（§3.4）。

**判读**：
- 有把握走 `Api` 模式的是 **火山方舟**（你已确认有 API）
- **LongCat / MiMo / 超算** 虽有开放平台，但余额查询疑似只在登录态控制台 → **先按 `Manual`**
- **豆包工作 / Qoder** 明确 `Manual`

> 这张表的价值在于：它把"猜 API"的冲动按住了。**除火山外，其余平台都需要先抓到真实响应**才能确定能否自动查 —— 见 §5 Phase 3。

### 3.3 Qoder 的处置

Qoder 目前是**三重损坏**（详见 §4 P0-1），且其"5 小时滚动窗口"是未验证的假设。

**处置**：本轮降级为 `Manual`。等确认是否有真实接口再决定。**不要继续在未验证的假设上投入。**

### 3.4 平台元数据集中化（吸收 backlog P2-8）

当前新增一个平台要改 **5 处**（`types.ts` / `commands.rs` / `AddProviderForm.vue` / `QuotaCard.vue` / `main.rs`）。改为单一 `PlatformSpec`：

```rust
pub struct PlatformSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub mode: PlatformMode,          // Api | Manual
    pub console_url: &'static str,
    pub key_docs_url: Option<&'static str>,
    pub key_hint: Option<&'static str>,
    pub key_pattern: Option<&'static str>,
}
```

前端通过 `get_platform_specs` 命令读取，**新增平台只改 Rust 一处**。

---

## 4. 本次审查发现的问题及处置

> 处置方式：**🔧 修复** · **♻️ 被改造消解** · **↩️ 撤回**（原判断有误）

### P0

| # | 问题 | 处置 |
|:---|:---|:---|
| P0-1 | **Qoder 三重损坏**：`add_provider` 被 `has_key` 拦截 → 加不进去；`get_all_quotas` 被 `get_key` 失败拦截 → 不显示；`delete_provider` 被 `delete_key` 失败拦截 → 删不掉。根因是 P3-11 与对抗性审查 #3/#7 两个修复互相冲突，Qoder 路径无人验证 | ♻️ 改造消解 —— 新模型下 Qoder 走 `Manual`，不碰 Keychain |
| P0-1b | **同一根因的普适版本**：任何 Keychain 条目缺失的平台（用户在「钥匙串访问」手动删过 key、换机迁移后）都**永久删不掉** | ♻️ **改造消解**（v1 误标为"🔧 修复"，与 P0-1 不一致，已更正）—— `Manual` 平台不进 Keychain；`Api` 平台的删除路径需容忍 `NoEntry`，在 Phase 1 实现 `delete` 时一并处理 |
| P0-2 | **两份 `rust-toolchain.toml` 冲突**：根目录 pin `1.97`，`src-tauri/` 是 `stable`。cargo 从 cwd 向上取第一个 → 实际构建用 `stable`，根目录的 pin **完全无效**，还白下载了一套 46M 的残缺工具链 | 🔧 修复：删除 `src-tauri/rust-toolchain.toml`，把 `targets` 合并进根目录那份；`rustup toolchain uninstall 1.97 1.97.1` 清理残留 |

### P1

| # | 问题 | 处置 |
|:---|:---|:---|
| P1-1 | **外链全部失效**：`window.open`（`QuotaCard.vue:68`）与 `<a target="_blank">`（`AddProviderForm.vue:126`）都是静默无操作。已逐层验证源码：tauri 2.11.5 `webview/mod.rs:354` 默认 `new_window_handler: None` → wry 0.55.1 `wry_web_view_ui_delegate.rs:262` 返回 `nil` → WebKit 不创建窗口、**不回退系统浏览器** | 🔧 修复：`Cargo.toml` 加 `tauri-plugin-opener`；`main.rs` 注册 `.plugin(tauri_plugin_opener::init())`；capability 加 `opener:default`。**注意**：装插件后 `<a target="_blank">` 自动可用，但 **`window.open` 仍无效**（opener 只注入 DOM click 监听，不 patch `window.open`）→ `QuotaCard.vue:68` 必须显式改用 `openUrl()` |
| P1-2 | **表单验证失败后 Key 被清空**：`AddProviderForm.vue:81-86` 在 `emit` 后立即 `apiKey = ''` + `showForm = false`，用户须重新粘贴整串 Key | 🔧 修复：改为由父组件验证成功后再清空/收起 |
| P1-3 | **通知无节流**：`checkAndNotify` 挂在每次 `refresh()` 后，自动刷新 5 分钟一次 → 余额低/即将到期时用户每 5 分钟被打扰 | ♻️ **改造消解**（v1 误标为"🔧 修复"）—— §1.4 决策移除系统通知后，`checkAndNotify` + `sendNotification` 整体删除，节流问题不存在 |
| P1-4 | **同一平台重复点「重新配置」无反应**：`App.vue:167` `reconfigureProvider` 永不重置，`watch` 因值未变不触发 | 🔧 修复：改为传递增 token 或消费后置回 `null` |

### P2

| # | 问题 | 处置 |
|:---|:---|:---|
| P2-1 | **`main.rs` 与 `lib.rs` 重复模块树**：两边各自 `mod models; mod keystore; ...`，`main.rs` 完全没用 lib → 整个后端编译两遍，`cargo test` 实测跑两遍 | 🔧 修复：`main.rs` 改为调用 `keykeeper::run()`，模块只声明一次。顺带 `check_low_balance` 移入 `commands.rs`（CLAUDE.md 已如此声明，代码与文档不符） |
| P2-2 | **`pnpm` 配置三处打架**：`pnpm-workspace.yaml` 内容是占位符文本（`allowBuilds` 不是有效字段）；`package.json` 的 `pnpm.onlyBuiltDependencies` 已被 pnpm 忽略（构建时打 WARN）；真正生效的是 `.npmrc` | 🔧 修复：统一到 `pnpm-workspace.yaml` 的 `onlyBuiltDependencies`，删除另两处 |
| P2-3 | **`doc/` 未纳入版本控制**：`.gitignore:32-34` 忽略 `/doc/`，而 `requirements-backlog.md` 是唯一路线图文档 | 🔧 修复：从 `.gitignore` 移除 `/doc/`（`CLAUDE.md` / `AGENTS.md` 是否保留见 §6） |
| P2-4 | **README 过期**：`README.md:12` 写"低额度通知（计划中）" | 🔧 修复：随改造重写。**注意**：通知功能本轮已决定移除（§1.4），README 不应再提"低额度通知" |
| P2-5 | **clippy 3 个 warning**：`models.rs:14` `CNY` 大写缩写、`main.rs:63` `match` 当等值判断、`main.rs:133` `if let` 可折叠 | 🔧 修复：`cargo clippy --fix` + 手工处理 |
| P2-6 | **无 CSP**：`tauri.conf.json:45` `"csp": null` | 🔧 修复：改造后补上基础 CSP |
| P2-7 | **`entitlements.plist` 的 `com.apple.security.device.keychain` 非 Apple 标准 entitlement key**；非沙盒应用本就不需要 | 🔍 待上架时验证，暂不动 |
| P2-8 | **测试仅 2 个且重复跑两遍**；adapter 字段解析零覆盖 —— 而字段错误正是本项目最大的风险敞口 | 🔧 修复：随 Phase 3 补 fixture + 解析测试 |

### 撤回项

| 原判断 | 撤回理由 |
|:---|:---|
| ~~通知权限未请求会导致通知失败~~ | **撤回**。实测 `tauri-plugin-notification 2.3.3` 桌面端 `request_permission()` 无条件返回 `Granted`，`sendNotification` 全链路无权限检查 —— 加不加都一样。<br>但需记录两个现实坑：① `pnpm tauri dev` 下通知归属**终端 App**，需给终端开通知权限；② 该插件底层是已废弃的 `NSUserNotificationCenter`，**无法触发 macOS 权限弹窗**，能否看到横幅取决于系统设置。**开发期不要在这上面浪费时间排查** |

> 📌 **注**：§1.4 已决定移除系统通知，本条目前**仅作存档**。若将来恢复通知功能（如改回"后台常驻"模式），上述两个坑仍然适用，不必重新排查。

---

## 5. 执行阶段

> **v1 的排序已按 F-6 调整**：原方案把「独立窗口改造」放在最前，但那是**外壳**；用户真正的痛点是图里那些手写的到期日，那是**核心价值**。
> 先做外壳的结果是——换了个窗口形态，痛点仍在。**现改为核心价值优先。**

### Phase 0 —— 地基清理（无外部依赖 · 约 1 小时）

| # | 任务 | 验证标准 |
|:---|:---|:---|
| 0.1 | 合并 `rust-toolchain.toml` + 清理残留（P0-2） | 根目录与 `src-tauri/` 下 `rustc --version` 一致，且不再触发下载 |
| 0.2 | 合并模块树（P2-1） | `cargo test` 只跑一遍；`cargo check` 通过 |
| 0.3 | pnpm 配置统一（P2-2） | `pnpm build` 无 WARN |
| 0.4 | clippy 清零（P2-5） | `cargo clippy --all-targets` 零 warning |
| 0.5 | `doc/` 纳入 git（P2-3） | `git ls-files doc/` 非空 |

### Phase 1 —— 核心价值：到期管理（约 1.5 天）

> 完成即可**立刻替代你手写的那份记录**。

| # | 任务 | 验证标准 | 估时 |
|:---|:---|:---|:---|
| 1.1 | `PlatformSpec` 集中化（§3.4） | 新增平台只改 Rust 一处；`get_platform_specs` 返回完整元数据 | 3h |
| 1.2 | 数据模型 v2（§2.2 精简版） | `cargo check` 通过；序列化与前端 TS 类型对齐 | 2h |
| 1.3 | 手动录入 CRUD + store 持久化（§2.3） | 重启应用后录入的到期日仍在 | 3h |
| 1.4 | 录入/编辑交互（§2.6） | 能新增、改期、删除；「+30 天」快捷键可用 | 2h |
| 1.5 | 卡片重设计（§2.5） | 到期型 / 余额型 / 双信息三种卡片均正确渲染；`Manual` 有标记与编辑入口 | 2h |
| 1.6 | 到期标记（§2.7） | 到期 ≤3 天的条目置顶且标红；≤7 天标橙；列表按到期日排序 | 1h |
| 1.7 | 预置平台录入（§3.2 中 `Manual` 的项） | 超算 DeepSeek（两个额度包 `0.1` / `10M 体验`）/ MiMo / 豆包工作 / LongCat 可录入并倒计时 | 1h |

**关键**：Phase 1 结束后应用仍是"菜单栏弹窗"形态——**但功能已经能用了**。外壳不影响价值兑现。

### Phase 2 —— 形态改造：独立窗口（约 0.5 天）

> ✅ **前置条件已满足**：§1.4 后台行为已定案（关窗退出）。

| # | 任务 | 验证标准 |
|:---|:---|:---|
| 2.1 | 落地「关窗退出」（§1.4） | `CloseRequested` → `app.exit(0)`；点关闭后 `ps aux \| grep keykeeper` **无残留进程** |
| 2.2 | 窗口配置（§1.1） | 有 Dock 图标、标题栏可拖动、可缩放、⌘W 关闭、⌘Tab 切换 |
| 2.3 | 删除托盘代码 | `main.rs` 无 `TrayIconBuilder` 残留；无编译 warning |
| 2.4 | **移除 notification 插件**（§1.4 连带） | `Cargo.toml` / `package.json` / `main.rs` / `capabilities` 中 notification 依赖全部清除；`cargo check` 与 `pnpm build` 通过 |
| 2.5 | 响应式布局 | 窗口缩放时布局不错乱 |

### Phase 3 —— API 适配器（依赖你提供真实响应 · 估时不确定）

> ✅ **已落地（2026-09-23，提交 `04f611b`）——但换了一条路径**：
> 原计划依赖用户抓包提供真实响应；实际改为**官方文档 / 实证实现 / 官方签名 Demo 驱动**，
> 三个适配器（DeepSeek / 智谱 / 火山）已按各自契约重写，fixture 与 13 个新单测入库。
> **尚欠的只有真实 Key 终验**（批次 C，见 code-review §5）——契约来自文档而非实测，仍存在"文档与线上不一致"的残余风险。

| # | 任务 | 验证标准 | 状态 |
|:---|:---|:---|:---|
| 3.1 | ~~你提供真实响应样本~~ → 存 `src-tauri/tests/fixtures/` | 每平台一份脱敏 JSON | 🟡 三份 fixture 已入库，但来源是官方文档示例 / 实证实现，**非真实 Key 响应** |
| 3.2 | 火山方舟余额接口抓包定位 | 拿到真实端点 + 鉴权方式 | ✅ 官方签名 Demo 定位到 `QueryBalanceAcct` + V4 签名（原 HMAC 四处错误已修）；🟡 billing region 待终验 |
| 3.3 | 按真实字段重写解析（strong-typed struct，吸收 backlog P2-7） | fixture 驱动的解析测试通过 | ✅ 三个适配器均已重写 |
| 3.4 | 验证 MiMo / LongCat / 超算 能否用 API Key 访问 | 可访问 → 升级为 `Api`；仅登录态 → 保持 `Manual` | ⬜ 未做，维持 `Manual` |

> ⚠️ **本阶段预期产出可能是负结果。**
> 按 §3.2，除火山外其余平台的余额查询都疑似登录态。若验证下来全部接不上，**Phase 3 的结论就是"确认不可行"，`Manual` 模式兜底 —— 这不是失败**，而是把不确定性消除了。
> 严格遵循铁律：任何字段路径、端点 URL、签名细节，**实施前必须先拿到真实响应**。Volcano 的 HMAC 签名就是整个前提错误的教训。
> 📌 **该铁律在当前状态下仍未被完全满足**：契约已从"猜测"升级为"有文档/实证支撑"，但**未经真实 Key 终验**。

### Phase 4 —— 收尾（约 0.5 天）

> ✅ **已完成**（提交 `3a963ec`；4.2 / 4.3 已在 Phase 1 提前完成，4.1 的 AddProviderForm 死链由 `04f611b` 补齐）

| # | 任务 | 验证标准 | 状态 |
|:---|:---|:---|:---|
| 4.1 | 接通 opener（P1-1） | 点"充值/控制台"能打开系统浏览器（`window.open` 必须改 `openUrl`） | ✅ |
| 4.2 | 表单保留 Key（P1-2） | 验证失败后 Key 仍在输入框 | ✅ |
| 4.3 | 重置 `reconfigureProvider`（P1-4） | 同一平台可重复触发"重新配置" | ✅ |
| 4.4 | 补 CSP（P2-6） | 应用正常渲染，无 CSP 报错 | ✅ |
| 4.5 | README 重写（P2-4） | 与实现一致 | ✅ |

**总估时**：约 4 个工作日（不含 Phase 3，其估时取决于响应验证结果）。

---

## 6. 待确认问题

> v1 曾声称"调研已解决前三项"——**该声明不准确**。v3 已由用户澄清：「超算 DeepSeek」是独立产品（非 scnet.cn）、「豆包工作」按推荐值采用、§1.4 后台行为已拍板。
> **v4 注**：下列条目中「待用户确认」的现行权威清单已移交 [code-review-2026-09-remediation.md](./code-review-2026-09-remediation.md) §5（Q1–Q6），表格保留原貌 + 当前状态。

| # | 问题 | 类型 | 影响 | 当前状态 |
|:---|:---|:---|:---|:---|
| 1 | **是否保留「黄灯最小化时继续提醒」**（§1.4 备选） | 🟡 影响 Phase 2 | 默认**不实现**。想要就说一声 | 仍开放 |
| 2 | **火山方舟的余额/用量 API 端点** | 🟠 阻塞 Phase 3.2 | 你已确认有 API，需抓包或文档路径 | 🟡 **端点已由官方签名 Demo 定位**（`QueryBalanceAcct`）；仅 billing region 待终验（code-review Q3） |
| 3 | **MiMo / LongCat 能否用 API Key 查余额** | 🟠 阻塞 Phase 3.4 | 需真实 Key 实测 | 仍开放（code-review Q5） |
| 4 | **真实响应样本** | 🟠 阻塞 Phase 3 | 见 §6.1 | 🟡 三份 fixture 已入库，但来自官方文档/实证实现，**真实 Key 响应仍待补** |
| 5 | **这些额度包的真实周期是多久**（§3.1 未决项） | 🟡 影响 §2.6 快捷按钮 | 决定「+30 天」是否合理 | 仍开放 |
| 6 | **超算 DeepSeek / 豆包工作的控制台 URL** | 🟡 影响充值按钮 | 留空不阻塞 Phase 1，可后补 | 仍开放（code-review Q6） |
| 7 | **`CLAUDE.md` / `AGENTS.md` 是否纳入 git** | 🟡 无关紧要 | 目前被忽略（`.gitignore` 显式排除） | 仍开放 |

### 6.1 响应样本的采集方式

**不要把 Key 发给我。** 你自己执行，把输出贴过来即可：

```bash
# 示例：DeepSeek
curl -s -H "Authorization: Bearer <你的Key>" https://api.deepseek.com/user/balance

# 示例：LongCat（OpenAI 兼容格式）
curl -s -H "Authorization: Bearer <你的Key>" https://api.longcat.chat/openai/v1/models
```

对于**只有登录态**的平台（MiMo / 超算 / 豆包工作）：
> 控制台 → DevTools → Network → 过滤 XHR/fetch → 找到余额/用量请求 → 右键 **Copy as cURL**（脱敏后贴给我）

同时请说明该请求用的是 **Cookie 还是 API Key** —— 这决定了它能否接入 Keychain 路径（Volcano 的教训：若是登录态 Cookie，就只能引导用户去控制台）。

---

## 7. 与旧 backlog 的关系

本计划实施后，`requirements-backlog.md` 中以下条目被吸收或作废：

- **被吸收**：P0-3（数据模型 v2）、P0-4（Qoder）、P2-2（阈值统一）、P2-3（Qoder 持久化）、P2-4（Mutex）、P2-5（Key 覆盖）、P2-7（strong-typed）、P2-8（PlatformSpec）、P3-9、P3-10、U-7、U-21
- **被本计划取代**：P0-6（托盘 Template Image）—— **去掉菜单栏后此问题自动消失**
- **被作废**：**P2-1（接入低额度通知）** —— §1.4 决定移除系统通知，该需求不再成立。backlog 中标注为"✅ 已完成"的此项，实际代码将在 Phase 2.4 被删除
- **仍需独立处理**：R-5（SQLite 趋势）、R-6（i18n）、R-7（全局快捷键）、R-10（CI）、R-11（Homebrew）、U-18（.env 导入）等

> 建议：Phase 1 完成后，把本文档的结论合并回 `requirements-backlog.md`，避免两份文档并存。
