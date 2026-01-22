<script setup>
const props = defineProps({
    toasts: {
        type: Array,
        default: () => []
    }
})

defineEmits(['remove'])

function getBgColor(type) {
    switch (type) {
        case 'error': return 'bg-red-500' 
        case 'warning': return 'bg-yellow-500'
        default: return 'bg-gray-800'
    }
}
</script>

<template>
    <div
        class="fixed bottom-4 right-4 z-[9999] flex flex-col gap-3 items-end pointer-events-none"
        aria-live="polite"
        aria-atomic="false"
    >
        <transition-group
            enter-active-class="transform ease-out duration-300 transition"
            enter-from-class="translate-y-2 opacity-0 sm:translate-y-0 sm:translate-x-4"
            enter-to-class="translate-y-0 opacity-100 sm:translate-x-0"
            leave-active-class="transition ease-in duration-200 absolute"
            leave-from-class="opacity-100"
            leave-to-class="opacity-0 translate-x-4"
            move-class="transition-transform duration-300 ease-in-out"
        >
            <div 
                v-for="toast in toasts" 
                :key="toast.id"
                class="pointer-events-auto group relative overflow-hidden"
                :role="toast.type === 'error' ? 'alert' : 'status'"
            >
                <div :class="[getBgColor(toast.type), 'text-white px-6 py-4 rounded-xl shadow-2xl flex items-center gap-4 min-w-[320px] max-w-md transition-all hover:scale-[1.02] active:scale-[0.98]']">
                    <div class="w-8 h-8 rounded-full bg-white/20 flex items-center justify-center shrink-0" aria-hidden="true">
                        <span class="text-lg">
                            <template v-if="toast.type === 'success'">✅</template>
                            <template v-else-if="toast.type === 'error'">❌</template>
                            <template v-else>ℹ️</template>
                        </span>
                    </div>
                    <div class="flex-1">
                        <p class="font-medium text-sm leading-tight">{{ toast.message }}</p>
                    </div>
                    <button 
                        @click="$emit('remove', toast.id)"
                        class="text-white/60 hover:text-white p-1 hover:bg-white/10 rounded-lg transition-colors"
                        aria-label="Close notification"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2.5" stroke="currentColor" class="w-4 h-4">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                    <!-- Progress Bar -->
                    <div class="absolute bottom-0 left-0 h-1 bg-white/30 progress-bar" aria-hidden="true"></div>
                </div>
            </div>
        </transition-group>
    </div>
</template>

<style scoped>
/* Ensure smooth transition for move */
.v-move {
  transition: all 0.3s ease;
}

.progress-bar {
    width: 100%;
    animation: shrink 3s linear forwards;
    transform-origin: left;
}

@keyframes shrink {
    from {
        transform: scaleX(1);
    }
    to {
        transform: scaleX(0);
    }
}
</style>
