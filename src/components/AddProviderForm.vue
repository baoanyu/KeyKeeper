<script setup lang="ts">
import { ref } from 'vue';
import { PROVIDERS } from '../types';

const emit = defineEmits<{ add: [provider: string, key: string] }>();

const selectedProvider = ref(PROVIDERS[0]);
const apiKey = ref('');
const showForm = ref(false);

function submit() {
  if (!apiKey.value.trim()) return;
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
      class="w-full py-2 text-sm text-blue-600 border border-dashed border-blue-300 rounded-lg hover:bg-blue-50 transition"
    >
      + 添加平台
    </button>
    <div v-else class="space-y-2">
      <div class="flex gap-2">
        <select
          v-model="selectedProvider"
          class="flex-1 text-sm border border-gray-300 rounded px-2 py-1.5 bg-white"
        >
          <option v-for="p in PROVIDERS" :key="p" :value="p">{{ p }}</option>
        </select>
      </div>
      <input
        v-model="apiKey"
        type="password"
        placeholder="输入 API Key"
        class="w-full text-sm border border-gray-300 rounded px-2 py-1.5"
        @keyup.enter="submit"
      />
      <div class="flex gap-2">
        <button
          @click="submit"
          class="flex-1 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition"
        >
          添加
        </button>
        <button
          @click="showForm = false"
          class="py-1.5 px-3 text-sm text-gray-500 hover:text-gray-700"
        >
          取消
        </button>
      </div>
    </div>
  </div>
</template>
