<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  title: {
    type: String,
    default: ''
  },
  description: {
    type: String,
    default: ''
  },
  variant: {
    type: String,
    default: 'info',
    validator: (val) => ['info', 'success', 'warning', 'danger'].includes(val)
  },
  icon: {
    type: [Boolean, String],
    default: true
  },
  closable: {
    type: Boolean,
    default: false
  },
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['close'])

const variantStyles = {
  info: 'bg-blue-50 text-blue-800 border-blue-200',
  success: 'bg-green-50 text-green-800 border-green-200',
  warning: 'bg-yellow-50 text-yellow-800 border-yellow-200',
  danger: 'bg-red-50 text-red-800 border-red-200'
}

const iconColors = {
  info: 'text-blue-500',
  success: 'text-green-500',
  warning: 'text-yellow-500',
  danger: 'text-red-500'
}

const defaultIcons = {
  info: 'alert-circle',
  success: 'check-circle',
  warning: 'alert-circle',
  danger: 'alert-circle'
}

const resolvedIcon = computed(() => {
  if (typeof props.icon === 'string') return props.icon
  if (props.icon === true) return defaultIcons[props.variant]
  return null
})
</script>

<template>
  <div
    class="rounded-lg border p-4 transition-all duration-200"
    :class="variantStyles[variant]"
    role="alert"
  >
    <div class="flex items-start gap-3">
      <!-- Loading Spinner -->
      <div v-if="loading" class="flex-shrink-0 mt-0.5">
          <svg class="animate-spin h-5 w-5 opacity-70" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" aria-hidden="true">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
      </div>

      <!-- Icon -->
      <div v-else-if="resolvedIcon" class="flex-shrink-0 mt-0.5" :class="iconColors[variant]">
        <Icon :name="resolvedIcon" :size="20" />
      </div>

      <!-- Content -->
      <div class="flex-1 min-w-0">
        <h3 v-if="title" class="text-sm font-semibold leading-5 mb-1">
          {{ title }}
        </h3>
        <div v-if="description || $slots.default" class="text-sm opacity-90 leading-relaxed">
           <p v-if="description">{{ description }}</p>
           <slot></slot>
        </div>
      </div>

      <!-- Action Slot (Right aligned) -->
      <div v-if="$slots.action" class="flex-shrink-0 self-center">
        <slot name="action"></slot>
      </div>

      <!-- Close Button -->
      <div v-if="closable" class="flex-shrink-0 -mr-1 -mt-1 ml-auto pl-3">
        <button
          type="button"
          class="inline-flex rounded-md p-1.5 hover:bg-black/5 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-current focus:ring-current transition-colors"
          @click="$emit('close')"
          aria-label="Dismiss"
        >
          <span class="sr-only">Dismiss</span>
          <!-- Reusing plus icon rotated 45deg for close -->
          <Icon name="plus" :size="16" class="transform rotate-45" />
        </button>
      </div>
    </div>
  </div>
</template>
