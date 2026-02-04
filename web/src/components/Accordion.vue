<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  items: {
    type: Array,
    required: true,
    // Expected structure: { title: String, content?: String, icon?: String, disabled?: Boolean }
  },
  modelValue: {
    type: [Array, Number, String, Object], // Object allows null
    default: null
  },
  multiple: {
    type: Boolean,
    default: false
  },
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['update:modelValue'])

const activeIds = computed(() => {
  if (props.multiple) {
    return Array.isArray(props.modelValue) ? props.modelValue : []
  }
  return props.modelValue !== null && props.modelValue !== undefined ? [props.modelValue] : []
})

const toggle = (index) => {
  const item = props.items[index]
  if (item && item.disabled) return

  if (props.multiple) {
    const newValue = [...activeIds.value]
    const pos = newValue.indexOf(index)
    if (pos === -1) {
      newValue.push(index)
    } else {
      newValue.splice(pos, 1)
    }
    emit('update:modelValue', newValue)
  } else {
    const isSame = activeIds.value.includes(index)
    emit('update:modelValue', isSame ? null : index)
  }
}

const isOpen = (index) => activeIds.value.includes(index)
</script>

<template>
  <div class="border border-border rounded-lg overflow-hidden divide-y divide-border bg-surface shadow-sm">
    <!-- Loading State -->
    <div v-if="loading" class="p-4 space-y-4">
      <div v-for="i in 3" :key="i" class="animate-pulse flex flex-col gap-2">
         <div class="h-10 bg-gray-100 dark:bg-gray-800 rounded w-full"></div>
      </div>
    </div>

    <!-- Empty State -->
    <div v-else-if="items.length === 0" class="p-8 text-center text-text-muted">
      <p>No items available</p>
    </div>

    <!-- Items -->
    <div v-else v-for="(item, index) in items" :key="index" class="bg-surface group">
      <h3>
        <button
          type="button"
          class="flex items-center justify-between w-full p-4 text-left font-medium text-text-main hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors focus:outline-none focus:ring-2 focus:ring-inset focus:ring-primary/20"
          :class="{ 'opacity-50 cursor-not-allowed': item.disabled }"
          @click="toggle(index)"
          :disabled="item.disabled"
          :aria-expanded="isOpen(index)"
        >
          <div class="flex items-center gap-3">
             <Icon v-if="item.icon" :name="item.icon" size="18" class="text-text-muted group-hover:text-primary transition-colors" />
             <span class="text-sm font-semibold">{{ item.title }}</span>
          </div>
          <Icon
              name="arrow-down"
              size="16"
              class="text-text-muted transition-transform duration-200"
              :class="{ 'rotate-180': isOpen(index) }"
          />
        </button>
      </h3>
      <div
          v-show="isOpen(index)"
          class="text-sm text-text-muted border-t border-border bg-gray-50/30 dark:bg-gray-900/10"
      >
           <div class="p-4">
             <slot :name="'item-' + index" :item="item">
                {{ item.content }}
             </slot>
           </div>
      </div>
    </div>
  </div>
</template>
