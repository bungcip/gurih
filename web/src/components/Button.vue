<script setup>
import { computed } from 'vue'

const props = defineProps({
  variant: {
    type: String,
    default: 'primary',
    validator: (value) => [
      'primary',
      'danger',
      'secondary',
      'outline',
      'ghost',
      'ghost-primary',
      'ghost-danger'
    ].includes(value)
  },
  size: {
    type: String,
    default: 'md',
    validator: (value) => ['sm', 'md', 'lg'].includes(value)
  },
  type: {
    type: String,
    default: 'button'
  },
  icon: {
      type: String,
      default: ''
  }
})

const classes = computed(() => {
  const base = 'font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-offset-1 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2'

  const sizes = {
    sm: 'px-3 py-1 text-xs',
    md: 'px-4 py-2 text-sm',
    lg: 'px-6 py-3 text-base'
  }

  const variants = {
    primary: 'bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500 shadow-sm',
    danger: 'bg-red-600 text-white hover:bg-red-700 focus:ring-red-500 shadow-sm border-red-700',
    secondary: 'bg-gray-100 text-gray-700 hover:bg-gray-200 focus:ring-gray-500',
    outline: 'border border-gray-200 text-gray-700 hover:bg-gray-50 focus:ring-gray-500',
    ghost: 'bg-transparent hover:bg-gray-100 text-gray-700',
    'ghost-primary': 'text-blue-600 hover:bg-blue-50 focus:ring-blue-500 bg-transparent',
    'ghost-danger': 'text-red-500 hover:bg-red-50 focus:ring-red-500 bg-transparent'
  }

  return [base, sizes[props.size], variants[props.variant]]
})
</script>

<template>
  <button :type="type" :class="classes">
    <slot name="icon">
        <span v-if="icon === 'plus'" class="text-lg leading-none">+</span>
    </slot>
    <slot />
  </button>
</template>
