<script setup>
import { ref, computed, onUnmounted, watch, nextTick } from 'vue'

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
const triggerRef = ref(null)

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
  triggerRef.value?.focus()
}

function handleClickOutside(event) {
  if (containerRef.value && !containerRef.value.contains(event.target)) {
    isOpen.value = false
  }
}

async function openAndFocusNext() {
  if (!isOpen.value) {
    isOpen.value = true
    await nextTick()
  }
  focusOption('first')
}

async function openAndFocusPrev() {
  if (!isOpen.value) {
    isOpen.value = true
    await nextTick()
  }
  focusOption('last')
}

function focusOption(position) {
  if (!containerRef.value) return
  const options = containerRef.value.querySelectorAll('[role="option"]:not([aria-disabled="true"])')
  if (options.length === 0) return

  if (position === 'first') options[0].focus()
  else if (position === 'last') options[options.length - 1].focus()
}

function focusNext(event) {
  let next = event.target.nextElementSibling
  while (next && next.getAttribute('role') !== 'option') {
    next = next.nextElementSibling
  }
  if (next) next.focus()
}

function focusPrev(event) {
  let prev = event.target.previousElementSibling
  while (prev && prev.getAttribute('role') !== 'option') {
    prev = prev.previousElementSibling
  }
  if (prev) prev.focus()
}

function closeAndFocusTrigger() {
  isOpen.value = false
  triggerRef.value?.focus()
}

// Bolt Optimization: Only attach global listener when dropdown is open (O(1) vs O(N))
watch(isOpen, (newValue) => {
  if (newValue) {
    // Use setTimeout to avoid immediate closing if the opening click bubbled up
    setTimeout(() => {
      document.addEventListener('click', handleClickOutside)
    }, 0)
  } else {
    document.removeEventListener('click', handleClickOutside)
  }
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<template>
  <div class="relative" ref="containerRef">
    <div 
      :id="id"
      ref="triggerRef"
      @click="toggle"
      @keydown.enter.prevent="toggle"
      @keydown.space.prevent="toggle"
      @keydown.down.prevent="openAndFocusNext"
      @keydown.up.prevent="openAndFocusPrev"
      @keydown.esc="isOpen = false"
      class="input-field flex items-center justify-between cursor-pointer bg-[--color-surface] focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-1"
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
          @keydown.down.prevent="focusNext"
          @keydown.up.prevent="focusPrev"
          @keydown.esc.prevent="closeAndFocusTrigger"
          @keydown.tab="isOpen = false"
          class="px-4 py-2 text-sm text-text-main hover:bg-gray-50 dark:hover:bg-gray-700 hover:text-primary cursor-pointer flex items-center justify-between group focus:bg-blue-50 focus:text-blue-600 dark:focus:bg-blue-900/30 dark:focus:text-blue-400 focus:outline-none"
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
