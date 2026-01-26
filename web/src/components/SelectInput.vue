<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'

const props = defineProps({
  modelValue: {
    type: [String, Number, null],
    default: null
  },
  options: {
    type: Array,
    default: () => []
  },
  placeholder: {
    type: String,
    default: 'Select an option...'
  },
  label: {
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

const isOpen = ref(false)
const containerRef = ref(null)

const selectedLabel = computed(() => {
  const selected = props.options.find(opt => opt.value === props.modelValue)
  return selected ? selected.label : props.placeholder
})

const isSelected = computed(() => {
    return props.options.some(opt => opt.value === props.modelValue)
})

function toggle() {
  isOpen.value = !isOpen.value
}

function select(option) {
  emit('update:modelValue', option.value)
  isOpen.value = false
}

function handleClickOutside(event) {
  if (containerRef.value && !containerRef.value.contains(event.target)) {
    isOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<template>
  <div class="relative" ref="containerRef">
    <div 
      :id="id"
      @click="toggle"
      @keydown.enter.prevent="toggle"
      @keydown.space.prevent="toggle"
      class="input-field flex items-center justify-between cursor-pointer bg-[--color-surface]"
      :class="{'text-text-muted': !isSelected, 'border-primary ring-1 ring-primary/20': isOpen}"
      tabindex="0"
      role="combobox"
      :aria-expanded="isOpen"
      aria-haspopup="listbox"
    >
      <span class="truncate block">{{ selectedLabel }}</span>
      <svg 
        class="w-4 h-4 text-text-muted transition-transform duration-200"
        :class="{'rotate-180': isOpen}"
        fill="none" 
        stroke="currentColor" 
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
      </svg>
    </div>

    <transition
      enter-active-class="transition ease-out duration-100"
      enter-from-class="transform opacity-0 scale-95"
      enter-to-class="transform opacity-100 scale-100"
      leave-active-class="transition ease-in duration-75"
      leave-from-class="transform opacity-100 scale-100"
      leave-to-class="transform opacity-0 scale-95"
    >
      <div 
        v-if="isOpen" 
        class="absolute z-50 mt-1 w-full bg-[--color-surface] border border-gray-200 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-auto py-1 focus:outline-none"
        role="listbox"
      >
        <div 
            v-if="options.length === 0" 
            class="px-4 py-2 text-sm text-text-muted"
            role="option"
            aria-disabled="true"
        >
            No options available
        </div>
        <div 
          v-for="option in options" 
          :key="option.value"
          @click="select(option)"
          @keydown.enter.prevent="select(option)"
          @keydown.space.prevent="select(option)"
          class="px-4 py-2 text-sm text-text-main hover:bg-gray-50 dark:hover:bg-gray-700 hover:text-primary cursor-pointer flex items-center justify-between group"
          :class="{'bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400 font-medium': option.value === modelValue}"
          role="option"
          :aria-selected="option.value === modelValue"
          tabindex="0"
        >
          <span>{{ option.label }}</span>
          <svg v-if="option.value === modelValue" class="w-4 h-4 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
               <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        </div>
      </div>
    </transition>
  </div>
</template>
