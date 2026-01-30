<script setup>
import { ref } from 'vue'
import Button from './Button.vue'
import Icon from './Icon.vue'

const props = defineProps({
  title: {
    type: String,
    default: 'Discussion'
  },
  items: {
    type: Array,
    default: () => []
  },
  placeholder: {
    type: String,
    default: 'Write a note...'
  },
  loading: {
    type: Boolean,
    default: false
  },
  readOnly: {
    type: Boolean,
    default: false
  },
  error: {
    type: String,
    default: ''
  },
  emptyText: {
    type: String,
    default: 'No notes yet.'
  }
})

const emit = defineEmits(['submit', 'delete'])

const newComment = ref('')

function handleSubmit() {
  const text = newComment.value.trim()
  if (!text) return

  emit('submit', text)
  newComment.value = ''
}

function handleDelete(id) {
    emit('delete', id)
}
</script>

<template>
  <div class="bg-[--color-surface] shadow rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden flex flex-col h-full bg-white dark:bg-[#1e1e1e]">
    <!-- Header -->
    <div class="px-4 py-3 border-b border-gray-100 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/50 flex justify-between items-center shrink-0">
      <h3 class="font-semibold text-text-main text-sm">{{ title }}</h3>
      <span v-if="items.length" class="bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-300 py-0.5 px-2 rounded-full text-xs font-medium">
          {{ items.length }}
      </span>
    </div>

    <!-- Body -->
    <div class="flex-1 overflow-y-auto min-h-[200px] p-4 space-y-4">
        <!-- Loading -->
        <div v-if="loading" class="space-y-4 animate-pulse">
             <div v-for="i in 3" :key="i" class="flex gap-3">
                 <div class="w-8 h-8 rounded-full bg-gray-200 dark:bg-gray-700 shrink-0"></div>
                 <div class="flex-1 space-y-2">
                     <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/4"></div>
                     <div class="h-10 bg-gray-200 dark:bg-gray-700 rounded w-full"></div>
                 </div>
             </div>
        </div>

        <!-- Error -->
        <div v-else-if="error" class="flex flex-col items-center justify-center h-full text-center p-4">
            <div class="text-red-500 mb-2">
                <Icon name="alert-circle" :size="24" />
            </div>
            <p class="text-sm text-text-muted">{{ error }}</p>
        </div>

        <!-- Empty -->
        <div v-else-if="items.length === 0" class="flex flex-col items-center justify-center h-full text-center p-4 text-text-muted">
            <div class="p-3 bg-gray-50 dark:bg-gray-800 rounded-full mb-2">
                <Icon name="users" :size="20" class="opacity-50" />
            </div>
            <p class="text-sm">{{ emptyText }}</p>
        </div>

        <!-- List -->
        <div v-else v-for="item in items" :key="item.id" class="flex gap-3 group">
            <div class="shrink-0">
                <div class="w-8 h-8 rounded-full bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-300 flex items-center justify-center">
                    <span v-if="item.avatar" class="text-xs font-bold">{{ item.avatar }}</span>
                    <Icon v-else name="user" :size="14" />
                </div>
            </div>
            <div class="flex-1 space-y-1">
                <div class="flex justify-between items-start">
                    <div class="flex items-center gap-2">
                        <span class="text-sm font-medium text-text-main">{{ item.author }}</span>
                        <span class="text-xs text-text-muted">{{ item.date }}</span>
                    </div>
                    <button
                        v-if="!readOnly"
                        @click="handleDelete(item.id)"
                        class="text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity p-1"
                        title="Delete"
                    >
                       <Icon name="minus" :size="14" />
                    </button>
                </div>
                <div class="text-sm text-text-main bg-gray-50 dark:bg-gray-800/50 rounded-lg p-3 rounded-tl-none break-words">
                    {{ item.content }}
                </div>
            </div>
        </div>
    </div>

    <!-- Footer / Input -->
    <div v-if="!readOnly && !loading && !error" class="p-4 border-t border-gray-100 dark:border-gray-700 bg-gray-50/30 dark:bg-gray-800/30 shrink-0">
        <div class="flex gap-2">
            <textarea
                v-model="newComment"
                :placeholder="placeholder"
                class="flex-1 min-h-[40px] max-h-[120px] p-2 text-sm border border-gray-200 dark:border-gray-700 rounded focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-[--color-surface] text-text-main resize-y"
                rows="1"
                @keydown.ctrl.enter="handleSubmit"
            ></textarea>
            <Button variant="primary" size="sm" :disabled="!newComment.trim()" @click="handleSubmit">
                Post
            </Button>
        </div>
        <p class="text-[10px] text-text-muted mt-1 text-right">Ctrl + Enter to send</p>
    </div>
  </div>
</template>
