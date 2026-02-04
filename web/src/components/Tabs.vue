<script setup>
import { ref } from 'vue'

const props = defineProps({
  modelValue: {
    type: Number,
    required: true
  },
  items: {
    type: Array, // Array of strings or objects with 'label'
    required: true
  }
})

const emit = defineEmits(['update:modelValue'])

const tabRefs = ref([])

function handleKeydown(e, index) {
  const count = props.items.length
  let nextIndex = null

  if (e.key === 'ArrowRight') {
    nextIndex = (index + 1) % count
  } else if (e.key === 'ArrowLeft') {
    nextIndex = (index - 1 + count) % count
  } else if (e.key === 'Home') {
    nextIndex = 0
  } else if (e.key === 'End') {
    nextIndex = count - 1
  }

  if (nextIndex !== null) {
    e.preventDefault()
    emit('update:modelValue', nextIndex)
    // Focus the new tab
    const el = tabRefs.value[nextIndex]
    if (el) el.focus()
  }
}
</script>

<template>
  <div class="border-b border-gray-200 dark:border-gray-700">
    <div
      class="flex gap-2"
      role="tablist"
      aria-orientation="horizontal"
    >
        <button 
            v-for="(item, index) in items" 
            :key="index"
            :ref="el => { if(el) tabRefs[index] = el }"
            type="button"
            role="tab"
            :aria-selected="index === modelValue"
            :tabindex="index === modelValue ? 0 : -1"
            @click="$emit('update:modelValue', index)"
            @keydown="handleKeydown($event, index)"
            class="px-4 py-2 text-sm font-medium transition-all relative top-[1px] border-b-2 flex items-center gap-2 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 focus-visible:rounded-t"
            :class="index === modelValue 
                ? 'text-primary border-primary bg-primary/5 dark:bg-primary/20'
                : 'text-gray-500 border-transparent hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300 dark:hover:border-gray-600'"
        >
            <span>{{ typeof item === 'string' ? item : item.label || item.title || item.name }}</span>
            <span 
                v-if="typeof item === 'object' && (item.badge !== undefined && item.badge !== null)"
                class="px-1.5 py-0.5 text-[10px] rounded-full min-w-[1.2rem] flex items-center justify-center font-bold"
                :class="index === modelValue ? 'bg-primary text-white' : 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'"
            >
                {{ item.badge }}
            </span>
        </button>
    </div>
  </div>
</template>
