# KeyKeeper 架构审查与优化建议清单

> 审查人：架构师（software-architect）
> 日期：2026-09-24
> 审查范围：全项目（前端 Vue 3 + TypeScript 281 行 App.vue 为核心 / 后端 Rust 2566 行）
> 审查基线：未提交工作树（`git dc8dc52` + 本次优化改动的 5 改 2 新文件）
> 验证基线：`vue-tsc --noEmit` ✅ 通过 · `cargo test` ✅ 23 passed

---

## 一、执行摘要

**本次优化产出质量评级：B+**

分组渲染（标记项/普通项分离）、骨架屏、过渡动画三项改造方向正确、实现干净：`splitPlatforms()` 为纯函数且保持组内独立排序语义，`PlatformSection` 用 props/emits/slots 正确封装了区块容器，`QuotaCard` 的事件透传链清晰。本次改动未引入类型安全问题（全库零 `any`、零 `v-html`），`vue-tsc` 零错误。

**关键发现（5 条）：**

1. **【P1 · 性能】** `App.vue onMounted` 中 `get_platform_specs` 与首次 `refresh()` 串行执行，首屏白屏被无谓延长——两个命令无数据依赖，可并行。
2. **【P1 · 内存】** `listen('auto-refresh')` 返回的 `Promise<UnlistenFn>` 从未保存，事件监听器无法注销——关窗退出架构下不构成运行时泄漏，但属 API 误用，窗口若改为常驻即泄漏。
3. **【P1 · 文档】** 本次分组/骨架屏改造完全未回写文档：CLAUDE.md 的配额获取流程图仍写「前端 sortPlatforms 排序」（实际已改为 splitPlatforms 分组排序），代码与事实来源脱节。
4. **【P2 · 架构】** `QuotaCard` 的四个命令式函数（`isAuthError`/`isManual`/`openConsole`/`anyLow`）定义在 `<script setup>` 顶层，每次组件实例化重建，且 `openConsole` 是唯一的 Tauri 依赖点却不可测——应收进 `utils.ts` 纯函数。
5. **【P2 · 性能】** 5 分钟自动刷新对**全量平台**（含 Manual 纯本地数据）发起完整 Rust 调度；Manual 条目不变化，却每次参与全量刷新与 TransitionGroup 重渲染——应拆分刷新范围或做数据比对短路。

**最优先改进项：** 首屏并行加载（P-01，一行改动，零风险，首屏提速约一个 IPC 往返）+ 文档回写（M-01，防止后续 agent 在过期认知上写代码）。

---

## 二、优化产出质量评估

### 2.1 `src/utils.ts`（+21 行）—— 亮点与风险

**亮点：**
- `PlatformGroup` 接口 + `splitPlatforms()` 为**纯函数**，无副作用，天然可单测（当前**无测试**，见 Q-05）。
- 组内独立 urgency 排序、不跨组混排的语义在 JSDoc 中显式注明，与 `system_design.md §3.2` 决策对齐。
- `sortPlatforms()` 的 `[...list]` 展开拷贝避免污染入参，函数纯度保持良好。

**风险点：**
- `splitPlatforms` 内部两次 `filter` + 两次 `sortPlatforms`，对同一列表遍历 4 次。数据量（<20 平台）下无实际影响，但函数组合可读性优于微优化，**无需改动**，记录备查。
- `platformUrgency` 返回 `Infinity` 表示「无到期日」，依赖 `Math.min` 语义正确；若未来 entitlement 数量级增长仍是 O(n)，无瓶颈。

### 2.2 `src/style.css`（+39 行）

**亮点：**
- `.list-leave-active` 的 `position: absolute; width: 100%` 是 TransitionGroup 的标准正确姿势——离开元素脱离布局流才能触发兄弟元素的 move 过渡，注释也解释了原因。
- `fade-*` 消息条过渡与 `list-*` 列表过渡分离，命名空间清晰。

**风险点：**
- `.list-leave-active` 的绝对定位参照是 `PlatformSection` 的 `.relative` 容器（`style.css` 中无全局 `relative` 定义，依赖 `PlatformSection.vue:54` 的内联 class）——**隐式耦合**：若 `PlatformSection` 移除 `relative`，leave 过渡的定位参照将漂移到更外层，动画变形。建议在 `PlatformSection.vue` 注释中标注「relative 为 list-leave 过渡锚点，勿删」。

### 2.3 `src/components/SkeletonCard.vue`（新文件，26 行）

**亮点：**
- 纯展示、零脚本、零 props——骨架结构与 `QuotaCard` 的 DOM 布局逐行对齐（标题行 → 额度包 ×2），满足「占位尺寸与真实卡片对齐」的设计目标。
- `animate-pulse` 使用 Tailwind 内置动画，无额外 keyframes。

**风险点：**
- 骨架屏硬编码「2 个额度包」结构；若 `QuotaCard` 布局未来大改，骨架需人工同步。数据量极小，**接受此耦合**，记录备查。

### 2.4 `src/components/PlatformSection.vue`（新文件，67 行）

**亮点：**
- props 带完整 JSDoc（含 `loading` 的语义边界「仅 platforms 为空时显示骨架屏」），emits 用 **tuple 类型签名**（`delete: [p: PlatformStatus]`）——Vue 3 类型化 emits 的最佳实践。
- `below-card` 插槽设计恰当：编辑表单的渲染责任留在 App.vue（它持有 `editingId` 状态），区块容器只做透传，职责边界清晰。
- `v-if="loading && platforms.length === 0"` 的骨架屏条件精确——非首屏刷新不闪骨架。

**风险点：**
- `defineSlots` 的返回类型用了 `any`（`'below-card'?: (props: { platform: PlatformStatus }) => any`）——这是 Vue 3.3+ 官方 defineSlots 类型签名的标准写法（插槽返回 VNode 数组，类型系统层面即 `any`），**不是类型安全缺陷**，无需修改。见 Q-01 说明。

### 2.5 `src/App.vue`（±97 行）

**亮点：**
- `groupedPlatforms` → `manualPlatforms`/`apiPlatforms` 三级 computed 链式拆分，响应式依赖追踪精确（任一 `platforms` 变化只重算受影响分支）。
- `v-if="manualPlatforms.length > 0 || loading"` 的区块显隐条件正确：加载期间保持区块可见以展示骨架屏，空数据时隐藏区块。
- 手动录入区块通过 `#below-card` 插槽集成编辑表单，未把 `editingId`/`editInitial` 状态下沉到 `PlatformSection`——状态归属合理。

**风险点：**
- 文件 281 行、约 15 个 ref/computed + 7 个业务函数，集中度尚可（见 A-01 分析），但 `addApi` 的「验证-回滚」逻辑（try/catch/finally 三层嵌套 + 旧 Key 恢复）是全项目最复杂的单函数，建议后续抽离（见 Q-02）。

### 2.6 `src/components/QuotaCard.vue`（+15 行）

**亮点：**
- R-12 进度条红色映射已落地（`barColor` 的 red/orange/blue 三分支），与 remediation 文档 §3 批次 A 的承诺一致。
- `isAuthError()` 的错误分类用正则覆盖 401/403/unauthorized/中文关键词，防御性良好。

**风险点：**
- 见 Q-02：四个顶层函数 + `openUrl` 副作用混在组件实例作用域。

### 2.7 `src/components/RefreshBar.vue`（±10 行）

**亮点：** loading 态的 SVG 旋转动画 + `disabled` 态样式完整；`defineEmits<{ refresh: [] }>()` 零参签名正确。

**风险点：** 无。本文件是全项目最干净的组件。

---

## 三、架构审查发现

### A-01　App.vue 状态集中度：可接受，但已接近单文件承载上限

- **现状描述：** App.vue 281 行，承载 12 个响应式状态（specs/platforms/loading/lastUpdated/error/success/verifying/verifyMessage/reconfigure/editingId/editInitial/formKey）+ 7 个业务函数（refresh/addApi/addManual/startEdit/saveEdit/deletePlatform/startReconfigure）+ 2 个消息计时器 + 1 个事件监听。
- **影响分析：** 当前规模（约 450 行含模板）处于「单文件可维护」与「需要拆分」的临界区。本次分组改造通过 `PlatformSection` 下沉了渲染职责，状态层未膨胀（新增状态为 0，仅新增 2 个 computed 派生）——**本次改造对架构健康度是正向的**。真正的复杂度热点是 `addApi`（验证-回滚状态机，46 行），不是状态数量。
- **建议方案：** 短期不拆文件；将 `addApi` 的验证-回滚逻辑抽为独立 async 函数（如 `verifyKeyThenSave(id, key): Promise<{ ok: boolean; error?: string }>`），App.vue 只负责调用与 UI 响应。
- **影响等级：** P2（技术债清理，无功能风险）

### A-02　组件层级 App → PlatformSection → QuotaCard：嵌套合理，但事件链可缩短

- **现状描述：** delete/retry/reconfigure/edit 四级透传：`QuotaCard emit` → `PlatformSection emit('xxx', ...)` → `App @xxx`。
- **影响分析：** 四级透传是 Vue 官方推荐模式的代价——中间层（PlatformSection）对业务事件零加工（纯 `emit('delete', p)`），每加一个事件要改 3 个文件。当前 4 个事件 × 3 文件 = 12 处样板代码。但换用 provide/inject 或事件总线会牺牲类型安全与可追踪性，**对 4 个事件的规模而言是过度设计**。
- **建议方案：** 维持现状。若事件增至 6 个以上，考虑 `PlatformSection` 用 `v-bind="$attrs"` 透传或引入 mitt 事件总线。另注意 `QuotaCard` 的 `delete: []`（无参）与 `PlatformSection` 的 `delete: [p: PlatformStatus]`（带参）签名不对称——PlatformSection 转发时补了 `p`，功能正确但增加了理解成本，可统一为带参（见 Q-04）。
- **影响等级：** P3（当前无痛点，事件数翻倍时再重构）

### A-03　前后端通信契约稳定性：serde 契约有测试守护，前端类型靠人工对齐

- **现状描述：** Rust 端 `models.rs` 有 6 个契约测试（snake_case 序列化 ×2、JSON 字段名、id 唯一性、key_hint 完备性、failed 结构）；前端 `types.ts` 注释声明「与 models.rs 的 serde 契约对齐」，但**无任何自动化校验**。
- **影响分析：** 前后端字段名/类型漂移只能靠 `vue-tsc` + `cargo test` 各自通过来间接保证，字段重命名（如 `used_percent`）时若漏改一端，运行期才暴露（TS 类型不匹配会报错，但 `invoke<PlatformStatus[]>` 的泛型是编译期断言，Rust 端反序列化失败会变成 `catch (e)` 的 `String(e)` 错误——用户看到的是「[object Object]」类错误而非清晰信息）。
- **建议方案：** ① 短期：在 `refresh()` 的 catch 中把 `String(e)` 改为提取 `e?.message ?? JSON.stringify(e)` 的友好格式（一行）；② 中期：考虑引入 `zod` 或手写运行时校验，或至少在 CI 加一个「types.ts 字段 ⊆ models.rs 字段」的文本比对脚本。
- **影响等级：** P2（契约变更时的回归防护缺口）

### A-04　首屏加载路径：串行等待可并行化（与 P-01 联动）

- **现状描述：** `onMounted` 中 `await invoke('get_platform_specs')` 完成后才 `await refresh()`（内部 `invoke('get_all_platforms')`）。两个命令**无数据依赖**。
- **影响分析：** 首屏渲染被两个串行 IPC 往返（各约 5-50ms）+ 后端调度延迟（Api 平台网络请求，最长 10s 超时）依次阻塞。specs 仅用于添加表单，**不影响平台列表首屏**——但模板中 `AddProviderForm` 在列表之前渲染，specs 未就绪时表单下拉为空。
- **建议方案：** 见 P-01。
- **影响等级：** 见 P-01（P1）

### A-05　Rust 端适配器并发效率：Semaphore 限流与 HTTP 池配置合理

- **现状描述：** `scheduler.rs` 用 `Semaphore(4)` 限流 + 10s 超时 + `join_all`；`lib.rs` 共享 `Arc<Client>`（`pool_max_idle_per_host(10)`，30s 超时）。
- **影响分析：** 当前 3 个 Api 平台，Semaphore(4) 实际不限流（任务数 ≤ 3），配置为未来扩展预留，合理。10s 请求超时 vs 30s 客户端超时的分层正确（先断请求、后断连接）。`join_all` 保序返回，与 `specs` 数组索引对应，panic 兜底为 `PlatformStatus::failed`——健壮。
- **建议方案：** 无需改动。唯一记录项：`tokio::spawn` 的任务在应用退出时（`app.exit(0)`）被直接终止，未完成的抓取请求结果丢弃——与「关窗即退出」语义一致，无影响。
- **影响等级：** 无（正向确认）

---

## 四、代码质量审查发现

### Q-01　TypeScript 类型安全性：整体优秀，无 `any`/`v-html`

- **现状描述：** 全库检索：`any` 仅出现于 `PlatformSection.vue:29` 的 `defineSlots` 返回类型（Vue 官方类型签名写法，非缺陷）；`v-html` 零使用；`as any` 零使用。`tsconfig` 开启 `strict` + `noUnusedLocals` + `noUnusedParameters` + `noFallthroughCasesIn`。
- **影响分析：** 类型安全基线全项目最佳。`QuotaCard` 的 `defineEmits` tuple 签名、`App.vue` 的 `invoke<PlatformStatus[]>` 泛型断言、`PlatformSection` 的 `defineSlots` 类型参数——三处均用了 Vue 3 最严格的类型形式。
- **建议方案：** 无需改动。记录为团队范式。
- **影响等级：** 无（正向确认）

### Q-02　`QuotaCard.vue` 顶层函数与 Tauri 副作用耦合，不可测

- **现状描述：** `isAuthError()`/`isManual()`/`openConsole()`/`anyLow()` 定义在 `<script setup>` 顶层（非模板内联），每次组件实例化重建。其中 `openConsole` 直接调用 `openUrl()`（`@tauri-apps/plugin-opener`），**是该组件唯一的外部副作用**。
- **影响分析：** ① 性能：卡片数量 × 每次渲染的函数重建开销（实际微小，Vue 会缓存模板编译产物，但逻辑函数仍每实例一份）；② 可测性：`isAuthError` 的正则分类逻辑（401/403/中文关键词匹配）是纯函数，**完全可以单测**，但当前夹在组件里无法独立测试——这是本条的核心问题；③ 扇出：`openUrl` 的调用埋在组件深处，未来接其他端（如 CI 预览）时难以替换。
- **建议方案：** 将 `isAuthError(error: string): boolean`、`isManual(p: PlatformStatus): boolean`、`anyLow(p: PlatformStatus): boolean` 移入 `utils.ts` 作为纯函数；`openUrl` 调用点保留在组件（或同样移入 utils 的 `openConsole(url: string)`，由 utils 承担副作用）。配套补 2-3 个单测。
- **影响等级：** P2（可测试性 + 可维护性，无功能风险）

### Q-03　Vue 3 Composition API 最佳实践：computed vs watch 使用正确

- **现状描述：** `AddProviderForm.vue` 用 `watch(() => props.preselected, ...)` 监听对象引用变化（配合 `n` 递增实现「重复点击同一平台也触发」——P1-4 模式）。
- **影响分析：** 这是**正确的**用法：监听目标是 `props.preselected` 的引用（`ref` 对象整体替换），而非深层属性，无需 `{ deep: true }`。`immediate: true` 处理了初始 preselected 非空的边界（虽然当前流程 preselected 初始为 null，属防御性配置）。`computed` vs `watch` 的取舍（派生值用 computed、副作用用 watch）在全项目执行一致。
- **建议方案：** 无需改动。唯一可选优化：`showForm` 的初始 `ref(true)` 与「默认选中第一个可选平台」的初始化逻辑（第 54-56 行）在 setup 顶层同步执行，可读性可接受。
- **影响等级：** 无（正向确认）

### Q-04　组件 props/emits 设计：整体合理，一处签名不对称

- **现状描述：** `QuotaCard` 的 `delete: []`（无参）vs `PlatformSection` 的 `delete: [p: PlatformStatus]`（带参）。`reconfigure`/`edit` 均为带参且对称。
- **影响分析：** 功能正确（PlatformSection 转发时补 `p`），但阅读者需对照两个文件才能确认 delete 携带平台对象。`retry: []` 两处均无参，对称。
- **建议方案：** 将 `QuotaCard` 的 `delete` emit 改为 `[p: PlatformStatus]`，模板中 `@click="emit('delete', platform)"`，与 PlatformSection 统一。一行改动。
- **影响等级：** P3（一致性微调）

### Q-05　工具函数纯度与可测试性：纯函数零测试覆盖

- **现状描述：** `utils.ts` 全部 8 个导出函数/常量均为纯函数（输入 → 输出，无 IO），但 `utils.ts` **零单测**。对比：Rust 端 `sanitize_error_body` 有 4 个单测、解析函数各有 fixture 测试。
- **影响分析：** 前端测试基建为零（`package.json` 无 test script、无 vitest 依赖），纯函数是唯一可低成本补齐的测试层。`daysLeft`（自然日计算，时区边界）、`dateInputToTimestamp`（本地时区 23:59:59 语义）、`entitlementBadge`（红/橙标记优先级）三处逻辑**有边界条件出错的可能**（如跨夏令时、expires_at 为 0）。
- **建议方案：** 引入 `vitest`（与 vite 同生态，零额外构建成本），为 `utils.ts` 补 6-8 个单测，优先覆盖 `daysLeft`（今天/明天/已过期/闰年 2 月 29 日）与 `entitlementBadge`（到期优先于低余额）。
- **影响等级：** P2（测试债，但纯函数层是最优起点）

---

## 五、性能瓶颈审查发现

### P-01　首屏加载：specs 与 platforms 串行等待，可并行

- **现状描述：** `App.vue onMounted`：
  ```ts
  specs.value = await invoke<PlatformSpec[]>('get_platform_specs');  // 等待完成
  await refresh();                                                    // 再发起
  ```
  两个命令无数据依赖，但被 `await` 串行化。
- **影响分析：** 首屏路径 = IPC(specs) + IPC(platforms) + Rust 调度（Keychain 读取 + Api 平台网络抓取，最长 10s）。串行化使 specs 的 IPC 耗时（约 5-20ms）叠加到 platforms 之前。specs 仅影响添加表单的填充，**不阻塞平台列表展示**——但模板顺序中表单在列表前，specs 未就绪时表单区空白。并行化后首屏总耗时 = max(specs, platforms) 而非两者之和。
- **建议方案：**
  ```ts
  const [specsResult] = await Promise.all([
    invoke<PlatformSpec[]>('get_platform_specs').catch((e) => { showError(String(e)); return [] as PlatformSpec[]; }),
    refresh(),  // refresh 内部自带 try/catch
  ]);
  specs.value = specsResult;
  ```
  或更简：`refresh()` 不等待 specs，两者 `Promise.all` 并行。注意保留 specs 失败不阻塞列表的容错。
- **预期收益：** 首屏白屏时间减少 5-20ms（单次 IPC 往返）+ 表单区提前就绪；代码改动约 5 行，零风险。
- **影响等级：** P1（近期改进）

### P-02　响应式依赖链深度：computed 链精确，无冗余重算

- **现状描述：** `platforms`（ref）→ `groupedPlatforms`（computed）→ `manualPlatforms`/`apiPlatforms`（computed）→ `PlatformSection props` → `TransitionGroup v-for`。
- **影响分析：** 依赖链深度 4 层，但每层都是必要的语义拆分（分组 → 子组 → 渲染），且 Vue 的 computed 缓存保证 `platforms` 不变时零重算。`configuredIds`（`platforms.value.map(...)`）每次 platforms 变化生成新数组——`AddProviderForm` 的 `availableSpecs` computed 依赖它，而 `availableSpecs` 的过滤逻辑对每个 spec 调用 `configuredIds.includes`——O(n²)，n ≤ 8，**无实际影响**。`PlatformSection` 的 `platforms.length` 在模板标题中直接访问（非 computed），每次渲染重算——O(n)，无影响。
- **建议方案：** 无需改动。若未来平台数 > 50，再将 `configuredIds` 改 `Set` 并加 computed 缓存。
- **影响等级：** 无（正向确认，记录扩展阈值）

### P-03　5 分钟自动刷新：全量刷新对 Manual 数据是浪费

- **现状描述：** `lib.rs` 每 300s 发射 `auto-refresh` → 前端 `refresh()` → `invoke('get_all_platforms')` → Rust 端**全量调度**（Api 平台重新抓取网络请求 + Manual 平台重新读 store + 重新序列化）→ 前端整体替换 `platforms.value` → 所有 `QuotaCard` 重渲染 + TransitionGroup move 过渡。
- **影响分析：** ① Manual 条目是本地静态数据，5 分钟内变化概率极低，却每次参与全量 IPC + 序列化；② `platforms.value` 整体替换使所有卡片（含未变化的）触发重渲染——Vue 的 keyed diff 会复用 DOM，但 `QuotaCard` 的 computed（`anyLow`/`barWidth` 等）会全部重算；③ 网络层面：3 个 Api 平台每 5 分钟一次请求，对第三方 API 是持续低频压力，合理。
- **建议方案（二选一）：**
  - **方案 A（推荐，前端短路）：** `refresh()` 返回数据后比对 `updated_at` 与内容哈希，无变化时跳过 `platforms.value` 赋值（保留 `lastUpdated` 更新）。需 `get_all_platforms` 返回不变时前端零渲染。
  - **方案 B（后端拆分）：** 新增 `get_api_platforms` 命令只查 Api 平台；Manual 数据在 `save_manual_platform` 后已由前端本地更新，无需重复拉取。改动较大但语义更干净。
  - 当前阶段数据量小、5 分钟频率低，**实际资源消耗可接受**——但方案 A 的「无变化不赋值」是响应式最佳实践，顺带消除 TransitionGroup 的无意义 move 动画。
- **影响等级：** P2（远期优化；方案 A 成本约 2 小时）

### P-04　内存泄漏风险：定时器有清理，事件监听器未清理

- **现状描述：** ① `successTimer`/`errorTimer`（`window.setTimeout`）：每次 `showSuccess`/`showError` 先 `clearTimeout` 旧计时器再设新——**处理正确**，但组件卸载时未清理（若计时器在途时组件销毁，回调触发 `success.value = ''` 写入已卸载组件的 ref——Vue 3 中无害但属泄漏模式）；② `listen('auto-refresh')` 返回 `Promise<UnlistenFn>`——App.vue **未保存返回值**，监听器永不注销；③ `lib.rs` 的 `tokio::time::interval` 随进程退出终止，无泄漏。
- **影响分析：** 关窗即退出架构下，窗口关闭 = 进程退出，所有监听器随进程回收——**当前无运行时泄漏**。但这是「靠架构兜底而非代码自律」：若未来窗口改为常驻（如最小化到托盘方案复活），`listen` 永不注销将累积泄漏；且 `onUnmounted` 缺失使组件无法被安全复用于其他生命周期场景。
- **建议方案：**
  ```ts
  let unlisten: (() => void) | undefined;
  onMounted(async () => {
    // ...
    unlisten = await listen('auto-refresh', () => { refresh(); });
  });
  onBeforeUnmount(() => { unlisten?.(); });
  ```
  补 `onBeforeUnmount` 清理计时器（2 行）。改动 5 行，零风险。
- **影响等级：** P1（代码自律 + 未来形态保险；当前无实际泄漏）

### P-05　Rust 端适配器并发效率：配置合理，无瓶颈

- **现状描述：** 见 A-05。补充：`reqwest::Client` 的 `pool_max_idle_per_host(10)` 对 3 个不同 host（deepseek/bigmodel/volcengine）各维护 10 个空闲连接——共 30 个连接上限，每 5 分钟一轮请求，池实际利用率极低但内存占用可忽略（每连接约几 KB socket 缓冲）。
- **影响分析：** 无瓶颈。`native-tls` 特性在 macOS 上使用系统钥匙串证书，与 Keychain 存储 TLS 凭证的语义一致。
- **建议方案：** 无需改动。
- **影响等级：** 无（正向确认）

---

## 六、安全审查发现

### S-01　Keychain 使用安全性：service 名 + 条目名设计正确，密钥不进日志/文件

- **现状描述：** `keystore.rs` 用 `keyring` crate，service=`com.keykeeper.app`，条目名=平台 id（`deepseek`/`zhipu`/`volcano`）；`save_key`/`get_key`/`delete_key` 三函数；`delete_key` 容忍 `NoEntry`；Adapters 的 `api_key` 参数经 `bearer_auth()` 或签名器使用，**不进入日志**（错误响应经 `sanitize_error_body` 脱敏）；`get_api_key` 命令仅返回 Key 给前端用于「失败时回滚旧值」，前端**不持久化**（`addApi` 的 `oldKey` 是函数内局部变量）。
- **影响分析：** Key 全生命周期：用户输入 → Keychain → 内存传递 → HTTP 请求头 → 错误时回滚写回 Keychain。不落盘（store 只存 Manual 数据）、不进日志、不回显 UI（input type="password"）。**安全链路完整**。唯一注意：`get_api_key` 把 Key 返回到前端内存——若前端未来调试打印需注意；当前代码无打印，安全。
- **建议方案：** 无需改动。记录为安全范式。
- **影响等级：** 无（正向确认）

### S-02　store 数据安全性：Manual 明文存储可接受，CSP 配置正确

- **现状描述：** `tauri-plugin-store` 的 `keykeeper-store.json` 存 Manual 到期数据（`platform_id → { entitlements, updated_at }`），**明文无加密**；CSP 配置（`tauri.conf.json`）：`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src ... ipc:`——`unsafe-inline` 为 style（Tailwind 运行时需要），script 无 inline。
- **影响分析：** Manual 数据是「何时到期」而非凭证，明文存储的风险等级 = 本机其他用户可读——桌面单用户场景可接受，且与「关窗退出」的轻量定位一致。CSP 无 `unsafe-eval`、无远程 script 来源，`connect-src` 仅 `self/ipc:`——无 SSRF/数据外泄通道。XSS 面：全库零 `v-html`、零 `innerHTML`，模板插值 `{{ }}` 默认转义——**XSS 攻击面已闭合**。
- **建议方案：** 无需改动。若未来 Manual 数据含更多敏感信息（如账号名），再评估 store 加密。
- **影响等级：** 无（正向确认，记录扩展触发条件）

---

## 七、可维护性审查发现

### M-01　文档完整性：本次分组改造完全未回写文档，CLAUDE.md 已与代码脱节

- **现状描述：** ① CLAUDE.md「配额获取流程图」仍写「前端 sortPlatforms 排序 + 应用内标记」——实际已改为 `splitPlatforms()` 分组排序（手动标记/API 平台两区块独立渲染）；② CLAUDE.md 无 `PlatformSection`/`SkeletonCard` 组件说明、无 TransitionGroup 过渡动画说明、无「标记项/普通项分离」的产品语义；③ `doc/refactor-plan-v2.md` 与 `doc/code-review-2026-09-remediation.md` 均无本次改造记录；④ `.gitignore` 已加 `/doc/`（commit `3c0ee08`），但 `doc/` 三份历史文件仍在版本库（`git ls-files` 确认）——**新文档如直接写入 `doc/` 将被 gitignore**，本次审查报告落盘后需 `git add -f` 或调整 ignore 规则。
- **影响分析：** ① 后续 agent（含 Claude Code 会话）读 CLAUDE.md 会基于「sortPlatforms 单列表排序」的过期认知写代码——**这是最高优先级文档债**；② `/doc/` 与 `docs/` 双目录并存（`docs/` 为架构设计产物，含 `system_design.md` + 两张 mermaid），职责边界模糊，新人易混淆；③ 分组改造是产品语义变更（「手动标记」成为一等区块），应有需求/设计记录。
- **建议方案：**
  1. **立即**：CLAUDE.md 流程图与组件清单补写分组改造（`splitPlatforms` + `PlatformSection` + 骨架屏 + 过渡动画），约 15 行；
  2. **立即**：`.gitignore` 的 `/doc/` 规则改为白名单例外（`!doc/*.md` 或仅忽略特定子目录），或将 `docs/` 合并进 `doc/` 统一入口；
  3. **近期**：在 `doc/refactor-plan-v2.md` 补 §2.8「分组渲染改造」小节，记录决策背景（标记项/普通项分离的产品动机）。
- **影响等级：** P1（文档脱节会直接污染后续开发认知）

### M-02　命名一致性：整体规范，一处语义模糊

- **现状描述：** 组件命名（`QuotaCard`/`PlatformSection`/`SkeletonCard`/`RefreshBar`/`AddProviderForm`/`ManualEntryForm`） PascalCase + 职责后缀，统一；工具函数（`dateInputToTimestamp`/`timestampToDateInput`/`daysLeft`/`formatDate`）语义化命名，一致；**例外**：`PlatformSection.vue` 注释称「分组区块容器（system_design.md §3.1）」，但 `system_design.md` 的 §3.1 实际是「数据结构与 Interfaces」——引用锚点错误（应为 §3.2 标记项/普通项分离方案）。
- **影响分析：** 文档锚点漂移，后续按图索骥会落空。
- **建议方案：** 修正 `PlatformSection.vue:6` 的注释锚点为 `system_design.md §3.2`；全库 grep 其他 `system_design.md` 引用核对。
- **影响等级：** P3（一行注释修正）

### M-03　错误处理完备性：前端错误信息有提升空间

- **现状描述：** ① `refresh()` 的 `catch (e) { showError(String(e)) }`——Tauri `invoke` 的拒绝值是 Rust 端 `map_err(|e| e.to_string())` 的 String，`String(e)` 会得到 `"Error: xxx"` 或 `"[object Object]"` 类信息；② `addApi` 的三处错误路径（验证失败/保存失败/删除失败）均 `showError(String(e))`；③ 后端错误链路完整（`anyhow` 链 + `is_no_entry` 区分 + `sanitize_error_body` 脱敏），**前端展示层是唯一薄弱环节**。
- **影响分析：** 用户看到的错误可能是 `"Error: Key 验证失败: HTTP 401: ..."`（可接受）或 `"[object Object]"`（不可接受，若 invoke 拒绝值非 String）。
- **建议方案：** 统一错误提取函数：
  ```ts
  function errMsg(e: unknown): string {
    if (typeof e === 'string') return e;
    if (e instanceof Error) return e.message;
    return JSON.stringify(e);
  }
  ```
  替换 5 处 `String(e)`。一行函数 + 5 处替换。
- **影响等级：** P2（用户体验 + 可调试性）

---

## 八、优化建议优先级清单（按影响程度排序）

| 优先级 | 编号 | 建议 | 预期收益 | 实施难度 | 建议时机 |
|:---:|:---:|:---|:---|:---|:---|
| P1 | P-01 | `onMounted` 中 specs 与 platforms 加载并行化（`Promise.all`） | 首屏白屏减少 5-20ms + 表单提前就绪 | 极低（5 行） | 立即 |
| P1 | M-01 | CLAUDE.md 回写分组改造 + 修正 `/doc/` gitignore 策略 | 消除文档-代码脱节，后续 agent 认知一致 | 低（30 分钟） | 立即 |
| P1 | P-04 | `listen` 保存 `UnlistenFn` + `onBeforeUnmount` 清理计时器与监听器 | 消除未来常驻形态的泄漏隐患，组件生命周期自律 | 极低（5 行） | 立即 |
| P1 | P-03 | 自动刷新无变化时跳过 `platforms.value` 赋值（updated_at 比对） | 消除无意义全量重渲染 + TransitionGroup 空转 | 低（约 1 小时） | 近期 |
| P2 | Q-02 | `QuotaCard` 纯函数（`isAuthError`/`isManual`/`anyLow`）移入 `utils.ts` 并补单测 | 可测试性 + 职责清晰 | 低（约 1 小时） | 近期 |
| P2 | Q-05 | 引入 vitest，为 `utils.ts` 纯函数补 6-8 个单测 | 前端测试债清零起点，边界条件防回归 | 低（约 1 小时） | 近期 |
| P2 | M-03 | 统一 `errMsg(e)` 错误提取，替换 5 处 `String(e)` | 错误信息可读性，消除 `[object Object]` | 极低（10 分钟） | 近期 |
| P2 | A-01 | 抽离 `addApi` 的验证-回滚状态机为独立函数 | 降低 App.vue 最复杂函数的心智负担 | 中（约 2 小时） | 近期 |
| P2 | A-03 | `refresh()` catch 中错误信息格式对齐（与 M-03 联动）+ 契约漂移防护评估 | 契约变更时回归可发现 | 中 | 近期 |
| P3 | Q-04 | `QuotaCard` 的 `delete` emit 改带参，与 `PlatformSection` 签名统一 | 事件签名一致性 | 极低（1 行） | 可选 |
| P3 | M-02 | 修正 `PlatformSection.vue` 注释中 `system_design.md §3.1` → `§3.2` 锚点 | 文档引用准确性 | 极低（1 行） | 可选 |
| P3 | A-02 | 事件透传链维持现状；事件数 ≥6 时再评估 `v-bind="$attrs"` 或 mitt | — | — | 远期观察 |
| P2 | P-03b | （备选）后端拆分 `get_api_platforms`，Manual 数据前端本地更新 | 消除 Manual 数据的全量刷新浪费 | 中 | 远期 |
| P3 | P-02 | 平台数 >50 时 `configuredIds` 改 `Set` + computed 缓存 | — | — | 远期观察 |

---

## 九、后续行动建议

1. **【立即 · 本次提交内】** 将 P-01（并行加载）、P-04（监听器清理）、Q-04（delete 签名统一）、M-02（注释锚点）四处微改动与本次分组改造一并提交，并在 CLAUDE.md 补写「分组渲染 + 骨架屏 + 过渡动画」章节（M-01），同步修正 `.gitignore` 的 `/doc/` 规则（白名单 `!doc/*.md`）。

2. **【本周】** 引入 vitest 为 `utils.ts` 建立前端测试基线（Q-05），优先覆盖 `daysLeft`（含闰年 2.29、跨夏令时）与 `entitlementBadge`（到期优先于低余额的优先级）；顺带把 `isAuthError` 移入 utils 并测试（Q-02）。

3. **【本周】** 落地 P-03 方案 A（自动刷新无变化不赋值）与 M-03（`errMsg` 统一），两者均 <1 小时，直接提升日常使用体验。

4. **【下次迭代前】** 执行 remediation 文档 §5 的批次 C「真实 Key 终验」（Q1-Q4）——这是比任何性能优化都重要的**功能正确性阻塞项**，三个 Api 适配器至今未用真实 Key 跑通一次完整流程。

5. **【持续】** 每季度核对 CLAUDE.md 与代码的一致性（尤其是数据流图与组件清单），把「文档回写」纳入 Definition of Done——本次 M-01 脱节本可在改造提交时避免。

---

> 审查方法说明：本次审查基于静态代码走查（全量 2566 行 Rust + 约 700 行 Vue/TS）+ 基线验证（`vue-tsc --noEmit` 通过、`cargo test` 23 passed）。性能与安全的「正向确认」结论基于代码路径分析，未做运行时 profiling 与渗透测试。
