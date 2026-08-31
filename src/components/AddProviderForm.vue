<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { PROVIDERS } from '../types';

const props = defineProps<{
  preselectedProvider?: string | null;
  disabled?: boolean;
}>();

const emit = defineEmits<{ add: [provider: string, key: string] }>();

const selectedProvider = ref<(typeof PROVIDERS)[number]>(
  (props.preselectedProvider as (typeof PROVIDERS)[number]) || PROVIDERS[0]
);
const apiKey = ref('');
const showForm = ref(true);

// When parent asks to reconfigure a provider, switch to it and reveal form
watch(
  () => props.preselectedProvider,
  (val) => {
    if (val && (PROVIDERS as readonly string[]).includes(val)) {
      selectedProvider.value = val as (typeof PROVIDERS)[number];
      showForm.value = true;
    }
  }
);

interface ProviderSpec {
  keyPattern: RegExp | null;
  placeholder: string;
  docsUrl: string;
  docsLabel: string;
}

// U-15 / U-16: per-provider key format + docs link
const PROVIDER_SPECS: Record<string, ProviderSpec> = {
  DeepSeek: {
    keyPattern: /^sk-[a-f0-9]{32,}$/,
    placeholder: 'sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx',
    docsUrl: 'https://platform.deepseek.com/api-docs/',
    docsLabel: 'DeepSeek 控制台 → API Key 管理',
  },
  ZhipuAI: {
    keyPattern: /^[a-f0-9]{32}\.[A-Za-z0-9]+$/,
    placeholder: 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.xxxxxxxx',
    docsUrl: 'https://open.bigmodel.cn/usercenter/apikeys',
    docsLabel: '智谱控制台 → API Key 管理',
  },
  Volcano: {
    keyPattern: null,
    placeholder: 'AccessKey:SecretKey',
    docsUrl: 'https://console.volcengine.com/ark/region:ark+cn-beijing',
    docsLabel: '火山方舟控制台 → API Key 管理',
  },
  Qoder: {
    keyPattern: null,
    placeholder: '本地估算模式（无需 Key）',
    docsUrl: 'https://qoder.dev/',
    docsLabel: 'Qoder 官网',
  },
};

const currentSpec = computed(() => PROVIDER_SPECS[selectedProvider.value]);

const keyError = computed(() => {
  const spec = currentSpec.value;
  const key = apiKey.value.trim();
  if (!key) return '';
  if (!spec.keyPattern) return '';
  return spec.keyPattern.test(key) ? '' : 'Key 格式看起来不对';
});

// P3-11: Qoder uses local estimation, no key required
const needsKey = computed(() => selectedProvider.value !== 'Qoder');

const canSubmit = computed(
  () => (!needsKey.value || apiKey.value.trim().length > 0) && !props.disabled
);

function submit() {
  if (!canSubmit.value) return;
  emit('add', selectedProvider.value, apiKey.value.trim());
  apiKey.value = '';
  showForm.value = false;
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
    <div v-else class="space-y-2">
      <div class="flex gap-2">
        <select
          v-model="selectedProvider"
          :disabled="disabled"
          class="flex-1 text-sm border border-gray-300 rounded px-2 py-1.5 bg-white disabled:opacity-50"
        >
          <option v-for="p in PROVIDERS" :key="p" :value="p">{{ p }}</option>
        </select>
      </div>
      <div v-if="needsKey">
        <input
          v-model="apiKey"
          type="password"
          :placeholder="currentSpec.placeholder"
          :disabled="disabled"
          class="w-full text-sm border rounded px-2 py-1.5 disabled:opacity-50"
          :class="keyError ? 'border-red-400 bg-red-50' : 'border-gray-300'"
          @keyup.enter="submit"
        />
        <p v-if="keyError" class="mt-1 text-xs text-red-500">
          {{ keyError }}（仍可添加）
        </p>
        <p v-else class="mt-1 text-xs text-gray-400">
          <a
            :href="currentSpec.docsUrl"
            target="_blank"
            rel="noopener noreferrer"
            class="text-blue-500 hover:underline"
            @click.stop
          >
            ↳ 如何获取？{{ currentSpec.docsLabel }}
          </a>
        </p>
      </div>
      <p v-else class="mt-1 text-xs text-gray-400">
        ℹ️ Qoder 使用本地估算模式，无需填写 API Key。
      </p>
      <div class="flex gap-2">
        <button
          @click="submit"
          :disabled="!canSubmit"
          class="flex-1 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition disabled:bg-gray-300 disabled:cursor-not-allowed"
        >
          添加
        </button>
        <button
          @click="showForm = false"
          :disabled="disabled"
          class="py-1.5 px-3 text-sm text-gray-500 hover:text-gray-700 disabled:opacity-50"
        >
          取消
        </button>
      </div>
      <!-- U-17: privacy notice -->
      <p class="text-xs text-gray-400 leading-relaxed">
        🔒 你的 API Key 存储在 macOS Keychain 中，KeyKeeper 永不上传。
      </p>
    </div>
  </div>
</template>
