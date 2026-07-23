<script setup lang="ts">
import type { QuotaInfo } from '../types';

const props = defineProps<{ quota: QuotaInfo }>();
const emit = defineEmits<{ delete: [] }>();

const usagePercent = () => {
  if (props.quota.total <= 0) return 0;
  return Math.min(100, Math.max(0, ((props.quota.total - props.quota.remaining) / props.quota.total) * 100));
};

const formatValue = (v: number) => {
  if (v >= 1000000) return (v / 1000000).toFixed(1) + 'M';
  if (v >= 1000) return (v / 1000).toFixed(1) + 'K';
  return v.toFixed(2);
};

const isLow = () => {
  const threshold = props.quota.quota_unit === 'cny' ? 10 : 1000;
  return props.quota.is_success && props.quota.remaining < threshold;
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
      <button
        @click="emit('delete')"
        class="text-gray-400 hover:text-red-500 text-sm"
        title="Delete"
      >
        ✕
      </button>
    </div>

    <div v-if="!quota.is_success" class="text-red-500 text-xs">
      {{ quota.error_msg || 'Failed to fetch' }}
    </div>
    <div v-else>
      <div class="flex justify-between text-xs text-gray-500 mb-1">
        <span>剩余: {{ formatValue(quota.remaining) }} {{ unitLabel[quota.quota_unit] }}</span>
        <span v-if="isLow()" class="text-orange-500 font-medium">低额度</span>
      </div>
      <div class="w-full h-2 bg-gray-100 rounded-full overflow-hidden">
        <div
          class="h-full rounded-full transition-all duration-300"
          :class="isLow() ? 'bg-orange-500' : 'bg-blue-500'"
          :style="{ width: usagePercent() + '%' }"
        />
      </div>
    </div>
  </div>
</template>
