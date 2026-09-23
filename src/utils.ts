import type { Entitlement, PlatformStatus, QuotaUnit } from './types';

// ═══════════════════════════════════════════════════════════
// 日期语义（refactor-plan-v2.md §2.4）
//
// 存储：本地时区到期日当天 23:59:59 的时间戳 → 到期当天仍可用
// 展示：按自然日算剩余天数，不做小时级倒计时
// ═══════════════════════════════════════════════════════════

/** date input 的 YYYY-MM-DD → 本地时区当天 23:59:59 的 Unix 秒 */
export function dateInputToTimestamp(dateStr: string): number {
  const [y, m, d] = dateStr.split('-').map(Number);
  return Math.floor(new Date(y, m - 1, d, 23, 59, 59).getTime() / 1000);
}

/** Unix 秒 → date input 的 YYYY-MM-DD（本地时区） */
export function timestampToDateInput(ts: number): string {
  const d = new Date(ts * 1000);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** 自然日差：0 = 今天到期，负数 = 已过期 */
export function daysLeft(expiresAt: number): number {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const exp = new Date(expiresAt * 1000);
  const expDay = new Date(exp.getFullYear(), exp.getMonth(), exp.getDate());
  return Math.round((expDay.getTime() - today.getTime()) / 86400000);
}

/** 展示用日期，如 "10.23" */
export function formatDate(ts: number): string {
  const d = new Date(ts * 1000);
  return `${d.getMonth() + 1}.${d.getDate()}`;
}

/** 从今天起 +n 天的 date input 值（快捷录入用） */
export function dateInputInDays(n: number): string {
  const now = new Date();
  const target = new Date(now.getFullYear(), now.getMonth(), now.getDate() + n);
  const y = target.getFullYear();
  const m = String(target.getMonth() + 1).padStart(2, '0');
  const day = String(target.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

// ═══════════════════════════════════════════════════════════
// 到期标记与排序（refactor-plan-v2.md §2.7 / 1.6）
//
// 应用内标记是状态而非事件 —— 不需要节流
// ═══════════════════════════════════════════════════════════

/** 低额度阈值（原 check_low_balance 的单位阈值） */
export const LOW_BALANCE_THRESHOLDS: Record<QuotaUnit, number | null> = {
  cny: 10,
  tokens: 1000,
  seconds: 600,
  unknown: null,
};

export type Badge = 'red' | 'orange' | null;

/** 额度包级标记：到期 ≤3 天红（含已过期），≤7 天橙；余额低于阈值橙 */
export function entitlementBadge(e: Entitlement): Badge {
  if (e.expires_at != null) {
    const d = daysLeft(e.expires_at);
    if (d <= 3) return 'red';
    if (d <= 7) return 'orange';
  }
  if (isLowBalance(e)) return 'orange';
  return null;
}

export function isLowBalance(e: Entitlement): boolean {
  if (e.remaining == null) return false;
  const threshold = LOW_BALANCE_THRESHOLDS[e.unit];
  return threshold != null && e.remaining < threshold;
}

/** 平台紧迫度 = 最近到期天数；无到期日的平台排最后 */
export function platformUrgency(p: PlatformStatus): number {
  let min = Infinity;
  for (const e of p.entitlements) {
    if (e.expires_at != null) {
      min = Math.min(min, daysLeft(e.expires_at));
    }
  }
  return min;
}

/** 列表排序：到期近的在前（≤3 天自然置顶），无到期日的按名称排后面 */
export function sortPlatforms(list: PlatformStatus[]): PlatformStatus[] {
  return [...list].sort((a, b) => {
    const ua = platformUrgency(a);
    const ub = platformUrgency(b);
    if (ua !== ub) return ua - ub;
    return a.display_name.localeCompare(b.display_name, 'zh-CN');
  });
}
