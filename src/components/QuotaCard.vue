<script setup lang="ts">
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

// §2.5 主信息：有 expires_at 显示倒计时，否则显示余额
function mainText(e: Entitlement): string {
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

const openConsole = () => {
  if (props.platform.console_url) {
    window.open(props.platform.console_url, '_blank', 'noopener,noreferrer');
  }
};

const anyLow = () => props.platform.entitlements.some(isLowBalance);
</script>

<template>
  <div class="bg-white rounded-lg border border-gray-200 p-3 shadow-sm">
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center gap-2">
        <span class="font-medium text-sm">{{ platform.display_name }}</span>
        <span
          v-if="isManual()"
          class="text-xs px-2 py-0.5 rounded-full bg-amber-50 text-amber-600 border border-amber-200"
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
      <p class="text-red-500 text-xs">{{ platform.error }}</p>
      <div class="flex flex-wrap gap-2">
        <button
          v-if="isAuthError()"
          @click="emit('reconfigure', platform.id)"
          class="text-xs px-2 py-1 bg-blue-50 text-blue-600 rounded hover:bg-blue-100 transition"
        >
          重新配置 Key
        </button>
        <button
          @click="emit('retry')"
          class="text-xs px-2 py-1 bg-gray-100 text-gray-600 rounded hover:bg-gray-200 transition"
        >
          重试
        </button>
      </div>
    </div>

    <!-- 额度包列表：到期型 / 余额型 / 双信息（§2.5） -->
    <div v-else class="space-y-2">
      <div v-for="(e, i) in platform.entitlements" :key="i" class="space-y-1">
        <div class="flex justify-between items-baseline text-xs">
          <span class="font-medium" :class="badgeClass(e)">{{ mainText(e) }}</span>
          <span v-if="lowTag(e)" class="text-orange-500 font-medium">低额度</span>
        </div>
        <p v-if="subText(e)" class="text-xs text-gray-500">{{ subText(e) }}</p>
        <!-- 进度条：total 已知时 -->
        <div v-if="hasTotal(e)" class="w-full h-2 bg-gray-100 rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-300"
            :class="entitlementBadge(e) ? 'bg-orange-500' : 'bg-blue-500'"
            :style="{ width: usagePercent(e) + '%' }"
          />
        </div>
        <p v-if="e.note" class="text-xs text-gray-400 italic">≈ {{ e.note }}</p>
      </div>

      <button
        v-if="anyLow() && platform.console_url"
        @click="openConsole"
        class="mt-1 w-full text-xs py-1.5 bg-orange-50 text-orange-600 rounded hover:bg-orange-100 transition"
      >
        立即充值
      </button>
    </div>
  </div>
</template>
