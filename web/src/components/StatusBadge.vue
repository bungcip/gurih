<script>
// Optimization: Hoisted static variants map to avoid reallocation on every render
const variants = {
  success: 'bg-green-100 text-green-800 border-green-200',
  warning: 'bg-yellow-100 text-yellow-800 border-yellow-200',
  danger: 'bg-red-100 text-red-800 border-red-200',
  info: 'bg-blue-100 text-blue-800 border-blue-200',
  gray: 'bg-gray-100 text-gray-600 border-gray-200'
}
</script>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  label: {
    type: String,
    required: true
  },
  variant: {
    type: String,
    default: 'gray',
    validator: (value) => ['success', 'warning', 'danger', 'info', 'gray'].includes(value)
  }
})

const variantClass = computed(() => variants[props.variant] || variants['gray'])
</script>

<template>
  <span 
    class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border"
    :class="variantClass"
  >
    {{ label }}
  </span>
</template>
