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
  },
  id: {
    type: String,
    default: null
  }
})

const emit = defineEmits(['update:modelValue'])

const displayValue = ref('')

function parseCurrency(value) {
  if (value === null || value === undefined || value === '') return 0;
  if (typeof value === 'number') return value;
  
  const cleanValue = props.decimals > 0 
    ? value.replace(/[^0-9,-]+/g, '').replace(',', '.') 
    : value.replace(/[^0-9-]+/g, '');
    
  return parseFloat(cleanValue) || 0;
}

function formatCurrency(value) {
  if (value === null || value === undefined || value === '') return ''
  
  const number = typeof value === 'string' ? parseCurrency(value) : value
  if (isNaN(number)) return ''

  return new Intl.NumberFormat('id-ID', {
    style: 'decimal',
    minimumFractionDigits: props.decimals,
    maximumFractionDigits: props.decimals,
  }).format(number)
}

function updateValue(event) {
  const numericValue = parseCurrency(event.target.value)
  emit('update:modelValue', numericValue)
}

watch(() => props.modelValue, (newVal) => {
  const currentNumeric = parseCurrency(displayValue.value)
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
    <!-- Label is handled by parent usually if this is part of dynamic form, but it has internal label prop too -->
    <label v-if="label" :for="id" class="block text-sm font-medium text-text-muted mb-1">
      {{ label }}
    </label>
    <div class="relative flex items-center">
      <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none z-10">
        <span class="text-text-muted sm:text-sm font-medium">{{ prefix }}</span>
      </div>
      <input
        :id="id"
        type="text"
        :value="displayValue"
        @input="updateValue"
        @blur="displayValue = formatCurrency(modelValue)"
        :placeholder="placeholder"
        :disabled="disabled"
        class="input-field block w-full pl-12 sm:text-sm"
        :class="{ 'bg-[--color-background] cursor-not-allowed text-text-muted': disabled }"
        style="padding-left: 3rem !important;"
      />
    </div>
  </div>
</template>
