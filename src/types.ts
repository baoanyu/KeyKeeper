// 数据模型 v2 —— 与 src-tauri/src/models.rs 的 serde 契约对齐
// 枚举序列化为 snake_case 字符串；Option 字段序列化为 null

export type QuotaUnit = 'cny' | 'tokens' | 'seconds' | 'unknown';
export type Source = 'api' | 'manual';
export type PlatformMode = 'api' | 'manual';

/** 一个额度包 —— 对应手写记录里的一行 */
export interface Entitlement {
  label: string;
  /** 到期时间戳（本地时区当天 23:59:59，到期当天仍可用） */
  expires_at: number | null;
  unit: QuotaUnit;
  total: number | null;
  remaining: number | null;
  /** 备注 / 估算说明 */
  note: string | null;
}

/** 一个平台的完整状态 */
export interface PlatformStatus {
  id: string;
  display_name: string;
  source: Source;
  entitlements: Entitlement[];
  console_url: string | null;
  /** 平台级错误（查询失败 / Key 失效等） */
  error: string | null;
  updated_at: number;
}

/** 平台元数据 —— 唯一事实来源在 Rust 端 PLATFORM_SPECS */
export interface PlatformSpec {
  id: string;
  display_name: string;
  mode: PlatformMode;
  console_url: string;
  key_docs_url: string;
  key_hint: string;
  /** 空字符串表示不做格式校验 */
  key_pattern: string;
}
