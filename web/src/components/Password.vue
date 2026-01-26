<script setup>
import { ref } from 'vue'

const props = defineProps({
    modelValue: {
        type: String,
        default: ''
    },
    label: {
        type: String,
        default: ''
    },
    placeholder: {
        type: String,
        default: ''
    },
    disabled: {
        type: Boolean,
        default: false
    },
    id: {
        type: String,
        default: () => 'password-' + Math.random().toString(36).substr(2, 9)
    }
})

const emit = defineEmits(['update:modelValue'])

const showPassword = ref(false)

function togglePassword() {
    if (props.disabled) return
    showPassword.value = !showPassword.value
}
</script>

<template>
    <div class="w-full">
        <label v-if="label" :for="id" class="block text-sm font-medium text-text-muted mb-1">
            {{ label }}
        </label>
        <div class="relative">
            <input
                :id="id"
                :type="showPassword ? 'text' : 'password'"
                :value="modelValue"
                @input="$emit('update:modelValue', $event.target.value)"
                :placeholder="placeholder"
                :disabled="disabled"
                class="input-field pr-10"
                :class="{ 'bg-[--color-background] cursor-not-allowed text-text-muted': disabled }"
            />
            <button
                type="button"
                @click="togglePassword"
                class="absolute inset-y-0 right-0 max-h-full flex items-center pr-3 text-text-muted hover:text-text-main focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1 rounded-md"
                :disabled="disabled"
                :class="{ 'cursor-not-allowed opacity-50': disabled }"
                :aria-label="showPassword ? 'Hide password' : 'Show password'"
                :aria-pressed="showPassword"
            >
                <!-- Eye Icon -->
                <svg v-if="!showPassword" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" />
                    <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                <!-- Eye Slash Icon -->
                <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" />
                </svg>
            </button>
        </div>
    </div>
</template>
