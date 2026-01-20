<script setup>
import { ref, watch, onMounted } from 'vue'

const props = defineProps({
  modelValue: {
    type: [Number, String],
    default: 0
  },
  label: {
    type: String,
    default: ''
  },
  prefix: {
    type: String,
    default: 'Rp'
  },
  decimals: {
    type: Number,
    default: 0
  },
  placeholder: {
    type: String,
    default: '0'
  },
  disabled: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['update:modelValue'])

const displayValue = ref('')

function formatCurrency(value) {
  if (value === null || value === undefined || value === '') return ''
  
  const number = typeof value === 'string' ? parseFloat(value.replace(/[^0-9.-]+/g, '')) : value
  if (isNaN(number)) return ''

  return new Intl.NumberFormat('id-ID', {
    style: 'decimal',
    minimumFractionDigits: props.decimals,
    maximumFractionDigits: props.decimals,
  }).format(number)
}

function updateValue(event) {
  const input = event.target.value
  // Remove all non-numeric characters except decimal point and minus sign
  const numericString = input.replace(/[^0-9.-]+/g, '')
  const numericValue = numericString === '' ? 0 : parseFloat(numericString)
  
  displayValue.value = formatCurrency(numericValue)
  emit('update:modelValue', numericValue)
}

watch(() => props.modelValue, (newVal) => {
  const currentNumeric = parseFloat(displayValue.value.replace(/[^0-9.-]+/g, ''))
  if (newVal !== currentNumeric) {
    displayValue.value = formatCurrency(newVal)
  }
}, { immediate: true })

onMounted(() => {
  displayValue.value = formatCurrency(props.modelValue)
})
</script>

<template>
  <div class="w-full">
    <label v-if="label" class="block text-sm font-medium text-gray-700 mb-1">
      {{ label }}
    </label>
    <div class="relative flex items-center">
      <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none z-10">
        <span class="text-gray-500 sm:text-sm font-medium">{{ prefix }}</span>
      </div>
      <input
        type="text"
        :value="displayValue"
        @input="updateValue"
        @blur="displayValue = formatCurrency(modelValue)"
        :placeholder="placeholder"
        :disabled="disabled"
        class="input-field block w-full pl-12 sm:text-sm"
        :class="{ 'bg-gray-100 cursor-not-allowed text-gray-400': disabled }"
        style="padding-left: 3rem !important;"
      />
    </div>
  </div>
</template>
