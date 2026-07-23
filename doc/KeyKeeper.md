# KeyKeeper 设计文档

> ⚠️ **审计警示（2026-07-23）**：本文档描述的"火山方舟 HMAC-SHA256 完整签名"、"Qoder 无公开余额接口"等关键陈述已被审计推翻，详见 [audit-and-roadmap.md](audit-and-roadmap.md) 的 P0-3 / P0-4。**该修的地方本文档正文尚未修——因为 Sprint 0（抓真实 API）未完成，暂无权威事实可写**。修正后再更新此文档。

## 项目概述

KeyKeeper 是一个 **macOS 菜单栏应用**，聚合多个 AI 平台的 API 配额/余额查询。用户点击菜单栏图标弹出 400×600 毛玻璃窗口，一键刷新查看所有平台余额。

技术栈：**Tauri v2 + Rust + Vue3 + TypeScript + TailwindCSS**

## 架构设计

### 前后端通信

前端通过 `@tauri-apps/api/core` 的 `invoke()` 调用 Rust 端命令。

| 命令 | 说明 |
|:---|:---|
| `get_all_quotas` | 遍历已注册平台，并发获取配额 |
| `save_provider_key` | 保存 API Key 到 Keychain + 持久化平台列表 |
| `delete_provider` | 删除 API Key + 更新平台列表 |
| `get_saved_providers` | 获取已注册平台列表 |
| `add_provider` | 注册新平台 |
| `check_low_balance` | 检查低额度平台 |

### 配额获取流程

```
App.vue → invoke("get_all_quotas")
  → commands.rs::get_all_quotas
    → 从 AppState 读取 providers 列表（首次从 store 加载）
    → 从 Keychain 读取各平台 API Key
    → 为每个平台构造对应的 QuotaFetcher（共享 HTTP Client）
    → scheduler.rs::fetch_all_quotas（并发执行）
      → Semaphore(4) 限流 + 10s 超时
      → 各适配器调用平台 API
    → 返回 Vec<QuotaInfo>
```

### 适配器模式

每个平台实现 `QuotaFetcher` trait：

```rust
#[async_trait]
pub trait QuotaFetcher: Send + Sync {
    async fn fetch_quota(&self, api_key: &str) -> Result<QuotaInfo>;
}
```

#### 各平台实现

| 平台 | 鉴权方式 | 接口 | 解析字段 |
|:---|:---|:---|:---|
| **DeepSeek** | Bearer Token | `https://api.deepseek.com/user/balance` | `data.balance` |
| **智谱AI** | Bearer Token | `https://open.bigmodel.cn/api/paas/v4/balance` | `data.remaining_tokens` |
| **Qoder** | 本地估算 | 无公开接口 | 基于首次启动时间计算 5 小时倒计时 |
| **火山方舟** | HMAC-SHA256 签名 | `https://open.volcengineapi.com/api/v3/quota/balance` | `data.remaining` |

### 安全存储

- API Key 通过 `keyring` crate 存储在 macOS Keychain
- Service 名：`com.keykeeper.app`
- 平台列表和 Qoder 首次启动时间通过 `tauri-plugin-store` 持久化

### HTTP Client 复用

所有适配器共享一个 `Arc<reqwest::Client>` 实例：
- 连接池：`pool_max_idle_per_host(10)`
- 超时：30s

### macOS 交互

- 窗口：`decorations: false`、`transparent: true`、`alwaysOnTop: true`
- 托盘图标：左键 toggle 窗口，右键菜单含"退出"
- 失焦自动隐藏
- 窗口定位：屏幕顶部右侧
- 自动刷新：每 5 分钟触发 `auto-refresh` 事件
- 低额度通知：`check_low_balance` 命令检查

## 数据结构

```rust
pub struct QuotaInfo {
    pub provider_name: String,
    pub plan_type: PlanType,      // PayAsYouGo | CodingPlan | Subscription
    pub quota_unit: QuotaUnit,    // CNY | Tokens | Seconds | Unknown
    pub total: f64,
    pub remaining: f64,
    pub is_success: bool,
    pub error_msg: Option<String>,
}
```

## 关键配置

### Cargo.toml 依赖

```toml
[dependencies]
tauri = { version = "2", features = ["macos-private-api", "tray-icon"] }
tauri-plugin-store = "2"
tauri-plugin-notification = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
reqwest = { version = "0.12", features = ["json", "native-tls"] }
anyhow = "1"
keyring = "3"
futures = "0.3"
async-trait = "0.1"
chrono = "0.4"
hmac = "0.12"
sha2 = "0.10"
base64 = "0.22"
hex = "0.4"
```

### 避坑指南

| 问题 | 解决方案 |
|:---|:---|
| Keychain 权限错误 | 配置 `entitlements.plist` + `com.apple.security.device.keychain` |
| Qoder 计时不准 | 首次启动时间持久化到 store，UI 标注"本地估算" |
| 火山方舟签名 | 完整 HMAC-SHA256 签名，API Key 格式 `AccessKey:SecretKey` |
| 窗口定位 | 屏幕顶部右侧，距右边缘 20px |
| 并发控制 | Semaphore(4) 匹配适配器数量 |
| HTTP 性能 | 共享 `Arc<Client>` 复用连接池 |
