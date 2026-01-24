<script setup>
import { onMounted, onUnmounted, useId, ref, nextTick, watch } from 'vue'
import Button from './Button.vue'

const props = defineProps({
    isOpen: Boolean,
    title: String,
    message: String,
    confirmText: { type: String, default: 'Confirm' },
    cancelText: { type: String, default: 'Cancel' },
    variant: { type: String, default: 'primary' } // primary, danger
})

const emit = defineEmits(['confirm', 'cancel'])

const modalId = useId()
const cancelButtonRef = ref(null)

function handleEscape(e) {
    if (e.key === 'Escape' && props.isOpen) {
        emit('cancel')
    }
}

function onOpen() {
    window.addEventListener('keydown', handleEscape)
    nextTick(() => {
        if (cancelButtonRef.value?.$el?.focus) {
            cancelButtonRef.value.$el.focus()
        }
    })
}

function onClose() {
    window.removeEventListener('keydown', handleEscape)
}

watch(() => props.isOpen, (val) => {
    if (val) onOpen()
    else onClose()
})

onMounted(() => {
    if (props.isOpen) onOpen()
})

onUnmounted(() => {
    onClose()
})
</script>

<template>
    <Teleport to="body">
        <div
            v-if="isOpen"
            class="fixed inset-0 z-50 flex items-center justify-center p-4"
            role="alertdialog"
            aria-modal="true"
            :aria-labelledby="`${modalId}-title`"
            :aria-describedby="`${modalId}-desc`"
        >
            <!-- Backdrop -->
            <div class="absolute inset-0 bg-black/30 backdrop-blur-sm transition-opacity" @click="$emit('cancel')"></div>
            
            <!-- Modal -->
            <div class="relative bg-white rounded-xl shadow-xl w-full max-w-md transform transition-all scale-100 opacity-100 p-6 space-y-4">
                <h3 :id="`${modalId}-title`" class="text-xl font-bold text-gray-900">{{ title }}</h3>
                <p :id="`${modalId}-desc`" class="text-gray-600">{{ message }}</p>

                <div class="flex justify-end gap-3 pt-2">
                    <Button
                        ref="cancelButtonRef"
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
    </Teleport>
</template>
