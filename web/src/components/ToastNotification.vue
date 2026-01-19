<script setup>
import { computed } from 'vue'

const props = defineProps({
    show: Boolean,
    message: String,
    type: {
        type: String,
        default: 'success'
    }
})

const bgColor = computed(() => {
    switch (props.type) {
        case 'error': return 'bg-red-500' // Red for error
        case 'warning': return 'bg-yellow-500' // Yellow for warning
        default: return 'bg-gray-800' // Dark for success/info (more sleek than green)
    }
})
</script>

<template>
    <transition
        enter-active-class="transform ease-out duration-300 transition"
        enter-from-class="translate-y-2 opacity-0 sm:translate-y-0 sm:translate-x-2"
        enter-to-class="translate-y-0 opacity-100 sm:translate-x-0"
        leave-active-class="transition ease-in duration-100"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
    >
        <div v-if="show" class="fixed bottom-4 right-4 z-50">
            <div :class="[bgColor, 'text-white px-6 py-3 rounded-lg shadow-lg flex items-center gap-3 min-w-[300px]']">
                <span class="text-xl">
                    <template v-if="type === 'success'">✅</template>
                    <template v-else-if="type === 'error'">❌</template>
                    <template v-else>ℹ️</template>
                </span>
                <span class="font-medium text-sm">{{ message }}</span>
            </div>
        </div>
    </transition>
</template>
