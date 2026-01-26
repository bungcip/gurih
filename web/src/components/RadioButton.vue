<script setup>
defineProps({
  modelValue: {
    type: [String, Number, Boolean],
    default: null
  },
  options: {
    type: Array,
    required: true,
    // Expects array of { label: String, value: Any }
  },
  name: {
    type: String,
    default: () => 'radio-' + Math.random().toString(36).substr(2, 9)
  },
  vertical: {
      type: Boolean,
      default: false
  }
})

defineEmits(['update:modelValue'])
</script>

<template>
  <div class="flex" :class="vertical ? 'flex-col gap-2' : 'flex-wrap gap-4'">
    <label 
      v-for="option in options" 
      :key="option.value" 
      class="flex items-center gap-2 cursor-pointer group"
    >
      <div class="relative flex items-center justify-center w-5 h-5">
        <input 
            type="radio" 
            :name="name" 
            :value="option.value"
            :checked="modelValue === option.value"
            @change="$emit('update:modelValue', option.value)"
            class="peer appearance-none w-5 h-5 border-2 border-gray-300 dark:border-gray-600 rounded-full checked:border-primary checked:border-[5px] transition-all bg-white dark:bg-gray-800"
        >
      </div>
      <span class="text-sm text-text-main group-hover:text-black dark:group-hover:text-white">{{ option.label }}</span>
    </label>
  </div>
</template>
