<script setup lang="ts">
defineProps<{
  lastUpdated: string;
  loading: boolean;
}>();
defineEmits<{ refresh: [] }>();
</script>

<template>
  <footer class="flex items-center justify-between px-4 py-3 border-t border-gray-200 bg-white/50">
    <span class="text-xs text-gray-400">
      {{ lastUpdated ? `上次更新: ${lastUpdated}` : '尚未刷新' }}
    </span>
    <button
      @click="$emit('refresh')"
      :disabled="loading"
      class="text-sm text-blue-600 hover:text-blue-800 disabled:text-gray-400 transition"
    >
      {{ loading ? '刷新中...' : '一键刷新' }}
    </button>
  </footer>
</template>
