<!-- 骨架屏卡片：首屏加载占位，尺寸与 QuotaCard 对齐（system_design.md §1.1 D3） -->
<template>
  <div class="bg-white rounded-xl border border-neutral-300 p-3 shadow-md animate-pulse">
    <!-- 标题行：圆形占位 + 文本占位 -->
    <div class="flex items-center justify-between mb-3">
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 rounded-full bg-neutral-300"></div>
        <div class="h-3 w-20 bg-neutral-300 rounded"></div>
      </div>
      <div class="h-3 w-10 bg-neutral-200 rounded"></div>
    </div>
    <!-- 额度包占位：文本行 + 进度条 ×2 -->
    <div class="space-y-2">
      <div class="flex justify-between">
        <div class="h-3 w-16 bg-neutral-200 rounded"></div>
        <div class="h-3 w-12 bg-neutral-200 rounded"></div>
      </div>
      <div class="h-2 w-full bg-neutral-200 rounded-full"></div>
      <div class="flex justify-between">
        <div class="h-3 w-24 bg-neutral-200 rounded"></div>
        <div class="h-3 w-8 bg-neutral-200 rounded"></div>
      </div>
      <div class="h-2 w-full bg-neutral-200 rounded-full"></div>
    </div>
  </div>
</template>
