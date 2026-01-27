<script setup>
import { computed, onMounted, onUnmounted, watch } from 'vue'

const props = defineProps({
  isOpen: {
    type: Boolean,
    default: false
  },
  title: {
    type: String,
    default: ''
  },
  placement: {
    type: String,
    default: 'right',
    validator: (v) => ['left', 'right'].includes(v)
  },
  size: {
    type: String,
    default: 'md',
    validator: (v) => ['sm', 'md', 'lg', 'xl', '2xl', 'full'].includes(v)
  },
  closable: {
    type: Boolean,
    default: true
  },
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['close'])

const sizeClasses = {
  sm: 'max-w-sm',
  md: 'max-w-md',
  lg: 'max-w-lg',
  xl: 'max-w-xl',
  '2xl': 'max-w-2xl',
  full: 'w-screen'
}

const transitionClasses = computed(() => {
  if (props.placement === 'left') {
    return {
      enterFrom: '-translate-x-full',
      enterTo: 'translate-x-0',
      leaveFrom: 'translate-x-0',
      leaveTo: '-translate-x-full'
    }
  }
  return {
    enterFrom: 'translate-x-full',
    enterTo: 'translate-x-0',
    leaveFrom: 'translate-x-0',
    leaveTo: 'translate-x-full'
  }
})

function close() {
  emit('close')
}

function handleEscape(e) {
  if (e.key === 'Escape' && props.isOpen) {
    close()
  }
}

// Lock body scroll when open
watch(() => props.isOpen, (val) => {
    if (typeof document !== 'undefined') {
        if (val) {
            document.body.style.overflow = 'hidden'
        } else {
            document.body.style.overflow = ''
        }
    }
})

onMounted(() => window.addEventListener('keydown', handleEscape))
onUnmounted(() => {
    window.removeEventListener('keydown', handleEscape)
    if (typeof document !== 'undefined') {
        document.body.style.overflow = ''
    }
})
</script>

<template>
  <Teleport to="body">
    <!-- Root: Always rendered but transparent to events when closed -->
    <div
        class="fixed inset-0 overflow-hidden z-[60] pointer-events-none"
        role="dialog"
        aria-modal="true"
    >
      <div class="absolute inset-0 overflow-hidden">
        <!-- Backdrop -->
        <transition
            enter-active-class="ease-in-out duration-500"
            enter-from-class="opacity-0"
            enter-to-class="opacity-100"
            leave-active-class="ease-in-out duration-500"
            leave-from-class="opacity-100"
            leave-to-class="opacity-0"
        >
            <div
                v-if="isOpen"
                class="absolute inset-0 bg-gray-500 bg-opacity-75 dark:bg-black/70 backdrop-blur-sm transition-opacity pointer-events-auto"
                @click="close"
            ></div>
        </transition>

        <!-- Panel Wrapper -->
        <div
            class="pointer-events-none fixed inset-y-0 flex max-w-full"
            :class="placement === 'left' ? 'left-0 pr-10' : 'right-0 pl-10'"
        >
            <transition
                enter-active-class="transform transition ease-in-out duration-500 sm:duration-700"
                :enter-from-class="transitionClasses.enterFrom"
                :enter-to-class="transitionClasses.enterTo"
                leave-active-class="transform transition ease-in-out duration-500 sm:duration-700"
                :leave-from-class="transitionClasses.leaveFrom"
                :leave-to-class="transitionClasses.leaveTo"
            >
                <!-- Panel -->
                <div
                    v-if="isOpen"
                    class="pointer-events-auto relative w-screen"
                    :class="sizeClasses[size]"
                >
                    <div class="flex h-full flex-col overflow-y-scroll bg-[--color-surface] shadow-xl">

                        <!-- Header -->
                        <div v-if="title || $slots.header" class="px-4 sm:px-6 py-6 border-b border-gray-200 dark:border-gray-700 shrink-0">
                             <div class="flex items-start justify-between">
                                <h2 class="text-lg font-semibold text-text-main leading-6">
                                    <slot name="header">{{ title }}</slot>
                                </h2>
                                <div class="ml-3 flex h-7 items-center" v-if="closable">
                                    <button
                                        type="button"
                                        class="rounded-md bg-transparent text-gray-400 hover:text-gray-500 focus:outline-none focus:ring-2 focus:ring-primary"
                                        @click="close"
                                    >
                                        <span class="sr-only">Close panel</span>
                                        <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" aria-hidden="true">
                                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                                        </svg>
                                    </button>
                                </div>
                             </div>
                        </div>

                        <!-- Body -->
                        <div class="relative mt-6 flex-1 px-4 sm:px-6">
                            <div v-if="loading" class="space-y-4 animate-pulse">
                                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
                                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
                                <div class="h-32 bg-gray-200 dark:bg-gray-700 rounded"></div>
                                <div class="h-4 bg-gray-200 dark:bg-gray-700 rounded w-5/6"></div>
                            </div>
                            <slot v-else></slot>
                        </div>

                        <!-- Footer -->
                        <div v-if="$slots.footer" class="flex shrink-0 justify-end px-4 py-4 gap-2 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                            <slot name="footer"></slot>
                        </div>
                    </div>
                </div>
            </transition>
        </div>
      </div>
    </div>
  </Teleport>
</template>
