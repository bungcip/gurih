<script setup>
const props = defineProps({
  modelValue: {
    type: Boolean,
    default: false
  },
  label: {
    type: String,
    default: ''
  },
  id: {
    type: String,
    default: null
  },
  disabled: {
    type: Boolean,
    default: false
  }
})

defineEmits(['update:modelValue'])
</script>

<template>
  <label
    class="inline-flex items-center gap-3 transition-opacity"
    :class="{'cursor-not-allowed opacity-60': disabled, 'cursor-pointer': !disabled}"
    :for="id"
    :aria-disabled="disabled"
  >
    <div class="relative">
      <input 
        :id="id"
        type="checkbox" 
        class="sr-only peer" 
        :checked="modelValue" 
        :disabled="disabled"
        @change="$emit('update:modelValue', $event.target.checked)"
      >
      <div class="w-11 h-6 bg-gray-200 dark:bg-gray-700 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-primary/20 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 dark:after:border-gray-500 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
    </div>
    <span v-if="label" class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ label }}</span>
  </label>
</template>
