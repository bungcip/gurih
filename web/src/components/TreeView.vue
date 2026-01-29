<script setup>
import { computed, inject, provide } from 'vue'
import Icon from './Icon.vue'

defineOptions({
  name: 'TreeView'
})

const props = defineProps({
  nodes: {
    type: Array,
    default: () => []
  },
  modelValue: {
    type: [String, Number],
    default: null
  },
  expandedKeys: {
    type: Array,
    default: () => []
  },
  loading: {
    type: Boolean,
    default: false
  },
  emptyText: {
    type: String,
    default: 'No items found'
  },
  error: {
    type: String,
    default: ''
  },
  depth: {
    type: Number,
    default: 0
  },
  itemKey: {
    type: String,
    default: 'id'
  },
  itemLabel: {
    type: String,
    default: 'label'
  },
  itemChildren: {
    type: String,
    default: 'children'
  },
  itemIcon: {
    type: String,
    default: 'icon'
  }
})

const emit = defineEmits(['update:modelValue', 'update:expandedKeys', 'node-click'])

// Bolt Optimization: Use a shared Set for O(1) lookup of expanded keys across recursive components
// instead of O(N) Array.includes at every level.
const parentExpandedSet = inject('expandedKeysSet', null)
const localExpandedSet = computed(() => new Set(props.expandedKeys))

const expandedSet = computed(() => {
  // If we are a child node (depth > 0) and have a parent provider, use it.
  if (props.depth > 0 && parentExpandedSet) {
    return parentExpandedSet.value
  }
  return localExpandedSet.value
})

if (props.depth === 0) {
  provide('expandedKeysSet', localExpandedSet)
}

function isExpanded(node) {
  const key = node[props.itemKey]
  return expandedSet.value.has(key)
}

function toggleNode(node, event) {
  event.stopPropagation()
  const key = node[props.itemKey]
  const isCurrentlyExpanded = props.expandedKeys.includes(key)
  let newExpandedKeys
  if (isCurrentlyExpanded) {
    newExpandedKeys = props.expandedKeys.filter(k => k !== key)
  } else {
    newExpandedKeys = [...props.expandedKeys, key]
  }
  emit('update:expandedKeys', newExpandedKeys)
}

function selectNode(node) {
  const key = node[props.itemKey]
  emit('update:modelValue', key)
  emit('node-click', node)
}

// Pass events up from children
function onUpdateExpandedKeys(newKeys) {
  emit('update:expandedKeys', newKeys)
}

function onUpdateModelValue(newValue) {
  emit('update:modelValue', newValue)
}

function onNodeClick(node) {
  emit('node-click', node)
}
</script>

<template>
  <div class="tree-view text-sm">
    <!-- Loading State (only at root) -->
    <div v-if="loading && depth === 0" class="space-y-2 animate-pulse p-4">
       <div v-for="i in 3" :key="i" class="flex items-center gap-2">
           <div class="w-4 h-4 bg-gray-200 dark:bg-gray-700 rounded"></div>
           <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/3"></div>
       </div>
    </div>

    <!-- Error State (only at root) -->
    <div v-else-if="error && depth === 0" class="p-4 flex flex-col items-center justify-center text-center text-red-600 dark:text-red-400 border border-red-200 dark:border-red-800 bg-red-50/50 dark:bg-red-900/10 rounded">
         <Icon name="alert-circle" :size="24" class="mb-2" />
         <p>{{ error }}</p>
    </div>

    <!-- Empty State (only at root) -->
    <div v-else-if="nodes.length === 0 && depth === 0" class="p-4 text-center text-text-muted border border-dashed border-gray-200 dark:border-gray-700 rounded bg-gray-50/50 dark:bg-gray-800/50">
      {{ emptyText }}
    </div>

    <!-- Tree Nodes -->
    <ul v-else class="space-y-0.5">
      <li v-for="node in nodes" :key="node[itemKey]">
        <div
            class="group flex items-center py-1.5 pr-2 rounded cursor-pointer select-none transition-colors duration-150"
            :class="[
                modelValue === node[itemKey]
                    ? 'bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300 font-medium'
                    : 'text-text-main hover:bg-gray-100 dark:hover:bg-gray-800'
            ]"
            :style="{ paddingLeft: (depth * 1.5) + 0.5 + 'rem' }"
            @click="selectNode(node)"
        >
            <!-- Toggle Icon -->
            <button
                type="button"
                class="w-6 h-6 flex items-center justify-center rounded hover:bg-black/5 dark:hover:bg-white/10 mr-1 focus:outline-none focus:ring-2 focus:ring-blue-500/40"
                @click.stop="toggleNode(node, $event)"
                :class="{ 'invisible': !node[itemChildren] || node[itemChildren].length === 0 }"
                aria-label="Toggle"
            >
                <svg
                    class="w-4 h-4 transition-transform duration-200 text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-400"
                    :class="{ 'rotate-90': isExpanded(node) }"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
            </button>

            <!-- Node Icon -->
            <div v-if="node[itemIcon]" class="mr-2 text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-400" :class="{ 'text-blue-500 dark:text-blue-400': modelValue === node[itemKey] }">
                <Icon :name="node[itemIcon]" :size="16" />
            </div>

            <!-- Label -->
            <span class="truncate">{{ node[itemLabel] }}</span>
        </div>

        <!-- Children (Recursive) -->
        <div v-if="isExpanded(node) && node[itemChildren] && node[itemChildren].length > 0" class="overflow-hidden">
            <TreeView
                :nodes="node[itemChildren]"
                :depth="depth + 1"
                :modelValue="modelValue"
                :expandedKeys="expandedKeys"
                :itemKey="itemKey"
                :itemLabel="itemLabel"
                :itemChildren="itemChildren"
                :itemIcon="itemIcon"
                @update:modelValue="onUpdateModelValue"
                @update:expandedKeys="onUpdateExpandedKeys"
                @node-click="onNodeClick"
            />
        </div>
      </li>
    </ul>
  </div>
</template>
