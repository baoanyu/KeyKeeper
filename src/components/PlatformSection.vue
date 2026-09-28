<script setup lang="ts">
import type { PlatformStatus } from '../types';
import QuotaCard from './QuotaCard.vue';
import SkeletonCard from './SkeletonCard.vue';

// 分组区块容器（system_design.md §3.2）：
// 标记项（Manual 源）与普通项（Api 源）各自独立渲染，事件全部透传给 App.vue

defineProps<{
  /** 区块标题，如「手动标记」/「API 平台」 */
  title: string;
  /** 已按 urgency 排序的平台列表 */
  platforms: PlatformStatus[];
  /** 首屏加载状态（仅 platforms 为空时显示骨架屏） */
  loading?: boolean;
  /** 空状态兜底文案（父组件默认隐藏空区块，此处仅防御性渲染） */
  emptyText?: string;
}>();

const emit = defineEmits<{
  delete: [p: PlatformStatus];
  retry: [];
  reconfigure: [id: string];
  edit: [id: string];
}>();

defineSlots<{
  /** 卡片下方扩展位：手动标记区块用于渲染就地编辑表单（§2.6） */
  'below-card'?: (props: { platform: PlatformStatus }) => any;
}>();
</script>

<template>
  <div>
    <!-- 区块标题 -->
    <h2 class="text-xs font-semibold text-neutral-500 uppercase tracking-wider mb-2 flex items-center gap-1">
      {{ title }}
      <span v-if="platforms.length > 0" class="text-neutral-400 font-normal">({{ platforms.length }})</span>
    </h2>

    <!-- 首屏骨架屏 -->
    <div v-if="loading && platforms.length === 0" class="space-y-3">
      <SkeletonCard />
      <SkeletonCard />
    </div>

    <!-- 空状态兜底（父组件通过 v-if 隐藏空区块，此分支仅防御性渲染） -->
    <p v-else-if="platforms.length === 0" class="text-center text-xs text-neutral-400 py-4">
      {{ emptyText || '暂无平台' }}
    </p>

    <!-- 卡片列表 + 过渡动画：每个平台一个容器，卡片下方可插入就地编辑表单；
         relative 供 leave 过渡的绝对定位元素参照 -->
    <TransitionGroup v-else tag="div" class="space-y-3 relative" name="list">
      <div v-for="p in platforms" :key="p.id" class="space-y-3">
        <QuotaCard
          :platform="p"
          @delete="emit('delete', p)"
          @retry="emit('retry')"
          @reconfigure="emit('reconfigure', p.id)"
          @edit="emit('edit', p.id)"
        />
        <slot name="below-card" :platform="p" />
      </div>
    </TransitionGroup>
  </div>
</template>
