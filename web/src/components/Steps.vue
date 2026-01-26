<script setup>
import Icon from './Icon.vue'

const props = defineProps({
  items: {
    type: Array,
    required: true,
    default: () => []
  },
  current: {
    type: Number,
    default: 0
  },
  clickable: {
    type: Boolean,
    default: false
  },
  vertical: {
    type: Boolean,
    default: false
  },
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['change'])

function onStepClick(index) {
  if (props.clickable && !props.loading) {
    emit('change', index)
  }
}

function getStepStatus(item, index) {
  if (item.status) return item.status
  if (index < props.current) return 'completed'
  if (index === props.current) return 'current'
  return 'upcoming'
}
</script>

<template>
  <div class="w-full">
    <!-- Loading State -->
    <div v-if="loading" class="animate-pulse w-full">
       <div v-if="vertical" class="space-y-6">
          <div v-for="i in 3" :key="i" class="flex gap-4">
             <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 shrink-0"></div>
             <div class="flex-1 space-y-2 py-1">
                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/3"></div>
                <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
             </div>
          </div>
       </div>
       <div v-else class="flex items-center justify-between gap-4">
          <div v-for="i in 3" :key="i" class="flex flex-col items-center flex-1">
             <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 shrink-0 mb-2"></div>
             <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
          </div>
       </div>
    </div>

    <!-- Empty State -->
    <div v-else-if="items.length === 0" class="text-center py-4 text-text-muted text-sm italic">
      No steps defined.
    </div>

    <!-- Steps -->
    <div v-else :class="['relative', vertical ? 'flex-col space-y-0' : 'flex items-start justify-between w-full']">

      <div
        v-for="(step, index) in items"
        :key="index"
        :class="[
            'relative flex group',
            vertical ? 'pb-8 last:pb-0' : 'flex-1',
            clickable && !loading ? 'cursor-pointer' : 'cursor-default'
        ]"
        @click="onStepClick(index)"
      >
        <!-- Connector Line (Horizontal) -->
        <div
            v-if="!vertical && index !== items.length - 1"
            class="absolute top-4 left-1/2 w-full h-[2px] -ml-0 bg-gray-200 dark:bg-gray-700 -z-10"
        >
             <div
                class="h-full bg-primary transition-all duration-300 ease-in-out"
                :class="{'w-full': index < current, 'w-0': index >= current}"
             ></div>
        </div>

        <!-- Connector Line (Vertical) -->
         <div
            v-if="vertical && index !== items.length - 1"
            class="absolute top-8 left-4 w-[2px] h-full -ml-px bg-gray-200 dark:bg-gray-700"
        >
             <div
                class="w-full bg-primary transition-all duration-300 ease-in-out"
                :class="{'h-full': index < current, 'h-0': index >= current}"
             ></div>
        </div>

        <div :class="['flex', vertical ? 'gap-4' : 'flex-col items-center w-full']">

            <!-- Circle/Icon -->
            <div
                class="relative flex h-8 w-8 items-center justify-center rounded-full border-2 transition-colors duration-200 z-10"
                :class="{
                    'bg-primary border-primary text-white': getStepStatus(step, index) === 'completed',
                    'bg-white dark:bg-gray-800 border-primary text-primary ring-4 ring-blue-50 dark:ring-blue-900/20': getStepStatus(step, index) === 'current',
                    'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600 text-gray-400 dark:text-gray-600': getStepStatus(step, index) === 'upcoming',
                    'bg-white dark:bg-gray-800 border-red-500 text-red-500 dark:text-red-400': getStepStatus(step, index) === 'error',
                    'group-hover:border-primary group-hover:text-primary': clickable && getStepStatus(step, index) !== 'completed' && getStepStatus(step, index) !== 'current'
                }"
            >
                <Icon v-if="getStepStatus(step, index) === 'completed'" name="check-circle" :size="20" />
                <Icon v-else-if="getStepStatus(step, index) === 'error'" name="alert-circle" :size="16" />
                <Icon v-else-if="step.icon" :name="step.icon" :size="16" />
                <span v-else class="text-xs font-bold">{{ index + 1 }}</span>
            </div>

            <!-- Content -->
            <div :class="['flex flex-col', vertical ? 'pt-0.5' : 'items-center text-center mt-2']">
                <span
                    class="text-sm font-medium transition-colors duration-200"
                    :class="{
                        'text-primary': getStepStatus(step, index) === 'current',
                        'text-text-main': getStepStatus(step, index) === 'completed',
                        'text-gray-500 dark:text-gray-500': getStepStatus(step, index) === 'upcoming',
                        'text-red-600 dark:text-red-400': getStepStatus(step, index) === 'error'
                    }"
                >
                    {{ step.label }}
                </span>
                <span v-if="step.description" class="text-xs text-text-muted mt-0.5 max-w-[150px]">
                    {{ step.description }}
                </span>
            </div>

        </div>
      </div>
    </div>
  </div>
</template>
