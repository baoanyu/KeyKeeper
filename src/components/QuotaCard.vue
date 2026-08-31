<script setup lang="ts">
import type { QuotaInfo } from '../types';

const props = defineProps<{ quota: QuotaInfo }>();
const emit = defineEmits<{ delete: []; retry: []; reconfigure: [provider: string] }>();

// F8: total may be undefined for wallet-style providers
const hasTotal = () => typeof props.quota.total === 'number' && props.quota.total > 0;

const usagePercent = () => {
  const total = props.quota.total;
  if (!total || total <= 0) return 0;
  return Math.min(100, Math.max(0, ((total - props.quota.remaining) / total) * 100));
};

const formatValue = (v: number) => {
  if (v >= 1000000) return (v / 1000000).toFixed(1) + 'M';
  if (v >= 1000) return (v / 1000).toFixed(1) + 'K';
  return v.toFixed(2);
};

// F6: thresholds must match backend check_low_balance (main.rs)
const isLow = () => {
  const threshold = (() => {
    switch (props.quota.quota_unit) {
      case 'cny': return 10;
      case 'tokens': return 1000;
      case 'seconds': return 600; // 10 minutes, matches backend
      default: return 1000;
    }
  })();
  return props.quota.is_success && props.quota.remaining < threshold;
};

// U-20: detect auth failures (401/403) to surface recovery actions
// F13: expanded pattern set — catch more auth error phrasings
const isAuthError = () => {
  if (props.quota.is_success) return false;
  const msg = props.quota.error_msg ?? '';
  return /\b401\b/.test(msg)
    || /\b403\b/.test(msg)
    || /unauthorized|invalid.*key|key.*invalid|认证|凭证|鉴权/i.test(msg);
};

const planLabel: Record<string, string> = {
  'pay_as_you_go': '按量付费',
  'coding_plan': '订阅计划',
  'subscription': '订阅',
};

const unitLabel: Record<string, string> = {
  'cny': '元',
  'tokens': 'tokens',
  'seconds': '秒',
  'unknown': '',
};

// U-7: per-provider top-up / console URLs (moved to PlatformSpec in P2-8)
const CONSOLE_URLS: Record<string, string> = {
  DeepSeek: 'https://platform.deepseek.com/api-docs/',
  ZhipuAI: 'https://open.bigmodel.cn/usercenter/apikeys',
  Volcano: 'https://console.volcengine.com/ark/region:ark+cn-beijing',
  Qoder: 'https://qoder.dev/',
};

const openConsole = () => {
  const url = CONSOLE_URLS[props.quota.provider_name];
  if (url) window.open(url, '_blank', 'noopener,noreferrer');
};
</script>

<template>
  <div class="bg-white rounded-lg border border-gray-200 p-3 shadow-sm">
    <div class="flex items-center justify-between mb-2">
      <div class="flex items-center gap-2">
        <span class="font-medium text-sm">{{ quota.provider_name }}</span>
        <span class="text-xs px-2 py-0.5 rounded-full bg-gray-100 text-gray-600">
          {{ planLabel[quota.plan_type] || quota.plan_type }}
        </span>
      </div>
      <div class="flex items-center gap-1">
        <!-- U-21: single-card retry -->
        <button
          @click="emit('retry')"
          class="text-gray-400 hover:text-blue-500 text-sm"
          title="刷新"
        >
          ↻
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

    <div v-if="!quota.is_success" class="space-y-2">
      <p class="text-red-500 text-xs">
        {{ quota.error_msg || '获取失败' }}
      </p>
      <!-- U-20: error recovery actions -->
      <div class="flex flex-wrap gap-2">
        <button
          v-if="isAuthError()"
          @click="emit('reconfigure', quota.provider_name)"
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
    <div v-else>
      <div class="flex justify-between text-xs text-gray-500 mb-1">
        <span>剩余: {{ formatValue(quota.remaining) }} {{ unitLabel[quota.quota_unit] }}</span>
        <span v-if="isLow()" class="text-orange-500 font-medium">低额度</span>
      </div>
      <!-- F7: show estimate disclaimer when present (e.g. Qoder) -->
      <p v-if="quota.error_msg" class="text-xs text-gray-400 italic mb-1">
        ≈ {{ quota.error_msg }}
      </p>
      <!-- F8: only render progress bar when total is known and positive -->
      <div v-if="hasTotal()" class="w-full h-2 bg-gray-100 rounded-full overflow-hidden">
        <div
          class="h-full rounded-full transition-all duration-300"
          :class="isLow() ? 'bg-orange-500' : 'bg-blue-500'"
          :style="{ width: usagePercent() + '%' }"
        />
      </div>
      <!-- U-7: top-up button when low balance (skip for Qoder — no console to top up) -->
      <button
        v-if="isLow() && quota.provider_name !== 'Qoder'"
        @click="openConsole"
        class="mt-2 w-full text-xs py-1.5 bg-orange-50 text-orange-600 rounded hover:bg-orange-100 transition"
      >
        立即充值
      </button>
    </div>
  </div>
</template>
