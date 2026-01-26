<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  modelValue: {
    type: String,
    default: ''
  },
  required: {
    type: Boolean,
    default: false
  },
  id: {
    type: String,
    default: null
  }
})

const emit = defineEmits(['update:modelValue'])

const inputRef = ref(null)

const dateValue = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})

function setToday() {
  const today = new Date().toISOString().split('T')[0]
  emit('update:modelValue', today)
}

function setLastWeek() {
  const d = new Date()
  d.setDate(d.getDate() - 7)
  const lastWeek = d.toISOString().split('T')[0]
  emit('update:modelValue', lastWeek)
}

function showPicker() {
  if (inputRef.value) {
    // showPicker support check
    if (typeof inputRef.value.showPicker === 'function') {
      try {
        inputRef.value.showPicker()
      } catch (e) {
        inputRef.value.focus()
      }
    } else {
      inputRef.value.focus()
    }
  }
}
</script>

<template>
  <div class="space-y-2">
    <div class="relative">
        <input 
            :id="id"
            ref="inputRef"
            v-model="dateValue" 
            type="date" 
            class="input-field w-full pr-10"
            :required="required"
        >
        <!-- Icon Wrapper: Opaque background to cover native icon, clickable to trigger picker -->
        <div 
            class="absolute inset-y-0 right-0 flex items-center pr-3 z-10 cursor-pointer text-text-muted hover:text-text-main"
            :class="{'bg-[--color-background] cursor-not-allowed': $attrs.disabled, 'bg-[--color-surface]': !$attrs.disabled}"
            style="margin: 1px; border-top-right-radius: 7px; border-bottom-right-radius: 7px; padding-left: 0.5rem;"
            @click="showPicker"
        >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
        </div>
    </div>
    <div class="flex gap-2 text-xs">
      <button 
        type="button" 
        @click="setToday"
        class="px-2 py-1 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-600 dark:text-gray-300 rounded transition"
      >
        Today
      </button>
      <button 
        type="button" 
        @click="setLastWeek"
        class="px-2 py-1 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-600 dark:text-gray-300 rounded transition"
      >
        Last Week
      </button>
    </div>
  </div>
</template>
