<script setup>
import Icon from './Icon.vue'

defineProps({
  items: {
    type: Array,
    required: true,
    // Expected format: { label, href?, icon?, current? }
  },
  separator: {
    type: String,
    default: '/'
  },
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['navigate'])

function handleClick(item) {
  if (item.current || !item.href) return
  emit('navigate', item)
}
</script>

<template>
  <nav aria-label="Breadcrumb">
    <!-- Loading State -->
    <div v-if="loading" class="flex items-center space-x-2 animate-pulse">
        <div class="h-4 w-16 bg-gray-200 dark:bg-gray-700 rounded"></div>
        <div class="text-gray-300 dark:text-gray-600">/</div>
        <div class="h-4 w-24 bg-gray-200 dark:bg-gray-700 rounded"></div>
        <div class="text-gray-300 dark:text-gray-600">/</div>
        <div class="h-4 w-32 bg-gray-200 dark:bg-gray-700 rounded"></div>
    </div>

    <!-- Breadcrumb List -->
    <ol v-else class="flex items-center flex-wrap gap-2 text-sm text-text-muted">
      <li v-for="(item, index) in items" :key="index" class="flex items-center">
        <!-- Separator (except first item) -->
        <span
            v-if="index > 0"
            class="mx-1 text-gray-400 dark:text-gray-600 select-none"
            aria-hidden="true"
        >
            {{ separator }}
        </span>

        <!-- Item -->
        <component
            :is="item.href && !item.current ? 'a' : 'span'"
            :href="item.href && !item.current ? item.href : undefined"
            @click.prevent="handleClick(item)"
            class="flex items-center gap-1.5 transition-colors duration-200"
            :class="[
                item.current
                    ? 'font-medium text-text-main cursor-default'
                    : (item.href ? 'hover:text-primary cursor-pointer' : 'cursor-default')
            ]"
            :aria-current="item.current ? 'page' : undefined"
        >
            <Icon
                v-if="item.icon"
                :name="item.icon"
                :size="16"
                :class="item.current ? 'text-text-main' : 'text-current'"
            />
            <span>{{ item.label }}</span>
        </component>
      </li>
    </ol>
  </nav>
</template>
