<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { sendNotification } from '@tauri-apps/plugin-notification';
import QuotaCard from './components/QuotaCard.vue';
import AddProviderForm from './components/AddProviderForm.vue';
import RefreshBar from './components/RefreshBar.vue';
import type { QuotaInfo } from './types';

const quotas = ref<QuotaInfo[]>([]);
const loading = ref(false);
const lastUpdated = ref<string>('');
const error = ref<string>('');
const success = ref<string>('');

// U-22: key pre-check state
const verifying = ref(false);
const verifyMessage = ref('');
const reconfigureProvider = ref<string | null>(null);

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

// P2-1: check low balance and fire system notification
async function checkAndNotify(result: QuotaInfo[]) {
  try {
    const lowProviders = await invoke<string[]>('check_low_balance', { quotas: result });
    if (lowProviders.length > 0) {
      sendNotification({
        title: 'KeyKeeper 提醒',
        body: `以下平台余额不足：${lowProviders.join('、')}`,
      });
    }
  } catch (e) {
    console.warn('check_low_balance failed:', e);
  }
}

async function refresh() {
  if (loading.value) return; // debounce: ignore concurrent refresh requests
  loading.value = true;
  error.value = '';
  try {
    const result = await invoke<QuotaInfo[]>('get_all_quotas');
    quotas.value = result;
    lastUpdated.value = new Date().toLocaleTimeString();
    await checkAndNotify(result);
  } catch (e) {
    showError(String(e));
  } finally {
    loading.value = false;
  }
}

// U-21: single-card retry (currently does full refresh; provider param reserved for future per-card refresh)
async function retryProvider(_provider: string) {
  if (loading.value) return; // debounce
  loading.value = true;
  error.value = '';
  try {
    const result = await invoke<QuotaInfo[]>('get_all_quotas');
    quotas.value = result;
    lastUpdated.value = new Date().toLocaleTimeString();
    await checkAndNotify(result);
  } catch (e) {
    showError(String(e));
  } finally {
    loading.value = false;
  }
}

// U-22: validate key before saving
// F1 (P0 fix): preserve old key on reconfigure — restore it if new key fails validation
async function validateAndAddProvider(provider: string, key: string) {
  if (provider === 'Qoder') {
    await addProviderWithoutKey(provider);
    return;
  }

  verifying.value = true;
  verifyMessage.value = '正在验证 Key...';

  // Snapshot the old key so we can restore it on failure
  let oldKey: string | null = null;
  let hadKey = false;
  try {
    oldKey = await invoke<string>('get_provider_key', { provider });
    hadKey = true;
  } catch {
    hadKey = false;
  }

  try {
    await invoke('save_provider_key', { provider, key });
    const result = await invoke<QuotaInfo[]>('get_all_quotas');
    const match = result.find((q) => q.provider_name === provider);
    if (match?.is_success) {
      verifyMessage.value = `✓ Key 有效`;
      quotas.value = result;
      lastUpdated.value = new Date().toLocaleTimeString();
      showSuccess(`已添加 ${provider}`);
    } else {
      // Restore old key or delete if there was none before
      if (hadKey && oldKey) {
        await invoke('save_provider_key', { provider, key: oldKey });
      } else {
        await invoke('delete_provider', { provider });
      }
      verifyMessage.value = `✗ ${match?.error_msg || 'Key 无效'}`;
      showError(`Key 验证失败，已恢复原有 Key`);
    }
  } catch (e) {
    if (hadKey && oldKey) {
      await invoke('save_provider_key', { provider, key: oldKey });
    } else {
      try { await invoke('delete_provider', { provider }); } catch {}
    }
    verifyMessage.value = `✗ ${String(e)}`;
    showError(`Key 验证失败，已恢复原有 Key`);
  } finally {
    verifying.value = false;
    verifyMessage.value = '';
  }
}

// P3-11 / Fix #3: Qoder has no key — skip Keychain write, only register the provider
async function addProviderWithoutKey(provider: string) {
  try {
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

// U-20: reconfigure handler
function startReconfigure(provider: string) {
  reconfigureProvider.value = provider;
}

onMounted(async () => {
  await refresh();

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
      <AddProviderForm
        :preselected-provider="reconfigureProvider"
        :disabled="verifying"
        @add="validateAndAddProvider"
      />
      <p v-if="verifying" class="mt-1 text-xs text-blue-500">
        {{ verifyMessage }}
      </p>
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
        @retry="retryProvider(q.provider_name)"
        @reconfigure="startReconfigure"
      />
    </div>

    <!-- Refresh Bar -->
    <RefreshBar :last-updated="lastUpdated" :loading="loading" @refresh="refresh" />
  </div>
</template>
