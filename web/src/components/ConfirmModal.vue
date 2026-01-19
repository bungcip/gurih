<script setup>
defineProps({
    isOpen: Boolean,
    title: String,
    message: String,
    confirmText: { type: String, default: 'Confirm' },
    cancelText: { type: String, default: 'Cancel' },
    variant: { type: String, default: 'primary' } // primary, danger
})

defineEmits(['confirm', 'cancel'])
</script>

<template>
    <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/30 backdrop-blur-sm transition-opacity" @click="$emit('cancel')"></div>
        
        <!-- Modal -->
        <div class="relative bg-white rounded-xl shadow-xl w-full max-w-md transform transition-all scale-100 opacity-100 p-6 space-y-4">
            <h3 class="text-xl font-bold text-gray-900">{{ title }}</h3>
            <p class="text-gray-600">{{ message }}</p>
            
            <div class="flex justify-end gap-3 pt-2">
                <button 
                    @click="$emit('cancel')"
                    class="px-4 py-2 text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg font-medium transition-colors"
                >
                    {{ cancelText }}
                </button>
                <button 
                    @click="$emit('confirm')"
                    class="px-4 py-2 text-white rounded-lg font-medium transition-colors shadow-sm"
                    :class="variant === 'danger' ? 'bg-red-600 hover:bg-red-700' : 'bg-blue-600 hover:bg-blue-700'"
                >
                    {{ confirmText }}
                </button>
            </div>
        </div>
    </div>
</template>
