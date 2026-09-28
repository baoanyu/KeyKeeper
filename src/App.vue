<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import PlatformSection from './components/PlatformSection.vue';
import AddProviderForm from './components/AddProviderForm.vue';
import ManualEntryForm from './components/ManualEntryForm.vue';
import RefreshBar from './components/RefreshBar.vue';
import type { Entitlement, PlatformSpec, PlatformStatus } from './types';
import { splitPlatforms, errMsg } from './utils';

const specs = ref<PlatformSpec[]>([]);
const platforms = ref<PlatformStatus[]>([]);
const loading = ref(false);
const lastUpdated = ref<string>('');
const error = ref<string>('');
const success = ref<string>('');

// Api Key 验证状态
const verifying = ref(false);
const verifyMessage = ref('');

// P1-4: n 每次递增 —— 同一平台重复点「重新配置」也能触发表单切换
const reconfigure = ref<{ id: string; n: number } | null>(null);
let reconfigureCount = 0;

// 手动录入就地编辑（§2.6）
const editingId = ref<string | null>(null);
const editInitial = ref<Entitlement[]>([]);

// 验证成功后递增，强制重建添加表单以清空状态
const formKey = ref(0);

// 标记项/普通项分离（system_design.md §1.1 D1）：组内独立 urgency 排序，不跨组混排
const groupedPlatforms = computed(() => splitPlatforms(platforms.value));
const manualPlatforms = computed(() => groupedPlatforms.value.manual);
const apiPlatforms = computed(() => groupedPlatforms.value.api);
const configuredIds = computed(() => platforms.value.map((p) => p.id));

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

async function refresh(skipUnchanged = false) {
  if (loading.value) return; // debounce: ignore concurrent refresh requests
  if (verifying.value) return; // R-16: Key 验证期间跳过自动刷新，避免并发交叉写入
  loading.value = true;
  error.value = '';
  try {
    const newPlatforms = await invoke<PlatformStatus[]>('get_all_platforms');
    // P-03: skipUnchanged 时仅在数据变化时赋值，消除 TransitionGroup 无意义 move 动画
    if (!skipUnchanged || platformsHaveChanged(platforms.value, newPlatforms)) {
      platforms.value = newPlatforms;
    }
    lastUpdated.value = new Date().toLocaleTimeString();
  } catch (e) {
    showError(errMsg(e));
  } finally {
    loading.value = false;
  }
}

// Api 平台：验证 Key 有效后才保留（F1: 失败时恢复旧 Key）
async function addApi(id: string, key: string) {
  verifying.value = true;
  verifyMessage.value = '正在验证 Key...';

  let oldKey: string | null = null;
  let hadKey = false;
  try {
    oldKey = await invoke<string>('get_api_key', { id });
    hadKey = true;
  } catch {
    hadKey = false;
  }

  try {
    await invoke('save_api_key', { id, key });
    const result = await invoke<PlatformStatus[]>('get_all_platforms');
    const match = result.find((p) => p.id === id);
    if (match && !match.error) {
      platforms.value = result;
      lastUpdated.value = new Date().toLocaleTimeString();
      formKey.value++; // P1-2: 成功才重置表单
      reconfigure.value = null;
      showSuccess(`已保存 ${match.display_name}`);
    } else {
      if (hadKey && oldKey) {
        await invoke('save_api_key', { id, key: oldKey });
      } else {
        await invoke('delete_platform', { id });
      }
      // P1-2: 不重置表单 —— Key 留在输入框供用户修改
      showError(`Key 验证失败：${match?.error || '查询无结果'}`);
    }
  } catch (e) {
    if (hadKey && oldKey) {
      await invoke('save_api_key', { id, key: oldKey });
    } else {
      try { await invoke('delete_platform', { id }); } catch { /* 恢复失败不掩盖原始错误 */ }
    }
    showError(`Key 验证失败：${errMsg(e)}`);
  } finally {
    verifying.value = false;
    verifyMessage.value = '';
  }
}

// Manual 平台：新增录入
async function addManual(id: string, entitlements: Entitlement[]) {
  try {
    await invoke('save_manual_platform', { id, entitlements });
    formKey.value++;
    await refresh();
    showSuccess('已保存到期记录');
  } catch (e) {
    showError(`保存失败: ${errMsg(e)}`);
  }
}

// Manual 平台：就地编辑
async function startEdit(id: string) {
  try {
    editInitial.value = await invoke<Entitlement[]>('get_manual_platform', { id });
    editingId.value = id;
    reconfigure.value = null;
  } catch (e) {
    showError(`加载数据失败: ${errMsg(e)}`);
  }
}

async function saveEdit(id: string, entitlements: Entitlement[]) {
  try {
    await invoke('save_manual_platform', { id, entitlements });
    editingId.value = null;
    await refresh();
    showSuccess('已更新到期记录');
  } catch (e) {
    showError(`保存失败: ${errMsg(e)}`);
  }
}

async function deletePlatform(p: PlatformStatus) {
  try {
    await invoke('delete_platform', { id: p.id });
    if (editingId.value === p.id) editingId.value = null;
    await refresh();
    showSuccess(`已删除 ${p.display_name}`);
  } catch (e) {
    showError(`删除失败: ${errMsg(e)}`);
  }
}

function startReconfigure(id: string) {
  reconfigureCount += 1;
  reconfigure.value = { id, n: reconfigureCount };
  editingId.value = null;
}

// P-03: 比对平台列表是否有实质变化（updated_at / entitlements 内容）
function platformsHaveChanged(oldList: PlatformStatus[], newList: PlatformStatus[]): boolean {
  if (oldList.length !== newList.length) return true;
  const oldMap = new Map(oldList.map(p => [p.id, p]));
  for (const newP of newList) {
    const oldP = oldMap.get(newP.id);
    if (!oldP) return true;
    if (oldP.updated_at !== newP.updated_at) return true;
    if (oldP.error !== newP.error) return true;
    if (oldP.entitlements.length !== newP.entitlements.length) return true;
  }
  return false;
}

let unlistenAutoRefresh: (() => void) | undefined;

onMounted(async () => {
  // P-01: specs 与 platforms 并行加载（无数据依赖）
  const [specsResult] = await Promise.all([
    invoke<PlatformSpec[]>('get_platform_specs').catch((e) => {
      showError(errMsg(e));
      return [] as PlatformSpec[];
    }),
    refresh(),
  ]);
  specs.value = specsResult;

  // P-04: 保存 UnlistenFn 以便后续清理
  unlistenAutoRefresh = await listen('auto-refresh', () => {
    refresh(true); // auto-refresh 使用 skipUnchanged 短路
  });
});

// P-04: 清理事件监听器与消息计时器
onBeforeUnmount(() => {
  unlistenAutoRefresh?.();
  if (successTimer !== null) clearTimeout(successTimer);
  if (errorTimer !== null) clearTimeout(errorTimer);
});
</script>

<template>
  <div class="h-screen w-full flex flex-col bg-neutral-100 text-neutral-900">
    <!-- Top Bar（可拖拽窗口，§1.1） · 深色高对比顶栏 -->
    <header data-tauri-drag-region class="flex items-center justify-between px-4 py-3 bg-neutral-900">
      <div class="flex items-center gap-2">
        <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center shadow">
          <span class="text-white text-sm font-bold">K</span>
        </div>
        <h1 class="text-lg font-bold text-white tracking-tight">KeyKeeper</h1>
      </div>
    </header>

    <!-- Success/Error Messages · 实色高对比 + 淡入淡出 -->
    <Transition name="fade">
      <div v-if="success" class="px-4 py-2 bg-green-600 text-white text-sm font-medium">
        {{ success }}
      </div>
    </Transition>
    <Transition name="fade">
      <div v-if="error" class="px-4 py-2 bg-red-600 text-white text-sm font-medium">
        {{ error }}
      </div>
    </Transition>

    <!-- Add Platform Form -->
    <div class="px-4 py-3 bg-white border-b border-neutral-300">
      <AddProviderForm
        :key="formKey"
        :specs="specs"
        :configured-ids="configuredIds"
        :preselected="reconfigure"
        :disabled="verifying"
        @add-api="addApi"
        @save-manual="addManual"
        @cancel-reconfigure="reconfigure = null"
      />
      <p v-if="verifying" class="mt-1 text-xs text-blue-500">
        {{ verifyMessage }}
      </p>
    </div>

    <!-- Platform List：标记项（Manual 源）与普通项（Api 源）分区渲染 -->
    <div class="flex-1 overflow-y-auto px-4 py-3 space-y-4">
      <!-- 标记项区块：手动录入到期数据的平台 -->
      <PlatformSection
        v-if="manualPlatforms.length > 0 || loading"
        title="手动标记"
        :platforms="manualPlatforms"
        :loading="loading && platforms.length === 0"
        empty-text="暂无手动标记的平台"
        @delete="deletePlatform"
        @retry="refresh"
        @reconfigure="startReconfigure"
        @edit="startEdit"
      >
        <!-- 手动录入就地展开编辑表单（§2.6） -->
        <template #below-card="{ platform }">
          <div v-if="editingId === platform.id" class="bg-amber-50/60 border border-amber-200 rounded-lg p-3">
            <p class="text-xs text-amber-700 mb-2">编辑 {{ platform.display_name }} 的到期记录</p>
            <ManualEntryForm
              :initial="editInitial"
              @save="saveEdit(platform.id, $event)"
              @cancel="editingId = null"
            />
          </div>
        </template>
      </PlatformSection>

      <!-- 普通项区块：自动拉取额度的平台 -->
      <PlatformSection
        v-if="apiPlatforms.length > 0 || loading"
        title="API 平台"
        :platforms="apiPlatforms"
        :loading="loading && platforms.length === 0"
        empty-text="暂无 API 平台"
        @delete="deletePlatform"
        @retry="refresh"
        @reconfigure="startReconfigure"
        @edit="startEdit"
      />

      <!-- 全局空状态 -->
      <div v-if="!loading && platforms.length === 0" class="text-center py-12">
        <div class="inline-flex items-center justify-center w-16 h-16 rounded-full bg-neutral-200 mb-4">
          <span class="text-2xl">🔑</span>
        </div>
        <p class="text-neutral-600 font-medium">还没有添加平台</p>
        <p class="text-neutral-400 text-sm mt-1">点击上方「添加平台」开始管理你的额度</p>
      </div>
    </div>

    <!-- Refresh Bar -->
    <RefreshBar :last-updated="lastUpdated" :loading="loading" @refresh="refresh" />
  </div>
</template>
