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
      class="text-sm px-3 py-1 rounded-lg bg-neutral-900 text-white hover:bg-neutral-700 disabled:opacity-50 disabled:cursor-not-allowed transition font-medium active:scale-95"
    >
      <span class="flex items-center gap-1.5">
        <svg v-if="loading" class="animate-spin h-3.5 w-3.5" viewBox="0 0 24 24" fill="none">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3" />
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v3a5 5 0 00-5 5H4z" />
        </svg>
        {{ loading ? '刷新中' : '一键刷新' }}
      </span>
    </button>
  </footer>
</template>
