<script setup>
import { computed } from 'vue'

const props = defineProps({
  value: {
    type: Number,
    default: 0
  },
  max: {
    type: Number,
    default: 100
  },
  label: {
    type: String,
    default: ''
  },
  variant: {
    type: String,
    default: 'primary',
    validator: (val) => ['primary', 'success', 'warning', 'danger', 'info', 'gray'].includes(val)
  },
  size: {
    type: String,
    default: 'md',
    validator: (val) => ['sm', 'md', 'lg', 'xl'].includes(val)
  },
  showValue: {
    type: Boolean,
    default: false
  },
  striped: {
    type: Boolean,
    default: false
  },
  loading: {
    type: Boolean,
    default: false
  },
  error: {
    type: String,
    default: ''
  }
})

const percentage = computed(() => {
  if (props.max <= 0) return 0
  const val = Math.min(Math.max(0, props.value), props.max)
  return Math.round((val / props.max) * 100)
})

const sizeClasses = computed(() => {
  const map = {
    sm: 'h-1.5',
    md: 'h-2.5',
    lg: 'h-4',
    xl: 'h-6'
  }
  return map[props.size]
})

const colorClasses = computed(() => {
  if (props.error) return 'bg-red-600 dark:bg-red-500'

  const map = {
    primary: 'bg-blue-600 dark:bg-blue-500',
    success: 'bg-green-500 dark:bg-green-400',
    warning: 'bg-yellow-400 dark:bg-yellow-300',
    danger: 'bg-red-600 dark:bg-red-500',
    info: 'bg-cyan-500 dark:bg-cyan-400',
    gray: 'bg-gray-600 dark:bg-gray-500'
  }
  return map[props.variant]
})

const displayValue = computed(() => {
  if (props.error) return 'Error'
  if (props.loading) return 'Loading...'
  return `${percentage.value}%`
})

</script>

<template>
  <div class="w-full">
    <!-- Header: Label & Value -->
    <div v-if="label || showValue || error" class="flex justify-between mb-1">
      <span v-if="label" class="text-sm font-medium text-text-main dark:text-gray-300">{{ label }}</span>
      <span v-if="error" class="text-sm font-medium text-red-600 dark:text-red-400">{{ error }}</span>
      <span v-else-if="showValue" class="text-sm font-medium text-text-muted dark:text-gray-400">{{ displayValue }}</span>
    </div>

    <!-- Progress Track -->
    <div
      class="w-full bg-gray-200 rounded-full dark:bg-gray-700 overflow-hidden"
      :class="[sizeClasses, { 'animate-pulse': loading }]"
      role="progressbar"
      :aria-valuenow="loading ? undefined : value"
      :aria-valuemin="0"
      :aria-valuemax="max"
      :aria-busy="loading"
    >
      <!-- Progress Fill -->
      <div
        v-if="!loading"
        class="h-full rounded-full transition-all duration-300 ease-out flex items-center justify-center text-[10px] leading-none text-white font-bold"
        :class="[
          colorClasses,
          { 'progress-striped': striped }
        ]"
        :style="{ width: `${percentage}%` }"
      >
        <!-- Optional: Value inside bar for XL size -->
        <span v-if="size === 'xl' && showValue && percentage > 10" class="drop-shadow-sm px-2">
            {{ displayValue }}
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.progress-striped {
  background-image: linear-gradient(
    45deg,
    rgba(255, 255, 255, 0.15) 25%,
    transparent 25%,
    transparent 50%,
    rgba(255, 255, 255, 0.15) 50%,
    rgba(255, 255, 255, 0.15) 75%,
    transparent 75%,
    transparent
  );
  background-size: 1rem 1rem;
  animation: progress-stripes 1s linear infinite;
}

@keyframes progress-stripes {
  from { background-position: 1rem 0; }
  to { background-position: 0 0; }
}
</style>
