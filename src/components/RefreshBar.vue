<script setup lang="ts">
defineProps<{
  lastUpdated: string;
  loading: boolean;
}>();
defineEmits<{ refresh: [] }>();
</script>

<template>
  <footer class="flex items-center justify-between px-4 py-3 border-t border-neutral-300 bg-white">
    <span class="text-xs text-neutral-500 font-medium">
      {{ lastUpdated ? `上次更新: ${lastUpdated}` : '尚未刷新' }}
    </span>
    <button
      @click="$emit('refresh')"
      :disabled="loading"
      class="text-sm px-3 py-1 rounded-lg bg-neutral-900 text-white hover:bg-neutral-700 disabled:opacity-50 disabled:cursor-not-allowed transition font-medium"
    >
      {{ loading ? '刷新中...' : '一键刷新' }}
    </button>
  </footer>
</template>
