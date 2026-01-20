<script setup>
import { ref, inject } from 'vue'
import Button from './Button.vue'

const username = ref('')
const password = ref('')
const loading = ref(false)
const error = ref('')

const emit = defineEmits(['login-success'])

const showToast = inject('showToast')

async function handleLogin() {
    loading.value = true
    error.value = ''
    try {
        const res = await fetch('/api/auth/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username: username.value, password: password.value })
        })
        const data = await res.json()

        if (res.ok) {
            emit('login-success', data)
            showToast('Login successful')
        } else {
            error.value = data.error || 'Login failed'
            showToast(error.value, 'error')
        }
    } catch (e) {
        error.value = 'Network error'
        showToast('Network error', 'error')
    } finally {
        loading.value = false
    }
}
</script>

<template>
    <div class="flex items-center justify-center min-h-screen bg-gray-100">
        <div class="w-full max-w-md bg-white rounded-lg shadow-md p-8">
            <h2 class="text-2xl font-bold text-center text-gray-800 mb-6">GurihERP Login</h2>
            <form @submit.prevent="handleLogin" class="space-y-4">
                <div>
                    <label for="username" class="block text-sm font-medium text-gray-700">Username</label>
                    <input
                        id="username"
                        v-model="username"
                        type="text"
                        class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                        required
                    />
                </div>
                <div>
                    <label for="password" class="block text-sm font-medium text-gray-700">Password</label>
                    <input
                        id="password"
                        v-model="password"
                        type="password"
                        class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
                        required
                    />
                </div>
                <div v-if="error" class="text-red-500 text-sm text-center">
                    {{ error }}
                </div>
                <Button
                    type="submit"
                    variant="primary"
                    :loading="loading"
                    class="w-full"
                >
                    Login
                </Button>
            </form>
        </div>
    </div>
</template>
