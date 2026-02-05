<script setup>
import { computed } from 'vue'
import Button from './Button.vue'

const props = defineProps({
  modelValue: { type: Number, required: true },
  total: { type: Number, required: true },
  perPage: { type: Number, default: 10 },
  loading: { type: Boolean, default: false },
  showDetails: { type: Boolean, default: true }
})

const emit = defineEmits(['update:modelValue', 'change'])

const totalPages = computed(() => {
    if (props.perPage <= 0) return 0
    return Math.ceil(props.total / props.perPage)
})

const startItem = computed(() => {
  if (props.total === 0) return 0
  return (props.modelValue - 1) * props.perPage + 1
})

const endItem = computed(() => {
  return Math.min(props.modelValue * props.perPage, props.total)
})

// Logic to generate page numbers with ellipsis
const pages = computed(() => {
  const total = totalPages.value
  const current = props.modelValue

  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1)
  }

  // Always show 1, total, current, and neighbours
  const pagesToShow = new Set([1, total, current, current - 1, current + 1])

  // If current is close to start, show more start
  if (current <= 4) {
      pagesToShow.add(2)
      pagesToShow.add(3)
      pagesToShow.add(4)
      pagesToShow.add(5)
  }

  // If current is close to end, show more end
  if (current >= total - 3) {
      pagesToShow.add(total - 1)
      pagesToShow.add(total - 2)
      pagesToShow.add(total - 3)
      pagesToShow.add(total - 4)
  }

  const sorted = [...pagesToShow].filter(p => p >= 1 && p <= total).sort((a, b) => a - b)

  const result = []
  let prev = null

  for (const p of sorted) {
      if (prev !== null) {
          if (p - prev === 2) {
              result.push(prev + 1)
          } else if (p - prev > 1) {
              result.push('...')
          }
      }
      result.push(p)
      prev = p
  }

  return result
})

function setPage(p) {
  if (p === '...' || p === props.modelValue || props.loading) return
  if (p < 1 || p > totalPages.value) return
  emit('update:modelValue', p)
  emit('change', p)
}
</script>

<template>
  <nav class="flex flex-col sm:flex-row items-center justify-between gap-4" role="navigation" aria-label="Pagination">
    <!-- Summary Text -->
    <div v-if="showDetails" class="text-sm text-text-muted">
      Showing <span class="font-medium text-text-main">{{ startItem }}</span> to <span class="font-medium text-text-main">{{ endItem }}</span> of <span class="font-medium text-text-main">{{ total }}</span> results
    </div>

    <!-- Controls -->
    <div class="flex flex-1 justify-center sm:justify-end gap-2 w-full sm:w-auto">
      <Button
        variant="outline"
        size="sm"
        :disabled="modelValue <= 1 || loading"
        @click="setPage(modelValue - 1)"
        aria-label="Previous Page"
      >
        <!-- Inline Chevron Left -->
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
      </Button>

      <div class="flex gap-1 overflow-x-auto max-w-[200px] sm:max-w-none no-scrollbar">
        <template v-for="(p, index) in pages" :key="index">
            <span v-if="p === '...'" class="px-2 py-1 flex items-center text-text-muted select-none" aria-hidden="true">...</span>
            <Button
              v-else
              :variant="p === modelValue ? 'primary' : 'ghost'"
              size="sm"
              :disabled="loading"
              @click="setPage(p)"
              :class="{'min-w-[32px]': true}"
              :aria-current="p === modelValue ? 'page' : undefined"
              :aria-label="p === modelValue ? `Page ${p}, current page` : `Go to page ${p}`"
            >
              {{ p }}
            </Button>
        </template>
      </div>

      <Button
        variant="outline"
        size="sm"
        :disabled="modelValue >= totalPages || loading"
        @click="setPage(modelValue + 1)"
        aria-label="Next Page"
      >
        <!-- Inline Chevron Right -->
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
      </Button>
    </div>
  </nav>
</template>
