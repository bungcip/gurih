<script setup>
import { computed } from 'vue'
import Button from './Button.vue'
import SelectInput from './SelectInput.vue'
import DatePicker from './DatePicker.vue'
import Switch from './Switch.vue'

const props = defineProps({
  filters: {
    type: Array,
    required: true
  },
  modelValue: {
    type: Object,
    default: () => ({})
  },
  loading: {
    type: Boolean,
    default: false
  },
  clearable: {
    type: Boolean,
    default: true
  },
  title: {
    type: String,
    default: 'Filters'
  }
})

const emit = defineEmits(['update:modelValue', 'clear'])

function updateFilter(key, value) {
  const newState = { ...props.modelValue, [key]: value }
  emit('update:modelValue', newState)
}

function clearFilters() {
  emit('clear')
  emit('update:modelValue', {})
}

const hasActiveFilters = computed(() => {
  if (!props.modelValue) return false
  return Object.values(props.modelValue).some(val => val !== null && val !== '' && val !== false)
})
</script>

<template>
  <div class="bg-[--color-surface] border border-gray-200 dark:border-gray-700 rounded-lg shadow-sm p-4">
    <!-- Header -->
    <div class="flex justify-between items-center mb-4">
      <div class="flex items-center gap-2 text-text-main font-medium">
        <!-- Filter Icon -->
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"></polygon>
        </svg>
        <span>{{ title }}</span>
      </div>
      <div class="flex gap-2">
         <Button
            v-if="clearable && hasActiveFilters"
            variant="ghost"
            size="sm"
            @click="clearFilters"
            :disabled="loading"
         >
            Clear All
         </Button>
         <slot name="actions"></slot>
      </div>
    </div>

    <!-- Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
       <template v-for="filter in filters" :key="filter.key">
          <!-- Text Input -->
          <div v-if="filter.type === 'text'" class="space-y-1">
             <label :for="filter.key" class="text-xs font-medium text-text-muted uppercase">{{ filter.label }}</label>
             <div class="relative">
                <input
                    :id="filter.key"
                    :value="modelValue[filter.key]"
                    @input="updateFilter(filter.key, $event.target.value)"
                    type="text"
                    class="input-field w-full pr-8"
                    :placeholder="filter.placeholder || 'Search...'"
                    :disabled="loading"
                />
                 <div v-if="modelValue[filter.key]" class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 cursor-pointer p-1" @click="updateFilter(filter.key, '')">
                     <!-- X Icon -->
                     <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <line x1="18" y1="6" x2="6" y2="18"></line>
                        <line x1="6" y1="6" x2="18" y2="18"></line>
                     </svg>
                 </div>
             </div>
          </div>

          <!-- Select Input -->
          <div v-else-if="filter.type === 'select'" class="space-y-1">
             <label :for="filter.key" class="text-xs font-medium text-text-muted uppercase">{{ filter.label }}</label>
             <SelectInput
                :id="filter.key"
                :modelValue="modelValue[filter.key]"
                @update:modelValue="updateFilter(filter.key, $event)"
                :options="filter.options || []"
                :placeholder="filter.placeholder || 'All'"
                :disabled="loading"
             />
          </div>

          <!-- Date Picker -->
          <div v-else-if="filter.type === 'date'" class="space-y-1">
             <label :for="filter.key" class="text-xs font-medium text-text-muted uppercase">{{ filter.label }}</label>
             <DatePicker
                :id="filter.key"
                :modelValue="modelValue[filter.key]"
                @update:modelValue="updateFilter(filter.key, $event)"
                :disabled="loading"
             />
          </div>

           <!-- Boolean/Switch -->
           <div v-else-if="filter.type === 'boolean'" class="flex flex-col justify-end h-full pb-2">
                <Switch
                    :id="filter.key"
                    :modelValue="!!modelValue[filter.key]"
                    @update:modelValue="updateFilter(filter.key, $event)"
                    :label="filter.label"
                    :disabled="loading"
                />
           </div>
       </template>
    </div>
  </div>
</template>
