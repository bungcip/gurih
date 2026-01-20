<script setup>
import { onMounted, onUnmounted } from 'vue'

const props = defineProps({
    isOpen: {
        type: Boolean,
        default: false
    },
    title: {
        type: String,
        default: ''
    },
    size: {
        type: String,
        default: 'md',
        validator: (value) => ['sm', 'md', 'lg', 'xl', 'full'].includes(value)
    }
})

const emit = defineEmits(['close'])

const sizes = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
    full: 'max-w-full m-4'
}

function handleEscape(e) {
    if (e.key === 'Escape' && props.isOpen) {
        emit('close')
    }
}

onMounted(() => window.addEventListener('keydown', handleEscape))
onUnmounted(() => window.removeEventListener('keydown', handleEscape))
</script>

<template>
    <Teleport to="body">
        <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6">
            <!-- Backdrop -->
            <transition
                enter-active-class="ease-out duration-300"
                enter-from-class="opacity-0"
                enter-to-class="opacity-100"
                leave-active-class="ease-in duration-200"
                leave-from-class="opacity-100"
                leave-to-class="opacity-0"
            >
                <div class="absolute inset-0 bg-black/40 backdrop-blur-sm" @click="$emit('close')"></div>
            </transition>
            
            <!-- Modal Container -->
            <transition
                enter-active-class="ease-out duration-300"
                enter-from-class="opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95"
                enter-to-class="opacity-100 translate-y-0 sm:scale-100"
                leave-active-class="ease-in duration-200"
                leave-from-class="opacity-100 translate-y-0 sm:scale-100"
                leave-to-class="opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95"
            >
                <div 
                    class="relative bg-white rounded-2xl shadow-2xl w-full flex flex-col max-h-[90vh] overflow-hidden"
                    :class="sizes[size]"
                >
                    <!-- Header -->
                    <div v-if="title || $slots.header" class="px-6 py-4 border-b border-gray-100 flex items-center justify-between shrink-0">
                        <slot name="header">
                            <h3 class="text-lg font-bold text-gray-900">{{ title }}</h3>
                        </slot>
                        <button 
                            @click="$emit('close')"
                            class="text-gray-400 hover:text-gray-600 p-1 hover:bg-gray-100 rounded-lg transition-colors"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    <!-- Body -->
                    <div class="px-6 py-6 overflow-y-auto flex-1">
                        <slot></slot>
                    </div>

                    <!-- Footer -->
                    <div v-if="$slots.footer" class="px-6 py-4 bg-gray-50 border-t border-gray-100 flex justify-end gap-3 shrink-0">
                        <slot name="footer"></slot>
                    </div>
                </div>
            </transition>
        </div>
    </Teleport>
</template>
