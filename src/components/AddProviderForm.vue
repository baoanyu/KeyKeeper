<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { Entitlement, PlatformSpec } from '../types';
import ManualEntryForm from './ManualEntryForm.vue';

// §3.4：平台元数据全部来自 Rust 端 PlatformSpec，本组件不再硬编码
// mode = api → Key 录入；mode = manual → 到期日录入（§2.6）

const props = defineProps<{
  specs: PlatformSpec[];
  /** 当前已有数据的平台 id —— 不在下拉中出现（编辑走卡片入口） */
  configuredIds: string[];
  /** 重新配置场景：强制显示该平台。n 每次递增，保证重复点击同一平台也触发（P1-4） */
  preselected?: { id: string; n: number } | null;
  disabled?: boolean;
}>();

const emit = defineEmits<{
  addApi: [id: string, key: string];
  saveManual: [id: string, entitlements: Entitlement[]];
}>();

const showForm = ref(true);
const selectedId = ref('');
const apiKey = ref('');

// 可选平台 = 未配置的 + 重新配置目标（已配置的 Api 平台可换 Key）
const availableSpecs = computed(() =>
  props.specs.filter(
    (s) =>
      !props.configuredIds.includes(s.id) ||
      s.id === props.preselected?.id
  )
);

const currentSpec = computed(
  () => props.specs.find((s) => s.id === selectedId.value) ?? availableSpecs.value[0]
);

// preselected 引用变化（含重复点击同一平台，n 递增）时切换到目标平台并展开表单
watch(
  () => props.preselected,
  (val) => {
    if (val) {
      selectedId.value = val.id;
      showForm.value = true;
    }
  },
  { immediate: true }
);

// 默认选中第一个可选平台
if (!selectedId.value && availableSpecs.value.length > 0) {
  selectedId.value = availableSpecs.value[0].id;
}

const keyError = computed(() => {
  const spec = currentSpec.value;
  const key = apiKey.value.trim();
  if (!key || !spec?.key_pattern) return '';
  try {
    return new RegExp(spec.key_pattern).test(key) ? '' : 'Key 格式看起来不对';
  } catch {
    return '';
  }
});

const canSubmitKey = computed(
  () =>
    currentSpec.value?.mode === 'api' &&
    apiKey.value.trim().length > 0 &&
    !props.disabled
);

function submitKey() {
  if (!canSubmitKey.value || !currentSpec.value) return;
  // P1-2: 不清空 Key —— 由父组件验证成功后重建表单，失败时用户可直接修改
  emit('addApi', currentSpec.value.id, apiKey.value.trim());
}

function submitManual(entitlements: Entitlement[]) {
  if (!currentSpec.value || props.disabled) return;
  emit('saveManual', currentSpec.value.id, entitlements);
}
</script>

<template>
  <div>
    <button
      v-if="!showForm"
      @click="showForm = true"
      :disabled="disabled"
      class="w-full py-2 text-sm text-blue-600 border border-dashed border-blue-300 rounded-lg hover:bg-blue-50 transition disabled:opacity-50 disabled:cursor-not-allowed"
    >
      + 添加平台
    </button>
    <div v-else-if="availableSpecs.length === 0" class="text-xs text-gray-400 py-1">
      所有平台都已添加
    </div>
    <div v-else class="space-y-2">
      <select
        v-model="selectedId"
        :disabled="disabled || !!preselected"
        class="w-full text-sm border border-gray-300 rounded px-2 py-1.5 bg-white disabled:opacity-50"
      >
        <option v-for="s in availableSpecs" :key="s.id" :value="s.id">
          {{ s.display_name }}{{ s.mode === 'manual' ? '（手动录入）' : '' }}
        </option>
      </select>

      <!-- Api 平台：Key 录入 -->
      <div v-if="currentSpec?.mode === 'api'">
        <input
          v-model="apiKey"
          type="password"
          :placeholder="currentSpec.key_hint"
          :disabled="disabled"
          class="w-full text-sm border rounded px-2 py-1.5 disabled:opacity-50"
          :class="keyError ? 'border-red-400 bg-red-50' : 'border-gray-300'"
          @keyup.enter="submitKey"
        />
        <p v-if="keyError" class="mt-1 text-xs text-red-500">
          {{ keyError }}（仍可添加）
        </p>
        <p v-else-if="currentSpec.key_docs_url" class="mt-1 text-xs text-gray-400">
          <a
            :href="currentSpec.key_docs_url"
            target="_blank"
            rel="noopener noreferrer"
            class="text-blue-500 hover:underline"
            @click.stop
          >
            ↳ 如何获取？{{ currentSpec.display_name }} API Key
          </a>
        </p>
        <div class="flex gap-2 pt-2">
          <button
            @click="submitKey"
            :disabled="!canSubmitKey"
            class="flex-1 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition disabled:bg-gray-300 disabled:cursor-not-allowed"
          >
            {{ preselected ? '更新 Key' : '添加' }}
          </button>
          <button
            v-if="!preselected"
            @click="showForm = false"
            :disabled="disabled"
            class="py-1.5 px-3 text-sm text-gray-500 hover:text-gray-700 disabled:opacity-50"
          >
            取消
          </button>
        </div>
        <!-- U-17: privacy notice -->
        <p class="mt-2 text-xs text-gray-400 leading-relaxed">
          🔒 你的 API Key 存储在 macOS Keychain 中，KeyKeeper 永不上传。
        </p>
      </div>

      <!-- Manual 平台：到期日录入（§2.6） -->
      <div v-else>
        <ManualEntryForm @save="submitManual" @cancel="showForm = false" />
      </div>
    </div>
  </div>
</template>
