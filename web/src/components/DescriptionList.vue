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
const formatCurrency = (value) => {
  if (value === null || value === undefined) return '-'
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0
  }).format(value)
}

const formatDate = (value) => {
  if (!value) return '-'
  return new Date(value).toLocaleDateString('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric'
  })
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
  <div class="bg-white shadow rounded-lg border border-gray-200 overflow-hidden">
    <!-- Header -->
    <div v-if="title" class="px-4 py-4 sm:px-6 border-b border-gray-100 bg-gray-50/50 flex justify-between items-center">
      <h3 class="text-base font-semibold leading-6 text-gray-900">{{ title }}</h3>
      <slot name="header-action"></slot>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="px-4 py-5 sm:p-6 animate-pulse">
      <dl class="grid gap-x-4 gap-y-8" :class="gridColsClass">
        <div v-for="i in (columns * 2)" :key="i" class="sm:col-span-1">
          <div class="h-4 bg-gray-200 rounded w-1/3 mb-2"></div>
          <div class="h-5 bg-gray-200 rounded w-2/3"></div>
        </div>
      </dl>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="px-4 py-12 text-center text-red-600 bg-red-50/30 border-red-100 border rounded-md m-4">
      <p class="text-sm font-medium">{{ error }}</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="items.length === 0" class="px-4 py-12 text-center text-gray-500 bg-gray-50/30">
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
          <dt class="text-xs font-medium text-gray-500 uppercase tracking-wide">{{ item.label }}</dt>
          <dd class="mt-1 text-sm text-gray-900">
            <!-- Slot for Custom Value -->
            <slot v-if="$slots['item-' + item.key]" :name="'item-' + item.key" :item="item" :value="item.value" />

            <!-- Default Renderers -->
            <template v-else>
              <StatusBadge
                v-if="item.type === 'status'"
                :label="item.value"
                :variant="item.variant || 'gray'"
              />

              <span v-else-if="item.type === 'currency'" class="font-mono text-gray-700">
                {{ formatCurrency(item.value) }}
              </span>

              <span v-else-if="item.type === 'date'">
                {{ formatDate(item.value) }}
              </span>

              <span v-else-if="item.type === 'code'" class="font-mono bg-gray-100 px-1.5 py-0.5 rounded text-xs text-gray-700">
                {{ item.value || '-' }}
              </span>

              <a v-else-if="item.type === 'link'" :href="item.href" class="text-blue-600 hover:text-blue-800 underline">
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
