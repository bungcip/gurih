<script setup>
defineProps({
  modelValue: {
    type: Number,
    required: true
  },
  items: {
    type: Array, // Array of strings or objects with 'label'
    required: true
  }
})

defineEmits(['update:modelValue'])
</script>

<template>
  <div class="border-b border-gray-200">
    <div class="flex gap-2">
        <button 
            v-for="(item, index) in items" 
            :key="index"
            type="button"
            @click="$emit('update:modelValue', index)"
            class="px-4 py-2 text-sm font-medium transition-all relative top-[1px] border-b-2"
            :class="index === modelValue 
                ? 'text-primary border-primary bg-primary/5' 
                : 'text-gray-500 border-transparent hover:text-gray-700 hover:border-gray-300'"
        >
            {{ typeof item === 'string' ? item : item.label || item.title }}
        </button>
    </div>
  </div>
</template>
