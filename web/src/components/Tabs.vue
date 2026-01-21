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
            class="px-4 py-2 text-sm font-medium transition-all relative top-[1px] border-b-2 flex items-center gap-2"
            :class="index === modelValue 
                ? 'text-primary border-primary bg-primary/5' 
                : 'text-gray-500 border-transparent hover:text-gray-700 hover:border-gray-300'"
        >
            <span>{{ typeof item === 'string' ? item : item.label || item.title || item.name }}</span>
            <span 
                v-if="typeof item === 'object' && (item.badge !== undefined && item.badge !== null)"
                class="px-1.5 py-0.5 text-[10px] rounded-full min-w-[1.2rem] flex items-center justify-center font-bold"
                :class="index === modelValue ? 'bg-primary text-white' : 'bg-gray-100 text-gray-500'"
            >
                {{ item.badge }}
            </span>
        </button>
    </div>
  </div>
</template>
