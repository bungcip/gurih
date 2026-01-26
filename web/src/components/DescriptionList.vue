<script setup>
import { computed } from 'vue'
import StatusBadge from './StatusBadge.vue'

const props = defineProps({
  title: {
    type: String,
    default: ''
  },
  items: {
    type: Array,
    default: () => []
  },
  columns: {
    type: Number,
    default: 1,
    validator: (val) => [1, 2, 3].includes(val)
  },
  loading: {
    type: Boolean,
    default: false
  },
  emptyText: {
    type: String,
    default: 'No details available'
  },
  error: {
    type: String,
    default: ''
  }
})

// Formatters
const numberFormatters = new Map()
const dateFormatters = new Map()

const formatCurrency = (value) => {
  if (value === null || value === undefined) return '-'
  const key = 'id-ID-IDR'

  if (!numberFormatters.has(key)) {
    numberFormatters.set(key, new Intl.NumberFormat('id-ID', {
      style: 'currency',
      currency: 'IDR',
      minimumFractionDigits: 0
    }))
  }

  return numberFormatters.get(key).format(value)
}

const formatDate = (value) => {
  if (!value) return '-'
  const key = 'en-GB'

  if (!dateFormatters.has(key)) {
    dateFormatters.set(key, new Intl.DateTimeFormat('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric'
    }))
  }

  return dateFormatters.get(key).format(new Date(value))
}

// Grid Classes
const gridColsClass = computed(() => {
  const map = {
    1: 'grid-cols-1',
    2: 'grid-cols-1 sm:grid-cols-2',
    3: 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-3'
  }
  return map[props.columns]
})
</script>

<template>
  <div class="bg-[--color-surface] shadow rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
    <!-- Header -->
    <div v-if="title" class="px-4 py-4 sm:px-6 border-b border-gray-100 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/50 flex justify-between items-center">
      <h3 class="text-base font-semibold leading-6 text-text-main">{{ title }}</h3>
      <slot name="header-action"></slot>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="px-4 py-5 sm:p-6 animate-pulse">
      <dl class="grid gap-x-4 gap-y-8" :class="gridColsClass">
        <div v-for="i in (columns * 2)" :key="i" class="sm:col-span-1">
          <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/3 mb-2"></div>
          <div class="h-5 bg-gray-200 dark:bg-gray-700 rounded w-2/3"></div>
        </div>
      </dl>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="px-4 py-12 text-center text-red-600 dark:text-red-400 bg-red-50/30 dark:bg-red-900/30 border-red-100 dark:border-red-800 border rounded-md m-4">
      <p class="text-sm font-medium">{{ error }}</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="items.length === 0" class="px-4 py-12 text-center text-text-muted bg-gray-50/30 dark:bg-gray-800/30">
      <p>{{ emptyText }}</p>
    </div>

    <!-- Content -->
    <div v-else class="px-4 py-5 sm:p-6">
      <dl class="grid gap-x-4 gap-y-8" :class="gridColsClass">
        <div
          v-for="(item, index) in items"
          :key="index"
          class="col-span-1"
          :class="{ 'sm:col-span-2': item.span === 2, 'lg:col-span-3': item.span === 3 }"
        >
          <dt class="text-xs font-medium text-text-muted uppercase tracking-wide">{{ item.label }}</dt>
          <dd class="mt-1 text-sm text-text-main">
            <!-- Slot for Custom Value -->
            <slot v-if="$slots['item-' + item.key]" :name="'item-' + item.key" :item="item" :value="item.value" />

            <!-- Default Renderers -->
            <template v-else>
              <StatusBadge
                v-if="item.type === 'status'"
                :label="item.value"
                :variant="item.variant || 'gray'"
              />

              <span v-else-if="item.type === 'currency'" class="font-mono text-text-main">
                {{ formatCurrency(item.value) }}
              </span>

              <span v-else-if="item.type === 'date'">
                {{ formatDate(item.value) }}
              </span>

              <span v-else-if="item.type === 'code'" class="font-mono bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded text-xs text-gray-700 dark:text-gray-300">
                {{ item.value || '-' }}
              </span>

              <a v-else-if="item.type === 'link'" :href="item.href" class="text-blue-600 dark:text-blue-400 hover:text-blue-800 underline">
                {{ item.value }}
              </a>

              <span v-else class="break-words">
                {{ item.value || '-' }}
              </span>
            </template>
          </dd>
        </div>
      </dl>
    </div>
  </div>
</template>
