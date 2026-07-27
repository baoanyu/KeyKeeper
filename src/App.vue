<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import QuotaCard from './components/QuotaCard.vue';
import AddProviderForm from './components/AddProviderForm.vue';
import RefreshBar from './components/RefreshBar.vue';
import type { QuotaInfo } from './types';

const quotas = ref<QuotaInfo[]>([]);
const loading = ref(false);
const lastUpdated = ref<string>('');
const error = ref<string>('');
const success = ref<string>('');

let successTimer: number | null = null;
let errorTimer: number | null = null;

function showSuccess(msg: string) {
  success.value = msg;
  if (successTimer !== null) clearTimeout(successTimer);
  successTimer = window.setTimeout(() => {
    success.value = '';
    successTimer = null;
  }, 3000);
}

function showError(msg: string) {
  error.value = msg;
  if (errorTimer !== null) clearTimeout(errorTimer);
  errorTimer = window.setTimeout(() => {
    error.value = '';
    errorTimer = null;
  }, 5000);
}

async function refresh() {
  loading.value = true;
  error.value = '';
  try {
    const result = await invoke<QuotaInfo[]>('get_all_quotas');
    quotas.value = result;
    lastUpdated.value = new Date().toLocaleTimeString();
  } catch (e) {
    showError(String(e));
  } finally {
    loading.value = false;
  }
}

async function addProvider(provider: string, key: string) {
  try {
    await invoke('save_provider_key', { provider, key });
    await invoke('add_provider', { provider });
    await refresh();
    showSuccess(`已添加 ${provider}`);
  } catch (e) {
    showError(`添加失败: ${String(e)}`);
  }
}

async function deleteProvider(provider: string) {
  try {
    await invoke('delete_provider', { provider });
    await refresh();
    showSuccess(`已删除 ${provider}`);
  } catch (e) {
    showError(`删除失败: ${String(e)}`);
  }
}

onMounted(async () => {
  await refresh();
  
  // Listen for auto-refresh events
  await listen('auto-refresh', () => {
    refresh();
  });
});

</script>

<template>
  <div class="h-screen w-[400px] flex flex-col bg-white/80 backdrop-blur-lg text-gray-800">
    <!-- Top Bar -->
    <header class="flex items-center justify-between px-4 py-3 border-b border-gray-200">
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center">
          <span class="text-white text-sm font-bold">K</span>
        </div>
        <h1 class="text-lg font-semibold">KeyKeeper</h1>
      </div>
    </header>

    <!-- Success/Error Messages -->
    <div v-if="success" class="px-4 py-2 bg-green-100 text-green-700 text-sm">
      {{ success }}
    </div>
    <div v-if="error" class="px-4 py-2 bg-red-100 text-red-700 text-sm">
      {{ error }}
    </div>

    <!-- Add Provider Form -->
    <div class="px-4 py-3 border-b border-gray-200">
      <AddProviderForm @add="addProvider" />
    </div>

    <!-- Quota List -->
    <div class="flex-1 overflow-y-auto px-4 py-3 space-y-3">
      <div v-if="loading" class="text-center text-gray-500 py-8">
        加载中...
      </div>
      <div v-else-if="quotas.length === 0" class="text-center text-gray-400 py-8">
        还没有添加平台，请在上方添加
      </div>
      <QuotaCard
        v-for="q in quotas"
        :key="q.provider_name"
        :quota="q"
        @delete="deleteProvider(q.provider_name)"
      />
    </div>

    <!-- Refresh Bar -->
    <RefreshBar :last-updated="lastUpdated" :loading="loading" @refresh="refresh" />
  </div>
</template>
