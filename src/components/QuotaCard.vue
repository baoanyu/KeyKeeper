<script setup lang="ts">
import { openUrl } from '@tauri-apps/plugin-opener';
import type { Entitlement, PlatformStatus, QuotaUnit } from '../types';
import { daysLeft, entitlementBadge, formatDate, isLowBalance } from '../utils';

const props = defineProps<{ platform: PlatformStatus }>();
const emit = defineEmits<{
  delete: [];
  retry: [];
  reconfigure: [id: string];
  edit: [id: string];
}>();

const unitLabel: Record<QuotaUnit, string> = {
  cny: '元',
  tokens: 'tokens',
  seconds: '秒',
  unknown: '',
};

const formatValue = (v: number) => {
  if (v >= 1000000) return (v / 1000000).toFixed(1) + 'M';
  if (v >= 1000) return (v / 1000).toFixed(1) + 'K';
  return v.toFixed(2);
};

const hasTotal = (e: Entitlement) => typeof e.total === 'number' && e.total > 0;

const usagePercent = (e: Entitlement) => {
  if (!e.total || e.total <= 0 || e.remaining == null) return 0;
  return Math.min(100, Math.max(0, ((e.total - e.remaining) / e.total) * 100));
};

// R-5: 百分比型用量（智谱/方舟）的进度条宽度直接取 used_percent
const barWidth = (e: Entitlement): number =>
  e.used_percent != null
    ? Math.min(100, Math.max(0, e.used_percent))
    : usagePercent(e);

// R-12: 进度条颜色统一映射（红/橙/蓝），覆盖到期与百分比两种来源
const barColor = (e: Entitlement): string => {
  if (e.used_percent != null) {
    if (e.used_percent >= 95) return 'bg-red-500';
    if (e.used_percent >= 80) return 'bg-orange-500';
    return 'bg-blue-500';
  }
  switch (entitlementBadge(e)) {
    case 'red': return 'bg-red-500';
    case 'orange': return 'bg-orange-500';
    default: return 'bg-blue-500';
  }
};

// R-5: 配额窗口重置倒计时（expires_at 在百分比型语义下为重置时刻）
function resetText(ts: number): string {
  const ms = ts * 1000 - Date.now();
  if (ms <= 0) return '即将重置';
  const h = Math.floor(ms / 3600000);
  const m = Math.floor((ms % 3600000) / 60000);
  return h > 0 ? `${h}h${m}m 后重置` : `${m}m 后重置`;
};

// §2.5 主信息：百分比型优先，其次到期倒计时，最后余额
function mainText(e: Entitlement): string {
  if (e.used_percent != null) return `已用 ${Math.round(e.used_percent)}%`;
  if (e.expires_at != null) {
    const d = daysLeft(e.expires_at);
    if (d > 0) return `还剩 ${d} 天`;
    if (d === 0) return '今天到期';
    return `已过期 ${-d} 天`;
  }
  if (e.remaining != null) {
    return `${formatValue(e.remaining)} ${unitLabel[e.unit]}`;
  }
  return e.label || '额度包';
}

function subText(e: Entitlement): string {
  // R-5: 百分比型显示"重置倒计时 · 窗口名"，而非"到期"
  if (e.used_percent != null) {
    const pctParts: string[] = [];
    if (e.expires_at != null) pctParts.push(resetText(e.expires_at));
    if (e.label) pctParts.push(e.label);
    return pctParts.join(' · ');
  }
  const parts: string[] = [];
  if (e.expires_at != null) {
    parts.push(`${formatDate(e.expires_at)} 到期`);
    if (e.label) parts.push(e.label);
  }
  if (e.expires_at != null && e.remaining != null) {
    parts.push(`余额 ${formatValue(e.remaining)} ${unitLabel[e.unit]}`);
  }
  return parts.join(' · ');
}

const badgeClass = (e: Entitlement) => {
  switch (entitlementBadge(e)) {
    case 'red': return 'text-red-600';
    case 'orange': return 'text-orange-500';
    default: return 'text-gray-800';
  }
};

const lowTag = (e: Entitlement) => !e.expires_at && isLowBalance(e);

// U-20: detect auth failures (401/403) to surface recovery actions
const isAuthError = () => {
  const msg = props.platform.error ?? '';
  return /\b401\b/.test(msg)
    || /\b403\b/.test(msg)
    || /unauthorized|invalid.*key|key.*invalid|认证|凭证|鉴权/i.test(msg);
};

const isManual = () => props.platform.source === 'manual';

// P1-1: window.open 在 Tauri webview 中静默无效，必须走 opener 插件
const openConsole = () => {
  if (props.platform.console_url) {
    openUrl(props.platform.console_url);
  }
};

const anyLow = () => props.platform.entitlements.some(isLowBalance);
</script>

<template>
  <div class="bg-white rounded-xl border border-neutral-300 p-3 shadow-md">
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center gap-2">
        <span class="font-semibold text-sm text-neutral-900">{{ platform.display_name }}</span>
        <span
          v-if="isManual()"
          class="text-xs px-2 py-0.5 rounded-full bg-amber-100 text-amber-800 border border-amber-300 font-medium"
        >
          手动
        </span>
      </div>
      <div class="flex items-center gap-1">
        <button
          v-if="!isManual()"
          @click="emit('retry')"
          class="text-gray-400 hover:text-blue-500 text-sm"
          title="刷新"
        >
          ↻
        </button>
        <button
          v-if="isManual()"
          @click="emit('edit', platform.id)"
          class="text-gray-400 hover:text-blue-500 text-sm"
          title="编辑"
        >
          ✎
        </button>
        <button
          @click="emit('delete')"
          class="text-gray-400 hover:text-red-500 text-sm"
          title="删除"
        >
          ✕
        </button>
      </div>
    </div>

    <!-- 平台级错误（查询失败 / Key 失效） -->
    <div v-if="platform.error" class="space-y-2">
      <p class="text-red-600 text-xs font-medium">{{ platform.error }}</p>
      <div class="flex flex-wrap gap-2">
        <button
          v-if="isAuthError()"
          @click="emit('reconfigure', platform.id)"
          class="text-xs px-2 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition font-medium"
        >
          重新配置 Key
        </button>
        <button
          @click="emit('retry')"
          class="text-xs px-2 py-1 bg-neutral-200 text-neutral-800 rounded hover:bg-neutral-300 transition font-medium"
        >
          重试
        </button>
      </div>
    </div>

    <!-- 额度包列表：到期型 / 余额型 / 双信息（§2.5） -->
    <div v-else class="space-y-2">
      <div v-for="(e, i) in platform.entitlements" :key="i" class="space-y-1">
        <div class="flex justify-between items-baseline text-xs">
          <span class="font-semibold" :class="badgeClass(e)">{{ mainText(e) }}</span>
          <span v-if="lowTag(e)" class="text-orange-600 font-semibold">低额度</span>
        </div>
        <p v-if="subText(e)" class="text-xs text-neutral-600">{{ subText(e) }}</p>
        <!-- 进度条：total 已知，或百分比型用量（R-5） -->
        <div v-if="hasTotal(e) || e.used_percent != null" class="w-full h-2 bg-neutral-200 rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-300"
            :class="barColor(e)"
            :style="{ width: barWidth(e) + '%' }"
          />
        </div>
        <p v-if="e.note" class="text-xs text-neutral-500 italic">≈ {{ e.note }}</p>
      </div>

      <button
        v-if="anyLow() && platform.console_url"
        @click="openConsole"
        class="mt-1 w-full text-xs py-1.5 bg-orange-600 text-white rounded hover:bg-orange-700 transition font-medium"
      >
        立即充值
      </button>
    </div>
  </div>
</template>
