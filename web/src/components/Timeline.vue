<script>
// Optimization: Hoisted static variants map to avoid reallocation on every render
const variants = {
  success: 'bg-green-100 text-green-800 border-green-200 ring-green-100 dark:bg-green-900/30 dark:text-green-300 dark:border-green-800 dark:ring-green-900/30',
  warning: 'bg-yellow-100 text-yellow-800 border-yellow-200 ring-yellow-100 dark:bg-yellow-900/30 dark:text-yellow-300 dark:border-yellow-800 dark:ring-yellow-900/30',
  danger: 'bg-red-100 text-red-800 border-red-200 ring-red-100 dark:bg-red-900/30 dark:text-red-300 dark:border-red-800 dark:ring-red-900/30',
  info: 'bg-blue-100 text-blue-800 border-blue-200 ring-blue-100 dark:bg-blue-900/30 dark:text-blue-300 dark:border-blue-800 dark:ring-blue-900/30',
  gray: 'bg-gray-100 text-gray-600 border-gray-200 ring-gray-100 dark:bg-gray-800 dark:text-gray-300 dark:border-gray-700 dark:ring-gray-800'
}
</script>

<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  items: {
    type: Array,
    default: () => []
  },
  loading: {
    type: Boolean,
    default: false
  },
  emptyText: {
    type: String,
    default: 'No history available'
  }
})

const emit = defineEmits(['click-item'])

const getVariantClass = (variant) => variants[variant] || variants['gray']

function onClick(item) {
  emit('click-item', item)
}
</script>

<template>
  <div class="relative">
    <!-- Loading State -->
    <div v-if="loading" class="space-y-6 animate-pulse">
      <div v-for="i in 3" :key="i" class="flex gap-4">
        <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 flex-shrink-0"></div>
        <div class="flex-1 space-y-2 py-1">
          <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4"></div>
          <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
        </div>
      </div>
    </div>

    <!-- Empty State -->
    <div v-else-if="items.length === 0" class="text-center py-8 text-text-muted bg-gray-50 dark:bg-gray-800/50 rounded-lg border border-gray-100 dark:border-gray-700 border-dashed">
      {{ emptyText }}
    </div>

    <!-- Timeline List -->
    <ul v-else class="space-y-0">
      <li v-for="(item, index) in items" :key="index" class="relative flex gap-4 group pb-8 last:pb-0">
        <!-- Connecting Line -->
        <div
          v-if="index !== items.length - 1"
          class="absolute top-8 left-4 w-0.5 h-full -ml-px bg-gray-200 dark:bg-gray-700"
          aria-hidden="true"
        ></div>

        <!-- Icon/Dot -->
        <div
          class="relative flex h-8 w-8 items-center justify-center rounded-full border ring-4 ring-white dark:ring-gray-900 shrink-0 z-10"
          :class="getVariantClass(item.variant)"
        >
          <Icon v-if="item.icon" :name="item.icon" :size="14" />
          <div v-else class="h-2.5 w-2.5 rounded-full bg-current"></div>
        </div>

        <!-- Content -->
        <div class="flex-1 min-w-0 pt-0.5 cursor-default group-hover:bg-gray-50/50 dark:group-hover:bg-gray-800/50 rounded -ml-2 pl-2 -mr-2 pr-2 transition-colors duration-200" @click="onClick(item)">
          <div class="flex justify-between items-start text-sm">
            <h3 class="font-medium text-text-main">{{ item.title }}</h3>
            <time v-if="item.date" class="text-xs text-text-muted whitespace-nowrap pl-4 pt-0.5">{{ item.date }}</time>
          </div>
          <p v-if="item.description" class="mt-1 text-sm text-text-muted">
            {{ item.description }}
          </p>
          <div v-if="$slots.extra" class="mt-2">
             <slot name="extra" :item="item"></slot>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>
