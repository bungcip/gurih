<script setup>
import Button from './Button.vue'

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
                <Button
                    variant="secondary"
                    @click="$emit('cancel')"
                >
                    {{ cancelText }}
                </Button>
                <Button
                    :variant="variant"
                    @click="$emit('confirm')"
                >
                    {{ confirmText }}
                </Button>
            </div>
        </div>
    </div>
</template>
