<script setup>
import Button from './Button.vue'
import StatusBadge from './StatusBadge.vue'
import Icon from './Icon.vue'

const props = defineProps({
  title: {
    type: String,
    required: true
  },
  description: {
    type: String,
    default: ''
  },
  // Meta items: { label, value, icon }
  meta: {
    type: Array,
    default: () => []
  },
  // Status: { label, variant } or just String
  status: {
    type: [String, Object],
    default: null
  },
  // Actions: { label, variant, value, disabled, loading, icon }
  actions: {
    type: Array,
    default: () => []
  },
  loading: {
    type: Boolean,
    default: false
  },
  error: {
    type: String,
    default: null
  },
  variant: {
    type: String,
    default: 'default', // default, bordered
    validator: (v) => ['default', 'bordered'].includes(v)
  }
})

const emit = defineEmits(['action', 'click'])

function handleAction(action) {
  if (action.disabled || action.loading) return
  emit('action', action)
}

function resolveStatus(status) {
    if (!status) return null
    if (typeof status === 'string') return { label: status, variant: 'gray' }
    return status
}
</script>

<template>
  <div
    class="card transition-all duration-200 flex flex-col h-full bg-white relative overflow-hidden"
    :class="{
        'border border-border shadow-sm': variant === 'default',
        'border-2 border-dashed border-gray-200 shadow-none': variant === 'bordered',
        'border-red-200 bg-red-50/10': error,
        'cursor-pointer hover:shadow-md hover:border-blue-200': !loading && !error && $attrs.onClick
    }"
    @click="$emit('click')"
  >
    <!-- Loading State -->
    <div v-if="loading" class="p-5 space-y-4 animate-pulse">
        <div class="flex justify-between items-start">
            <div class="h-5 bg-gray-200 rounded w-1/2"></div>
            <div class="h-5 bg-gray-200 rounded w-16"></div>
        </div>
        <div class="space-y-2">
            <div class="h-3 bg-gray-200 rounded w-3/4"></div>
            <div class="h-3 bg-gray-200 rounded w-1/2"></div>
        </div>
        <div class="pt-4 flex gap-2">
            <div class="h-8 bg-gray-200 rounded w-20"></div>
            <div class="h-8 bg-gray-200 rounded w-20"></div>
        </div>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="p-5 flex flex-col h-full justify-center items-center text-center space-y-3">
        <div class="p-3 bg-red-100 text-red-600 rounded-full">
            <Icon name="alert-circle" :size="24" />
        </div>
        <h3 class="font-medium text-gray-900">Failed to load</h3>
        <p class="text-sm text-gray-500">{{ error }}</p>
    </div>

    <!-- Content State -->
    <div v-else class="flex flex-col h-full">
        <!-- Header -->
        <div class="p-5 pb-3 flex justify-between items-start gap-4">
            <div>
                <h3 class="font-bold text-lg text-gray-900 leading-tight">{{ title }}</h3>
                <p v-if="description" class="text-sm text-gray-500 mt-1 line-clamp-2">{{ description }}</p>
            </div>
            <div v-if="status" class="shrink-0">
                <StatusBadge
                    :label="resolveStatus(status).label"
                    :variant="resolveStatus(status).variant"
                />
            </div>
        </div>

        <!-- Meta Data -->
        <div v-if="meta.length > 0" class="px-5 py-2 space-y-2 flex-1">
            <div v-for="(item, idx) in meta" :key="idx" class="flex items-center text-sm gap-2 text-gray-600">
                <Icon v-if="item.icon" :name="item.icon" :size="16" class="text-gray-400" />
                <span class="font-medium text-gray-500 min-w-[80px] text-xs uppercase tracking-wide">{{ item.label }}:</span>
                <span class="text-gray-900 truncate font-medium">{{ item.value }}</span>
            </div>
        </div>

        <div v-else class="flex-1"></div>

        <!-- Actions -->
        <div v-if="actions.length > 0" class="p-5 pt-3 mt-auto border-t border-gray-50 flex flex-wrap gap-2">
            <Button
                v-for="action in actions"
                :key="action.label"
                :variant="action.variant || 'secondary'"
                size="sm"
                :disabled="action.disabled"
                :loading="action.loading"
                :icon="action.icon"
                @click.stop="handleAction(action)"
                class="flex-1 justify-center"
            >
                {{ action.label }}
            </Button>
        </div>
    </div>
  </div>
</template>
