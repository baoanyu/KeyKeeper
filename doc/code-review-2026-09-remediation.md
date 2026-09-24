# KeyKeeper 代码评审与修复执行文档

> 版本：**v2（已完成对抗性审查并修订）** · 评审日期：2026-09-23
> 评审范围：全量代码库（Rust 后端 996 行 / Vue 前端 901 行，评审时基线提交 `3a963ec`）
> 评审维度：**业务逻辑 · 性能 · 可行性**
> 阅读顺序：§0 基线 → §1 结论摘要 → §2 问题清单 → §3 修复执行批次 → §4 不修复项 → §5 待确认项 → 附录
> **执行状态（2026-09-23 更新）**：批次 A（工程卫生与交互）、批次 B（模型 v2.1 + 三条自动查询契约）已实施并提交于 `04f611b`，`cargo test` 23 passed / clippy 零警告 / `vue-tsc` / `pnpm build` 全绿。**批次 C（真实 Key 终验）待用户执行**，问题清单见 §5。§1（§1.1 附修复后状态列）/ §2 保留评审时原貌，作为修复依据与证据链。

### 修订记录

| 版本 | 变更 |
|:---|:---|
| v1 | 评审初稿 |
| v2 | 对抗性审查后修订（审查清单见 §6）：<br>· **A-1 降级**：R-2 原表述"端点错误"过强——实测旧端点路由存在（无凭证 401、坏凭证返回 error 信封）；改为"成功响应结构无证据，第三方实证契约在 monitor 端点"<br>· **A-2 澄清**：R-3 region 仅能按 iam 权威向量证伪，billing region 仍由 Q3 确认<br>· **A-3 新增 R-17**：AddProviderForm 切换两个 Manual 平台时表单实例复用、已填数据串台<br>· **A-4 澄清**：R-9 集成测试无法访问 `pub(crate)`，改用内联 `#[cfg(test)]` + `include_str!`<br>· **A-5 澄清**：R-5 进度条的模板条件与宽度绑定<br>· **A-6 措辞**：R-12 为潜在缺陷（当前无触发路径）<br>· **A-7 补充**：R-15 验证必须检查响应体 code（monitor 端点鉴权失败也返回 HTTP 200）<br>· **A-8 排除**：App.vue 内联编辑切换经核查不是问题（v-for keyed 子树会重新挂载） |

---

## 0. 评审基线（事实与口径）

### 0.1 环境与静态检查结果（均可复跑）

| 检查项 | 命令 | 结果 |
|:---|:---|:---|
| Rust 单元测试 | `cd src-tauri && cargo test` | ✅ 10 passed，模块树只编译/运行一遍 |
| Rust Lint | `cd src-tauri && cargo clippy --all-targets -- -D warnings` | ✅ 零警告 |
| 前端类型检查 | `pnpm exec vue-tsc --noEmit` | ✅ 无错误 |
| 前端构建 | `pnpm build` | ✅ 成功（JS 82.4 kB / gzip 32.1 kB） |
| 工作树状态 | `git status --short` | ✅ 干净 |
| Cargo.lock | `git ls-files src-tauri/Cargo.lock` | ✅ 已入库 |
| 文档入库 | `git ls-files doc/` | ✅ 两份文档均在库 |

结论：**工程卫生基线良好**，本次发现的问题集中在"适配器契约正确性"与少量交互/健壮性细节，而非编译或类型层面。

### 0.2 证据分级约定

- **【官方】**：平台官方 API 文档原文
- **【实证】**：可运行的第三方实现（源码可读、被实际使用），但非官方文档
- **【代码】**：本仓库源码直接可证
- **【推导】**：由上述证据推理得出，需终验
- 所有"字段路径 / 端点 / 签名"结论，标注证据级别；未经真实 Key 终验的，明确写出。

### 0.3 评审方法

1. 先读项目指南（`CLAUDE.md`、`AGENTS.md`）与唯一事实来源（`doc/refactor-plan-v2.md`、`requirements-backlog.md`）；
2. 逐文件精读全部 Rust / Vue 源码，追踪 `入口 → 命令 → 适配器 → 展示` 完整链路；
3. 对每个适配器，用官方文档 / 可运行第三方实现核对端点、鉴权、字段；
4. 对火山签名，用官方签名 Demo 的已知 AK/SK 与期望签名做密码学复现；
5. 运行全部静态检查建立事实基线。

---

## 1. 评审结论摘要

### 1.1 总体判断

> 下表是**评审时（提交 `3a963ec`）**的判断。修复后状态见末列。

| 维度 | 评审时评级 | 说明 | 修复后（2026-09-23） |
|:---|:---|:---|:---|
| 业务逻辑（Manual 路径） | 🟢 良好 | 到期录入、CRUD、持久化、倒计时、排序、错误传播链路完整，可立即替代手写记录 | 🟢 不变，另补 Manual 写入串行化 + 失败回滚（R-6） |
| 业务逻辑（Api 自动查询） | 🔴 不可用 | **三个适配器（DeepSeek / 智谱 / 火山）在当前代码下均必然失败**，且失败点已逐一用证据定位（§2 P0） | 🟡 **已按各自契约重写**（批次 B）；契约来自官方文档 / 实证实现 / 官方签名 Demo，**未经真实 Key 终验** |
| 性能 | 🟢 总体良好，🟡 有 3 处小瑕疵 | 数据量小（<20 平台），无真实瓶颈；仅有重复反序列化、全量验证、刷新交叉三类可低成本消除的浪费 | 🟢 重复反序列化已消除（R-10）、刷新交叉避让已验证（R-16）；🟡 **全量验证（R-15）仍在批次 D，未实施** |
| 可行性 | 🟡 风险显著下降 | 本次评审把三条自动查询路径从"猜字段"升级为"有文档/实证契约 + 可确定性测试"，**唯一剩余阻塞是用户用真实 Key 做一次终验**；方舟套餐到期预计无公开 API，Manual 兜底 | 🟡 **唯一剩余阻塞不变**：批次 C 真实 Key 终验（§5） |

### 1.2 与改造计划（refactor-plan-v2.md）的关系

- 计划 Phase 0/1/2/4 已落地，结论与代码基本一致；
- 本次新发现 / 修正：
  1. **Phase 4.1「opener 外链修复」不完整**——AddProviderForm 的"如何获取 Key"链接仍是死链（P1-1 同类，且被 `@click.stop` 主动阻断）；
  2. **DeepSeek 真实响应结构经官方文档确认**（顶层 `balance_infos[]`，无 `data` 包裹层），比计划中"字段存疑"更明确；
  3. **智谱真实用量端点经第三方实现定位**（`/api/monitor/usage/quota/limit`）；对抗性审查实测：旧端点 `/api/paas/v4/balance` 路由存在（无凭证 401），但其成功响应结构无任何文档/实证支持，当前代码的 `data.remaining_tokens` 字段同样未被证实；
  4. **火山签名错误点经密码学复现精确定位**（`VOLC` 前缀、region、URI、Action/Version 四处），且找到可立即实现的账户余额接口 `QueryBalanceAcct`；
  5. `rust-toolchain.toml` 在 Phase 0 被**过度删除**（两份都没了），与 `CLAUDE.md` 描述不符，损害可复现构建。

### 1.3 问题统计

| 严重度 | 数量 | 编号 |
|:---|:---|:---|
| P0（核心功能必然失败） | 3 | R-1、R-2、R-3 |
| P1（核心流程受阻 / 功能失效） | 2 | R-4、R-5 |
| P2（健壮性 / 一致性 / 可复现性） | 5 | R-6、R-7、R-8、R-9、R-10 |
| P3（体验 / 清理 / 性能小项） | 7 | R-11、R-12、R-13、R-14、R-15、R-16、R-17 |

---

## 2. 问题清单

> 每条含：位置 · 现象 · 证据 · 修复步骤 · 验证方式 · 回归风险。

### 🔴 P0

#### R-1　DeepSeek 适配器字段路径与官方文档不符，任何有效 Key 都查询失败

- **位置**：`src-tauri/src/adapters/deepseek.rs:36-52`
- **现象**：代码解析 `json["data"]["balance"]`；官方响应中**没有 `data` 包裹层**，余额在顶层 `balance_infos[].total_balance`（字符串）。`balance` 恒为 `None` → 返回错误"响应缺少 data.balance 字段"。
- **证据**：【官方】DeepSeek API 文档《Get User Balance》：
  ```json
  {
    "is_available": true,
    "balance_infos": [
      { "currency": "CNY", "total_balance": "110.00",
        "granted_balance": "10.00", "topped_up_balance": "100.00" }
    ]
  }
  ```
  端点 `GET https://api.deepseek.com/user/balance`，Bearer 鉴权——端点本身正确，仅解析结构错误。
- **修复步骤**：
  1. 定义 strong-typed 结构（吸收 backlog P2-7）：
     ```rust
     #[derive(serde::Deserialize)]
     struct BalanceResponse {
         is_available: bool,
         balance_infos: Vec<BalanceInfo>,
     }
     #[derive(serde::Deserialize)]
     struct BalanceInfo {
         currency: String,
         total_balance: String,
         granted_balance: String,
         topped_up_balance: String,
     }
     ```
  2. `is_available == false` → 返回 `Err`（附"账户不可用"）；
  3. 每个 `balance_info` 生成一个 `Entitlement`：`label="余额"`，`unit` 按 currency 映射（CNY→`QuotaUnit::CNY`，USD→CNY 并在 note 注明），`remaining = total_balance.parse()`；解析失败按错误返回，**不得静默取 0**；
  4. `note` 可写 `format!("赠金 {} / 充值 {}", granted_balance, topped_up_balance)`；
  5. 用官方文档示例 JSON 建 fixture `src-tauri/tests/fixtures/deepseek_balance.json`，补解析测试。
- **验证**：`cargo test`（含新解析测试）通过 → 用户真实 Key 终验（§5 Q1）。
- **回归风险**：低。仅改动适配器内部解析；输出仍为单 Entitlement 余额形态，前端无需改动。

#### R-2　智谱适配器字段路径无任何证据支撑，自动查询未被证明可工作

- **位置**：`src-tauri/src/adapters/zhipu.rs:21-46`
- **现象**：代码请求 `/api/paas/v4/balance` 并解析 `data.remaining_tokens`。对抗性审查实测（2026-09-23，curl）：
  - 旧端点**路由存在**：无凭证返回 `401`；坏凭证返回 `{"error":{"code":"401","message":"令牌已过期或验证不正确"}}`；
  - 但旧端点的**成功响应结构在官方文档与第三方生态中均查无实据**，`data.remaining_tokens` 没有任何来源支撑；
  - 可运行的第三方实现一致表明：Coding Plan 的用量数据在另一个端点 `/api/monitor/usage/quota/limit`，返回**各配额窗口的已用百分比 + 重置时间**。
  因此当前代码在真实 Key 下能否成功完全未知，按实证契约重写是风险最低的路径。
- **证据**：【实证】npm 包 `dsh-usage-glm-cn@1.1.5`（周下载量数百、源码可读）`lib/logic.js`；【代码】上述 curl 实测：
  - `GET https://open.bigmodel.cn/api/monitor/usage/quota/limit`（全球端点 `https://api.z.ai/api/monitor/usage/quota/limit`）
  - Header：`Authorization: Bearer <Coding Plan Key>`
  - 注意该端点鉴权失败也返回 **HTTP 200**（体内 `code=401/1001`），必须以**响应体 code** 判定成败，不能只看 HTTP 状态；
  - 成功响应：
    ```json
    { "code": 200,
      "data": { "limits": [
        { "type": "TOKENS_LIMIT", "unit": 3, "percentage": 42,
          "nextResetTime": "2026-08-18T10:00:00Z" },
        { "type": "TOKENS_LIMIT", "unit": 6, "percentage": 15,
          "nextResetTime": "2026-08-24T00:00:00Z" },
        { "type": "TIME_LIMIT", "unit": 5, "percentage": 0,
          "nextResetTime": "..." }
      ] } }
    ```
  - 映射：`unit=3` → 5 小时滚动窗口；`unit=6` → 周窗口；`unit=5`（TIME_LIMIT）→ MCP 月度；`percentage` 为**已用**百分比（0–100）。
- **修复步骤**：
  1. 端点改为上述 URL，Bearer 鉴权保持；
  2. strong-typed 结构 `QuotaLimitResponse { code: i64, data: Option<QuotaLimitData>, msg: Option<String> }`、`QuotaLimitData { limits: Vec<QuotaLimit> }`、`QuotaLimit { r#type: String, unit: i64, percentage: f64, next_reset_time: String }`（serde rename `nextResetTime → next_reset_time`）；
  3. **先判体 `code != 200` → Err**（HTTP 状态可能是 200）；
  4. 每个 limit 生成 Entitlement，**依赖 R-5 引入的 `used_percent` 字段**：
     - `label`：5小时滚动 / 周额度 / MCP月度（未知 unit 用 `limitLabel` 兜底）
     - `used_percent = Some(percentage)`，`expires_at = Some(nextResetTime 解析为 Unix 秒)`（语义为"重置时刻"，卡片在 used_percent 存在时文案改为"X 后重置"）
     - TOKENS_LIMIT → `unit=Tokens`；TIME_LIMIT → `unit=Unknown`
  5. fixture `zhipu_quota_limit.json` + 解析测试。
- **验证**：`cargo test` 通过 → 用户真实 Coding Plan Key 终验（§5 Q2）。
- **回归风险**：中。输出从"假设的剩余 token"变为"已用百分比 + 重置"，需同步卡片渲染（R-5）。
- **备选 1（若真实 Key 证明旧端点可用）**：保留 `/api/paas/v4/balance`，按其真实成功响应重写解析——此时仅改解析，成本更低。**这正是 Q2 要先 curl 一次的原因。**
- **备选 2（若不接受模型改动）**：用 `total=Some(100), remaining=Some(100-percentage)` 复用进度条，`expires_at` 放重置时间；缺点是主文案显示"今天到期/还剩 0 天"，语义混淆。不推荐，仅记录。

#### R-3　火山适配器签名与端点四处错误，请求必然被拒

- **位置**：`src-tauri/src/adapters/volcano.rs`（全文件）
- **现象 / 证据**：
  1. **签名派生前缀错误**：代码用 `HMAC("VOLC"+SK, date)`；官方签名 Demo 的派生链为 `HMAC(HMAC(HMAC(HMAC(SK, date), region), service), "request")`，**无 VOLC 前缀**。【官方 + 密码学复现】用 Demo 提供的 SK 与日期，无前缀派生得到签名密钥 `abee62e5…79826`、最终签名 `e31c4558…dde93`，与官方期望值**逐字节一致**；VOLC 前缀变体得到 `e6cb055c…af425`，不符。
  2. **region 取值与权威签名向量不一致**：代码用 `cn-north-1`；官方 Demo 向量（iam，含期望签名）为 `cn-beijing`。注意：这只能证伪"签名器在 iam 向量下的正确性"；**billing 调用的实际 region 未被单独证实**，需经 API Explorer 确认（见 Q3），不应在代码里静默拍板。
  3. **CanonicalURI 错误**：代码 `/api/v3/quota/balance`；火山 OpenAPI 的 URI 绝大多数为 `/`，动作由查询参数 `Action/Version` 指定。
  4. **缺少 Action/Version**：OpenAPI 网关要求 `?Action=...&Version=...` 并参与签名。
- **修复步骤（分两层）**：

  **(a) 通用签名器（可立即做，确定性测试）**
  1. 抽出独立签名函数，输入 `(method, uri, query BTreeMap<&str,&str>, host, region, service, ak, sk, timestamp)`，输出 `Authorization` 头（同时给 `X-Date`；GET 空体 payload hash 固定 `e3b0c4…b855`）；
  2. 派生链严格按官方 Demo（无前缀）；query 按 RFC3986 编码并按 key ASCII 排序；
  3. 用官方 Demo 向量写测试（已知 AK/SK、`X-Date=20240619T071306Z`、`Action=ListUsers/Version=2018-01-01/Limit=10/Offset=0`、host `iam.volcengineapi.com`、region `cn-beijing`、service `iam`），断言签名 == `e31c4558…dde93`。此测试**无需真实凭证**。

  **(b) 账户余额接口（有官方文档，可立即实现）**
  1. `GET https://open.volcengineapi.com/?Action=QueryBalanceAcct&Version=2022-01-01`，service `billing`；
  2. 响应（【官方】费用中心文档）：
     ```json
     { "ResponseMetadata": { "RequestId": "...", "Action": "QueryBalanceAcct",
       "Version": "2022-01-01", "Service": "billing" },
       "Result": { "AccountID": 210000000,
         "ArrearsBalance": "1.01", "AvailableBalance": "77.01",
         "CashBalance": "83.01", "CreditLimit": "0.01", "FreezeAmount": "5.01" } }
     ```
  3. strong-typed 解析 `Result`，生成 Entitlement：`label="账户余额"`，`unit=CNY`，`remaining=AvailableBalance.parse()`；`note` 写 `现金 {CashBalance} / 冻结 {FreezeAmount} / 欠费 {ArrearsBalance}`；
  4. fixture + 解析测试。
- **待终验项**：
  - **region 取值**：官方另一份示例（67269 的 Python 代码）对 iam 用了 `cn-north-1`，与权威 Demo 向量矛盾。签名器以 Demo 向量为准；billing 调用的实际 region 请在 **API Explorer 选 QueryBalanceAcct → 签名工具** 复制生成的 curl 确认（§5 Q3）。若 API Explorer 显示 cn-north-1，仅改常量一处。
  - 该接口给的是**账号现金余额**；**方舟套餐到期**预计只有登录态控制台接口，维持 Manual / 待抓包（§5 Q4）。
- **验证**：`cargo test`（签名向量测试 + 解析测试）通过 → 用户真实 AK/SK 终验。
- **回归风险**：中。火山 Key 录入格式仍为 `AccessKey:SecretKey`（`key_pattern` 为空、不校验，保持现状）；输出 Entitlement 形态与原设计一致。
- **失败模式自检（防止测试空转）**：一个仍可能让"签名测试通过、真实请求失败"的错误实现——签名器内部把 query 编码/排序做错，但测试向量恰好只用固定字符串拼接。因此测试必须走**完整 query 构造路径**（传入 BTreeMap 让签名器自己编码排序），而非直接喂最终字符串。

### 🟠 P1

#### R-4　AddProviderForm「如何获取 Key」外链是死链（P1-1 同类，Phase 4.1 遗漏）

- **位置**：`src/components/AddProviderForm.vue:126-134`
- **现象**：点击"↳ 如何获取？… API Key"无任何反应。
- **证据**：【代码 + 依赖源码】
  1. 该 `<a target="_blank">` 绑定了 `@click.stop`，调用 `stopPropagation()`；
  2. tauri-plugin-opener（本机 2.5.5）的链接拦截脚本（`src/init-iife.js`）注册方式为 `window.addEventListener("click", …)`，即 **window 上的冒泡阶段**监听；事件在锚点处被 stop，无法到达 window → 插件不触发；
  3. WKWebView 对 `target="_blank"` 默认不创建新窗口、不回退系统浏览器（refactor-plan P1-1 已逐层验证）。
- **修复步骤**：删除该锚点上的 `@click.stop`（保留 `target="_blank" rel="noopener noreferrer"`，opener 自动接管）；或显式改为 `@click.prevent="openUrl(spec.key_docs_url)"`。
- **验证**：`pnpm tauri dev` → 选择任一 Api 平台 → 点击链接 → 系统浏览器打开对应 `key_docs_url`。
- **回归风险**：极低。

#### R-5　数据模型缺少"已用百分比"表达，智谱/方舟类用量接口无法正确呈现

- **位置**：`src-tauri/src/models.rs:34-44`、`src/types.ts:9-18`、`src/components/QuotaCard.vue`
- **现象**：智谱（R-2）与未来方舟用量返回"已用百分比 + 重置时间"，当前 `Entitlement` 只能用余额字段或 note 间接表达，导致卡片主文案语义错误（"到期" vs "重置"）。
- **证据**：【实证】R-2 所引第三方契约；【推导】方舟用量同构。
- **修复步骤（模型 v2.1，最小扩展）**：
  1. `Entitlement` 增加 `#[serde(default)] pub used_percent: Option<f64>`（0–100，已用百分比）；TS 接口同步 `used_percent: number | null`；
  2. builder 增加 `with_used_percent(self, pct: f64)`；
  3. QuotaCard 渲染优先级调整：
     - `used_percent != null`：主文案 `已用 X%`；进度条渲染条件改为 `v-if="hasTotal(e) || e.used_percent != null"`；百分比型宽度直接绑定 `:style="{ width: used_percent + '%' }"`（≥80 橙 / ≥95 红，否则蓝）；若 `expires_at` 存在，副文案 `Xh Ym 后重置 · {label}`；
     - 其余分支（total/remaining、纯到期、纯余额）保持现状；
  4. 为旧数据兼容：`#[serde(default)]` 保证历史 Manual 数据可反序列化。
- **验证**：`cargo test`（含 serde 往返）、`vue-tsc`、手测 R-2 卡片渲染。
- **回归风险**：中。需前后端同时发布；因字段 `#[serde(default)]` 且为 Option，对既有数据与 Manual 录入无破坏。

### 🟡 P2

#### R-6　手动录入的读-改-写无串行化，并发保存可能丢条目；保存失败时内存态不回滚

- **位置**：`src-tauri/src/commands.rs:53-58、148-186`
- **现象**：
  1. `save_manual_platform` 与 Manual 分支 `delete_platform` 都是"load 全量 map → 改自己的条目 → 写全量 map"，无锁；两个并发调用（如快速连续编辑不同平台）后写覆盖先写，**先写的条目丢失**；
  2. `store.set()` 先改内存、`store.save()` 后落盘；若 save 失败，命令报错但**内存已是新值**——随后 get_all 显示"已保存"，重启后回退，形成二次困惑。
- **证据**：【代码】上述函数；`tauri-plugin-store` set/save 语义。
- **修复步骤**：
  1. `AppState` 增加 `pub manual_write_lock: tokio::sync::Mutex<()>`；
  2. `save_manual_platform` 与 Manual 分支 `delete_platform` 入口 `let _g = state.manual_write_lock.lock().await;`（注意 delete_platform 需增加 `state: State<'_, AppState>` 参数）；
  3. save 失败时回滚内存：在 set 前 `let prev = store.get(MANUAL_PLATFORMS_KEY)`；save 返回 Err 后用 `store.set(.., prev.unwrap_or(Null))` 恢复，再把原错误返回前端；
  4. 写一个并发测试：spawn 两个不同 id 的保存逻辑（提取可测的纯函数或用注入 store 的接缝），断言两条目都在。若接缝成本过高，至少保留锁并在文档注明未自动化部分。
- **验证**：`cargo test`；手测连续快速编辑两个平台均生效。
- **回归风险**：低。锁仅在命令内持有，不跨命令；不与其他锁嵌套，无死锁面。

#### R-7　get_all_platforms 将所有 Keychain 读取错误一律当作"未配置"静默跳过

- **位置**：`src-tauri/src/commands.rs:91-94`
- **现象**：`keystore::get_key()` 返回任何 Err 都 `continue`，平台直接消失。NoEntry（未配置）应当跳过；但 Keychain 后端错误（ACL、损坏、访问被拒）也被吞掉，用户看到的是"平台莫名没了"且无任何线索。
- **证据**：【代码】。
- **修复步骤**：
  1. `keystore` 增加区分函数或在命令处按 `keyring::Error::NoEntry` 匹配：NoEntry → `continue`；
  2. 其他错误 → `log::warn!` 并向结果追加 `PlatformStatus::failed(spec, &format!("Keychain 读取失败: {}", e))`，使用户至少能看到该平台与重试/重新配置入口。
- **验证**：`cargo test`；临时制造非 NoEntry 错误（如 mock 接缝）观察 failed 卡片；无法 mock 时以代码审查 + 日志验证并注明。
- **回归风险**：低。

#### R-8　rust-toolchain.toml 被过度删除，可复现构建失去工具链锚点

- **位置**：仓库根（应有）；现状两份 `rust-toolchain.toml` 均不存在
- **现象**：refactor-plan P0-2 要求"删除 src-tauri 那份、合并进根目录那份"，实际根目录那份也没了。`CLAUDE.md:129` 仍声明根目录有该文件。新环境克隆后只能用本机默认工具链，universal2 所需 target 无声明。
- **证据**：【代码】`ls rust-toolchain.toml src-tauri/rust-toolchain.toml` 均 No such file；本机已手动装 `aarch64-apple-darwin`、`x86_64-apple-darwin` 两个 target。
- **修复步骤**：在仓库根新建 `rust-toolchain.toml`：
  ```toml
  [toolchain]
  channel = "stable"
  components = ["rustfmt", "clippy"]
  targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
  ```
  （与 CLAUDE.md "channel=stable" 的口径一致；不要 pin 死 1.97，避免重蹈 P0-2 白下载工具链的覆辙。）
- **验证**：在根与 src-tauri 下分别 `rustc --version` 一致且不触发额外下载；`cargo check` 通过。
- **回归风险**：无。

#### R-9　适配器解析零测试覆盖——本项目最大风险敞口仍未闭合

- **位置**：`src-tauri/tests/`（不存在）
- **现象**：10 个单测全部围绕模型/序列化/空调度，三个适配器的端点与字段解析零覆盖；而字段错误正是本项目连续两代价差的根源。
- **证据**：【代码】。
- **修复步骤**：随 R-1/R-2/R-3 一并落地：
  1. 建 `src-tauri/tests/fixtures/`，放入 deepseek / zhipu / volcano(QueryBalanceAcct) 三份（来自官方文档与实证，脱敏）；
  2. 解析逻辑提取为独立纯函数（`pub(crate) fn parse_xxx(serde_json::Value) -> Result<Vec<Entitlement>>`），适配器只负责 HTTP；
  3. **测试可见性注意（A-4）**：`src-tauri/tests/` 下的集成测试是外部 crate，**无法调用 `pub(crate)` 函数**。两种落地方式任选：① 在各适配器模块内用 `#[cfg(test)] mod tests` 内联测试，通过 `include_str!("../tests/fixtures/xxx.json")` 加载 fixture（推荐，私有函数可直接测）；② 把解析函数改为 `pub` 并在 `adapters/mod.rs` 导出。火山签名向量同理。
- **验证**：`cargo test` 可见并通过新增测试。
- **回归风险**：无（纯新增测试）。

#### R-10　【性能】get_all_platforms 在循环内对每个 Manual 平台重复全量反序列化 store

- **位置**：`src-tauri/src/commands.rs:106-119`（`load_manual_map(&app)` 位于循环体内）
- **现象**：当前 5 个 Manual 平台 → 每次刷新执行 5 次 `load_manual_map`，每次对整份 map 做 `serde_json::from_value` 全量反序列化（store 实例本身有缓存，重复成本主要在反序列化而非读盘）。自动刷新 5 分钟一次，属持续浪费。
- **证据**：【代码】。
- **修复步骤**：在循环**之前**调用一次 `let manual_map = load_manual_map(&app);`，循环内改为查 `manual_map.get(spec.id)`。
- **验证**：`cargo test`；刷新结果与改动前一致（Manual 条目数量/字段不变）。
- **回归风险**：极低。

### ⚪ P3

#### R-11　重新配置（reconfigure）模式下没有取消出口

- **位置**：`src/components/AddProviderForm.vue:144-151`、`src/App.vue:204-213`
- **现象**：进入重新配置后，平台下拉被禁用、"取消"按钮被 `v-if="!preselected"` 隐藏；验证失败时用户被困在该表单，无法收起或退出。
- **修复**：在 preselected 状态下显示一个"取消"按钮 → 新增 emit `cancelReconfigure` → App 侧 `reconfigure.value = null`（并可 `formKey++` 复位）。
- **验证**：点重新配置 → 取消 → 表单恢复默认新增态。
- **回归风险**：极低。

#### R-12　进度条颜色不反映红色（≤3 天）到期（潜在缺陷，当前无触发路径）

- **位置**：`src/components/QuotaCard.vue:158-164`
- **现象**：进度条 class 仅区分 `entitlementBadge(e) ? 橙 : 蓝`；红色到期（≤3 天）时进度条仍为橙色。**注意（A-6）**：当前没有任何数据源会同时产生红色标记与 `total`（Manual 录入无 total、Api 余额 total 为 None），所以这是面向 R-5/R-15 之后的潜在缺陷，不是现网可见 bug，故定 P3。
- **修复**：`badge==='red' → bg-red-500`、`'orange' → bg-orange-500`、默认蓝。
- **验证**：构造 ≤3 天且有 total 的 Entitlement（当前无自动来源，可临时在 fixture/开发数据中验证）。
- **回归风险**：无。

#### R-13　错误响应脱敏不完整：Bearer 后的凭证与智谱格式 Key 会漏出

- **位置**：`src-tauri/src/adapters/mod.rs:14-31`
- **现象**：按空白分词后，仅把以 "Bearer" 开头的那个词替换为 `***`，**其后的凭证词原样保留**；智谱格式（`32hex.secret`，无 `sk-`）也不匹配现有规则。
- **证据**：【代码】。
- **修复**：分词前先用正则整体替换 `Bearer\s+[A-Za-z0-9._\-]+` → `Bearer ***`；再对匹配智谱 Key 形态（`^[a-f0-9]{32}\.[A-Za-z0-9]+$`）的词替换；保留现有 `sk-` 规则。
- **验证**：补一个针对构造样本的单测（含 Bearer token、智谱 Key、普通文本），断言输出不含凭证原文。
- **说明**：错误体仅在本机界面展示，属纵深防御，故定 P3。
- **回归风险**：极低。

#### R-14　死代码 / 残留 feature：`keystore::has_key` 未使用、tauri 的 `tray-icon` feature 残留

- **位置**：`src-tauri/src/keystore.rs:28-35`；`src-tauri/Cargo.toml`（`features=["macos-private-api","tray-icon"]`）
- **现象**：Phase 2 删除托盘后 tray-icon feature 已无使用方；`has_key` 无调用方。
- **修复**：移除 `tray-icon` feature；删除 `has_key`（若 R-7 想复用 NoEntry 判断，可保留并注明）。
- **验证**：`cargo clippy --all-targets -- -D warnings`、`cargo test` 通过。
- **回归风险**：极低。

#### R-15　【性能】addApi 验证走全量 get_all_platforms，耗时被其他平台拖累

- **位置**：`src/App.vue:86-104`
- **现象**：保存 Key 后调用 `get_all_platforms` 查询**全部**平台来验证单个新 Key；若其他平台接口慢/超时，本次验证最多多等 10s，且产生与验证无关的请求。
- **修复（可选增强）**：新增命令 `validate_api_key(id)`，后端直接构造该平台 fetcher 单次抓取并返回 `Result<Vec<Entitlement>, String>`；前端验证改用它，成功后再 refresh。同时简化前端回滚逻辑。**实现注意（A-7）**：智谱端点鉴权失败也返回 HTTP 200，验证逻辑必须检查响应体 `code`，不能只看传输层状态。
- **验证**：类型检查 + 手测新增/换 Key/失败回滚三路径。
- **回归风险**：中（动到验证链路），可放到 P0 修复之后独立批次。

#### R-16　【性能】自动刷新与 Key 验证可并发交叉写入 platforms

- **位置**：`src/App.vue:58-70、73-116、177-179`
- **现象**：`refresh()` 的防抖只看 `loading`，而 `addApi` 不设置 loading；验证期间若触发 5 分钟自动刷新，两者并发且都写 `platforms.value`，结果顺序不确定（内容均为全量列表，影响轻微）。
- **修复**：`refresh()` 开头增加 `if (verifying.value) return;`，或验证期间的自动刷新延后。
- **验证**：手测/代码审查。
- **回归风险**：极低。

#### R-17　AddProviderForm 切换两个 Manual 平台时录入表单数据串台

- **位置**：`src/components/AddProviderForm.vue:159-162`
- **现象**：Manual 平台共用同一个 `<ManualEntryForm>` 位置（`v-else` 分支，无 key）。用户在平台 A 下填写了额度包名/到期日，随后把下拉切到另一个 Manual 平台 B —— 组件实例被复用、行数据原样保留，此时点保存会把 A 的到期数据提交到 B 名下。
- **证据**：【代码】模板中 ManualEntryForm 未绑定 key；Vue 对同位置同组件按实例复用。
  （对照：App.vue 内联编辑切换不同平台时，v-for keyed 子树会整体重新挂载，**不存在**此问题——A-8 已核查排除。）
- **修复**：给该 ManualEntryForm 加 `:key="selectedId"`，切换平台即重置为空行；顺带也可在 Api/Manual 类型切换时保证状态干净。
- **验证**：`pnpm tauri dev` → 选 longcat 填数据 → 切到小米 MiMo → 表单应为空行。
- **回归风险**：极低。

---

## 3. 修复执行批次（按依赖与风险排序）

> 原则：**先做无外部依赖、可确定性验证的；真实 Key 终验放最后**。每批次结束必须跑对应验证命令，全绿才进入下一批。

### 批次 A —— 工程卫生与交互修复（无外部依赖 · 预估 0.5 人日）

> ✅ **已实施**（提交 `04f611b`）

| 任务 | 关联 |
|:---|:---|
| 恢复根 `rust-toolchain.toml` | R-8 |
| 删除死代码 `has_key`、移除 `tray-icon` feature | R-14 |
| 修复 AddProviderForm 死链（去 `@click.stop`） | R-4 |
| reconfigure 增加取消出口 | R-11 |
| 进度条红色映射 | R-12 |
| 错误脱敏增强 + 单测 | R-13 |
| Manual store 读取移出循环 | R-10 |
| 自动刷新避让验证 | R-16 |
| Manual 表单切换平台加 key | R-17 |

**批次验证（可整段复跑）**：
```bash
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
pnpm exec vue-tsc --noEmit && pnpm build
```

### 批次 B —— 模型 v2.1 与三条自动查询契约（文档/实证驱动 · 预估 1 人日）

> ✅ **已实施**（提交 `04f611b`）——B1~B7 全部落地，新增 13 个单测（含火山签名向量），累计 23 个全绿

| 顺序 | 任务 | 关联 |
|:---|:---|:---|
| B1 | `Entitlement` 增加 `used_percent` + 卡片渲染 + serde 兼容 | R-5 |
| B2 | DeepSeek 按官方契约重写解析 + fixture/测试 | R-1、R-9 |
| B3 | 智谱按实证契约重写 + fixture/测试 | R-2、R-9 |
| B4 | 火山通用签名器 + 官方 Demo 向量测试 | R-3(a)、R-9 |
| B5 | 火山 QueryBalanceAcct 余额实现 + fixture/测试 | R-3(b)、R-9 |
| B6 | Manual 写入串行化 + 失败回滚内存 | R-6 |
| B7 | Keychain 错误区分与 failed 兜底 | R-7 |

**批次验证**：
```bash
cd src-tauri && cargo test    # 预期新增：签名向量、3 个适配器解析、脱敏、并发写入等测试
cd src-tauri && cargo clippy --all-targets -- -D warnings
pnpm exec vue-tsc --noEmit && pnpm build
```

### 批次 C —— 真实 Key 终验（需用户配合 · 预估 0.5 人日）

> ⬜ **待执行**——这是当前唯一的剩余阻塞项

按 §5 的问题清单逐项执行 curl / API Explorer，留存输出（脱敏）到 fixtures 或评审记录：
1. DeepSeek 余额返回结构确认；
2. 智谱 quota/limit 在真实 Coding Plan Key 下返回确认；
3. 火山 QueryBalanceAcct 的 region（API Explorer 签名 curl）与返回确认；
4. 方舟套餐到期是否有可 Keychain 化的 API（抓包），否则确认 Manual 兜底。

**终验通过标准**：`pnpm tauri dev` 中，配置了凭证的三个平台卡片均显示真实数据、无 error；未配置/Manual 行为不变；关窗后无残留进程。

### 批次 D —— 可选增强（独立排期）

- `validate_api_key` 单平台验证命令（R-15）；
- backlog 中 R-10 CI、R-11 Homebrew、R-5 SQLite 趋势等，仍按原 backlog 推进。

**总估时**：A+B+C ≈ 2 人日（不含用户抓包等待与批次 D）。

---

## 4. 不修复项（含理由）

| 项 | 理由 / 处置 |
|:---|:---|
| `entitlements.plist` 的 `com.apple.security.device.keychain` 非标准 key | 与 refactor-plan P2-7 一致：非沙盒应用本就不需要，**上架签名时再验证**，当前不动 |
| 系统通知 / 后台常驻 | §1.4 已决策"关窗退出、通知移除"，不重新引入；临期靠应用内标记 |
| 「黄灯最小化继续提醒」备选 | 计划 §6 Q1：默认不实现，用户需要再提 |
| MiMo / LongCat 升级 Api | 现有证据其余额/用量仅登录态可查，维持 Manual；真实 Key 实测后再定（§5 Q5） |
| 引入 SQLite | 仅趋势功能需要，当前 Manual 数据量小，不提前引入 |
| 全局快捷键 / i18n / 暗色模式 | 属 backlog 独立条目，不混入本次修复 |

> 产品可行性提醒（仅记录，不改变决策）：关窗退出后应用不会主动提醒，"别忘"依赖用户主动打开；这是已拍板取舍，本文不再展开。

---

## 5. 待用户确认 / 提供的问题清单

| # | 问题 | 阻塞 | 建议动作（命令可复跑） |
|:---|:---|:---|:---|
| Q1 | DeepSeek 真实余额响应是否与官方文档示例一致 | R-1 终验 | `curl -s -H "Authorization: Bearer <Key>" https://api.deepseek.com/user/balance`（自行执行，勿发 Key） |
| Q2 | 智谱用量：monitor 端点在真实 Coding Plan Key 下是否可用；旧端点 `/api/paas/v4/balance` 的成功结构是什么 | R-2 终验 | 两条都 curl 一次：`curl -s -H "Authorization: Bearer <Key>" https://open.bigmodel.cn/api/monitor/usage/quota/limit` 与 `…/api/paas/v4/balance`，按真实结构二选一落地 |
| Q3 | 火山 billing 调用的 region（cn-beijing / cn-north-1） | R-3 终验 | API Explorer → QueryBalanceAcct → 「签名工具」复制 curl 对照；或用真实 AK/SK 直接跑生成的请求 |
| Q4 | 方舟**套餐到期**是否有 API Key 可鉴权的接口 | R-3 范围 | 控制台套餐页 DevTools → Network → Copy as cURL（脱敏），注明 Cookie 还是 AK/SK |
| Q5 | MiMo / LongCat 能否用 API Key 查用量 | 未来升级 | 真实 Key 实测；仅登录态则保持 Manual |
| Q6 | 超算 DeepSeek 控制台 URL | 充值按钮 | 有则补 `PLATFORM_SPECS`，留空不阻塞 |

---

## 6. 对抗性审查记录（元审查）

> 本节是对 v1 文档的独立攻击记录，按"主张是否成立 / 是否有遗漏 / 方案是否可执行"三路进行。审查日期 2026-09-23。

### 6.1 主张核验结果

| 攻击项 | 方法 | 结论 |
|:---|:---|:---|
| R-1「无 data 包裹层」 | 官方文档 + 文档镜像交叉 | ✅ 主张成立 |
| R-2「端点错误」 | 无凭证 / 坏凭证实测两个端点 | ❌ **原主张过强，已降级（A-1）**：旧端点路由存在；改为"成功结构无证据" |
| R-3「VOLC 前缀错误」 | 用官方 Demo 期望签名做密码学复现 | ✅ 主张成立（无前缀变体逐字节命中） |
| R-3「region 错误」 | 比对两份官方页面 | ⚠️ **已澄清（A-2）**：仅 iam 向量可证伪；billing region 待 Q3 |
| R-4「死链」 | 读 opener 2.5.5 注入脚本原文 | ✅ 主张成立（window 冒泡监听 + @click.stop 阻断） |
| R-10「重复 5 次」 | 计数 Manual 平台 = 5；核对 store 缓存语义 | ✅ 成立，且已把"重复读盘"修正为"重复反序列化" |
| R-13「Bearer 凭证漏出」 | 手工推演分词结果 | ✅ 成立 |

### 6.2 遗漏排查结果

| 排查路径 | 结果 |
|:---|:---|
| 补读 main.ts / style.css / index.html / public | 无新问题（favicon 资源存在） |
| App.vue 内联编辑切换平台是否串数据 | **不是问题（A-8）**：v-for keyed 子树整体重挂载 |
| AddProviderForm 内 ManualEntryForm 切换平台 | ❗ **新问题 R-17（A-3）**：同位置实例复用导致数据串台 |
| 火山失败模式自检（测试空转风险） | 已在 R-3 补"必须走完整 query 构造路径"的约束 |

### 6.3 方案可执行性核验

| 检查项 | 结论 / 修订 |
|:---|:---|
| R-9 集成测试访问 `pub(crate)` | 不可行 → 已改为内联 `#[cfg(test)]` + `include_str!`，或 pub 导出（A-4） |
| R-5 进度条模板/宽度 | 已补明确的 `v-if` 条件与 `:style` 绑定（A-5） |
| R-6 锁方案死锁面 | 锁不跨命令、不嵌套，无死锁；delete_platform 需补 State 参数，已注明 |
| R-15 验证命令对 HTTP 200 错误体的处理 | 已补"必须检查响应体 code"（A-7） |
| 范围蔓延检查 | R-5 模型改动最大，已提供"不改动模型"的备选 2 与"旧端点可用"的备选 1，用户可否决 |

### 6.4 审查后仍未闭合的不确定性（如实保留）

1. 三条自动查询路径均**未用真实 Key 终验**——这是唯一剩余的硬阻塞，全部收敛到 §5 Q1–Q4；
2. 火山 billing region 二选一未定；
3. 方舟套餐到期接口可能确认为负结果（仅登录态），届时 Manual 兜底即最终结论。

> 元审查结论：v2 中不存在已知的误报主张；所有强结论均可回链到 §0.1 实测或附录 B 证据；无法证实的部分已显式降级或转入待确认清单。

---

## 附录 A：一键复跑命令清单

```bash
# 1. Rust 测试 + Lint（在 src-tauri/）
cd src-tauri && cargo test
cd src-tauri && cargo clippy --all-targets -- -D warnings

# 2. 前端类型检查 + 构建（在项目根）
pnpm exec vue-tsc --noEmit
pnpm build

# 3. 端到端手动验证
pnpm tauri dev
#   · 外链：点击"如何获取 Key"应打开系统浏览器
#   · Manual：新增/编辑/续费/删除到期条目，重启后仍在
#   · Api：配置真实 Key 后卡片显示真实数据
#   · 关窗：ps aux | grep -i keykeeper 应无残留进程
```

## 附录 B：证据来源清单

| 编号 | 来源 | 用途 |
|:---|:---|:---|
| S1 | DeepSeek 官方《Get User Balance》 https://api-docs.deepseek.com/api/get-user-balance/ | R-1 响应结构【官方】 |
| S2 | DeepSeek 响应示例（Apifog 镜像） https://deepseek.apifox.cn/ | R-1 示例交叉印证 |
| S3 | npm `dsh-usage-glm-cn@1.1.5`（unpkg / jsdelivr 可读源码） | R-2 端点与字段【实证】 |
| S4 | 火山《签名方法》 https://www.volcengine.com/docs/6369/67269 | R-3 签名规范【官方】 |
| S5 | 火山《签名过程 Demo》 https://www.volcengine.com/docs/6369/67270 | R-3 测试向量【官方】 |
| S6 | 火山《QueryBalanceAcct》 https://www.volcengine.com/docs/6269/1223898 | R-3 余额接口【官方】 |
| S7 | tauri-plugin-opener 2.5.5 `src/init-iife.js`（本机 cargo registry） | R-4 拦截机制【代码】 |
| S8 | 本仓库全量源码（提交 3a963ec） | 全部【代码】结论 |
