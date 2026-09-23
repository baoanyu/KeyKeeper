<script setup lang="ts">
import { ref } from 'vue';
import type { Entitlement } from '../types';
import { dateInputInDays, dateInputToTimestamp, timestampToDateInput } from '../utils';

// §2.6 手动录入交互：额度包名（可选）+ 到期日 + 「+30 天」「+7 天」快捷
// 新增与编辑共用本组件 —— 初始为空行 / 预填现有额度包

interface Row {
  label: string;
  date: string; // YYYY-MM-DD，date input 的值
}

const props = defineProps<{ initial?: Entitlement[] }>();
const emit = defineEmits<{ save: [Entitlement[]]; cancel: [] }>();

function toRow(e: Entitlement): Row {
  return {
    label: e.label,
    date: e.expires_at != null ? timestampToDateInput(e.expires_at) : '',
  };
}

const rows = ref<Row[]>(
  props.initial && props.initial.length > 0
    ? props.initial.map(toRow)
    : [{ label: '', date: '' }]
);

function addRow() {
  rows.value.push({ label: '', date: '' });
}

function removeRow(index: number) {
  rows.value.splice(index, 1);
  if (rows.value.length === 0) rows.value.push({ label: '', date: '' });
}

const rowError = (row: Row) => (!row.date ? '需要到期日' : '');

function submit() {
  // 丢弃完全空白的行；其余行必须有到期日
  const valid = rows.value.filter((r) => r.label.trim() || r.date);
  if (valid.length === 0) return;
  if (valid.some(rowError)) return;

  const entitlements: Entitlement[] = valid.map((r) => ({
    label: r.label.trim(),
    expires_at: dateInputToTimestamp(r.date),
    unit: 'unknown',
    total: null,
    remaining: null,
    note: null,
    used_percent: null,
  }));
  emit('save', entitlements);
}
</script>

<template>
  <div class="space-y-2">
    <div
      v-for="(row, i) in rows"
      :key="i"
      class="flex items-start gap-1"
    >
      <div class="flex-1 space-y-1">
        <div class="flex gap-1">
          <input
            v-model="row.label"
            placeholder="额度包名（可选）"
            class="flex-1 min-w-0 text-xs border border-neutral-400 rounded px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          />
          <input
            v-model="row.date"
            type="date"
            class="text-xs border border-neutral-400 rounded px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            :class="rowError(row) ? 'border-red-400 bg-red-50' : 'border-gray-300'"
          />
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="row.date = dateInputInDays(30)"
            class="text-xs px-1.5 py-0.5 bg-neutral-200 text-neutral-800 rounded hover:bg-neutral-300 transition font-medium"
          >
            +30 天
          </button>
          <button
            type="button"
            @click="row.date = dateInputInDays(7)"
            class="text-xs px-1.5 py-0.5 bg-neutral-200 text-neutral-800 rounded hover:bg-neutral-300 transition font-medium"
          >
            +7 天
          </button>
          <span v-if="rowError(row)" class="text-xs text-red-500">{{ rowError(row) }}</span>
        </div>
      </div>
      <button
        type="button"
        @click="removeRow(i)"
        class="mt-1.5 text-gray-400 hover:text-red-500 text-sm"
        title="删除此额度包"
      >
        ✕
      </button>
    </div>

    <button
      type="button"
      @click="addRow"
      class="text-xs text-blue-600 hover:text-blue-800 transition"
    >
      + 添加额度包
    </button>

    <div class="flex gap-2 pt-1">
      <button
        @click="submit"
        class="flex-1 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition"
      >
        保存
      </button>
      <button
        @click="emit('cancel')"
        class="py-1.5 px-3 text-sm text-gray-500 hover:text-gray-700"
      >
        取消
      </button>
    </div>
  </div>
</template>
