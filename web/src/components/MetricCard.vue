<script setup>
import Icon from './Icon.vue'

defineProps({
    label: {
        type: String,
        required: true
    },
    value: {
        type: [String, Number],
        default: null
    },
    trend: {
        type: [String, Number],
        default: null
    },
    trendDirection: {
        type: String,
        default: 'neutral',
        validator: (value) => ['up', 'down', 'neutral'].includes(value)
    },
    icon: {
        type: String,
        default: null
    },
    variant: {
        type: String,
        default: 'primary'
    },
    loading: {
        type: Boolean,
        default: false
    },
    error: {
        type: String,
        default: null
    },
    empty: {
        type: Boolean,
        default: false
    },
    emptyText: {
        type: String,
        default: 'No data available'
    }
})

const variantClasses = {
    primary: 'text-blue-600 bg-blue-50',
    success: 'text-green-600 bg-green-50',
    warning: 'text-yellow-600 bg-yellow-50',
    danger: 'text-red-600 bg-red-50',
    info: 'text-cyan-600 bg-cyan-50',
    gray: 'text-gray-600 bg-gray-50'
}
</script>

<template>
    <div class="card p-5 flex flex-col justify-between h-full relative transition-all duration-200 hover:shadow-md"
         :class="{'border-red-200 bg-red-50/10': error}">

        <!-- Loading State -->
        <div v-if="loading" class="animate-pulse flex space-x-4 w-full">
            <div class="flex-1 space-y-4 py-1">
                <div class="h-3 bg-gray-200 rounded w-1/3"></div>
                <div class="h-8 bg-gray-200 rounded w-1/2"></div>
                <div class="h-3 bg-gray-200 rounded w-1/4"></div>
            </div>
            <div class="h-10 w-10 bg-gray-200 rounded-lg"></div>
        </div>

        <!-- Error State -->
        <div v-else-if="error" class="flex flex-col h-full justify-between">
             <div class="flex items-start justify-between">
                <div>
                    <p class="text-sm font-medium text-text-muted uppercase tracking-wider truncate">{{ label }}</p>
                    <h3 class="mt-2 text-lg font-medium text-red-600 flex items-center gap-2">
                        <Icon name="alert-circle" :size="20" />
                        Error
                    </h3>
                </div>
            </div>
            <p class="text-xs text-red-500 mt-2">{{ error }}</p>
        </div>

        <!-- Empty State -->
        <div v-else-if="empty || value === null || value === undefined" class="flex flex-col h-full justify-between">
            <div class="flex items-start justify-between">
                <div>
                    <p class="text-sm font-medium text-text-muted uppercase tracking-wider truncate">{{ label }}</p>
                    <h3 class="mt-2 text-2xl font-bold text-gray-300">--</h3>
                </div>
                 <div v-if="icon" class="p-3 rounded-lg flex items-center justify-center text-gray-300 bg-gray-50">
                    <Icon :name="icon" :size="24" />
                </div>
            </div>
             <p class="text-xs text-text-muted mt-2">{{ emptyText }}</p>
        </div>

        <!-- Content -->
        <div v-else class="flex flex-col h-full justify-between">
            <div class="flex items-start justify-between">
                <div>
                    <p class="text-sm font-medium text-text-muted uppercase tracking-wider truncate">{{ label }}</p>
                    <h3 class="mt-2 text-2xl font-bold text-text-main">{{ value }}</h3>
                </div>

                <div v-if="icon" :class="['p-3 rounded-lg flex items-center justify-center', variantClasses[variant] || variantClasses.primary]">
                    <Icon :name="icon" :size="24" />
                </div>
            </div>

            <div v-if="trend" class="mt-4 flex items-center text-sm">
                <span
                    :class="{
                        'text-green-600': trendDirection === 'up',
                        'text-red-600': trendDirection === 'down',
                        'text-gray-500': trendDirection === 'neutral'
                    }"
                    class="font-medium flex items-center gap-1"
                >
                    <span v-if="trendDirection === 'up'">↑</span>
                    <span v-else-if="trendDirection === 'down'">↓</span>
                    {{ trend }}
                </span>
                <span class="text-text-muted opacity-80 ml-2 text-xs">vs last period</span>
            </div>
        </div>
    </div>
</template>
