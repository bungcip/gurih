<script setup>
import Icon from './Icon.vue'
import Button from './Button.vue'

defineProps({
  title: {
    type: String,
    default: 'No data available'
  },
  description: {
    type: String,
    default: ''
  },
  icon: {
    type: String,
    default: 'alert-circle'
  },
  actionLabel: {
    type: String,
    default: ''
  },
  loading: {
    type: Boolean,
    default: false
  },
  error: {
    type: Boolean,
    default: false
  }
})

defineEmits(['action'])
</script>

<template>
  <div
    class="flex flex-col items-center justify-center p-8 text-center rounded-lg border-2 border-dashed transition-colors duration-200"
    :class="[
      error
        ? 'border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-900/10'
        : 'border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/30'
    ]"
  >
    <!-- Loading State -->
    <div v-if="loading" class="animate-pulse flex flex-col items-center space-y-4">
       <div class="h-12 w-12 bg-gray-200 dark:bg-gray-700 rounded-full"></div>
       <div class="h-4 w-48 bg-gray-200 dark:bg-gray-700 rounded"></div>
       <div class="h-3 w-32 bg-gray-200 dark:bg-gray-700 rounded"></div>
    </div>

    <!-- Content State -->
    <div v-else class="flex flex-col items-center max-w-sm">
        <div
            class="mb-4 p-4 rounded-full transition-colors duration-200"
            :class="error
                ? 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400'
                : 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400'"
        >
            <Icon :name="icon" :size="32" />
        </div>

        <h3
            class="text-base font-semibold"
            :class="error ? 'text-red-800 dark:text-red-300' : 'text-text-main'"
        >
            {{ title }}
        </h3>

        <p v-if="description" class="mt-1 text-sm text-text-muted">
            {{ description }}
        </p>

        <div v-if="actionLabel || $slots.action" class="mt-6">
            <slot name="action">
                <Button
                    v-if="actionLabel"
                    :variant="error ? 'ghost-danger' : 'primary'"
                    @click="$emit('action')"
                >
                    {{ actionLabel }}
                </Button>
            </slot>
        </div>
    </div>
  </div>
</template>
