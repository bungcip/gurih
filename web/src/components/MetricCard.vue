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
    primary: 'text-blue-600 bg-blue-50 dark:text-blue-400 dark:bg-blue-900/20',
    success: 'text-green-600 bg-green-50 dark:text-green-400 dark:bg-green-900/20',
    warning: 'text-yellow-600 bg-yellow-50 dark:text-yellow-400 dark:bg-yellow-900/20',
    danger: 'text-red-600 bg-red-50 dark:text-red-400 dark:bg-red-900/20',
    info: 'text-cyan-600 bg-cyan-50 dark:text-cyan-400 dark:bg-cyan-900/20',
    gray: 'text-gray-600 bg-gray-50 dark:text-gray-400 dark:bg-gray-800'
}
</script>

<template>
    <div class="card p-5 flex flex-col justify-between h-full relative transition-all duration-200 hover:shadow-md"
         :class="{'border-red-200 bg-red-50/10 dark:border-red-800 dark:bg-red-900/10': error}">

        <!-- Loading State -->
        <div v-if="loading" class="animate-pulse flex space-x-4 w-full">
            <div class="flex-1 space-y-4 py-1">
                <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/3"></div>
                <div class="h-8 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
                <div class="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/4"></div>
            </div>
            <div class="h-10 w-10 bg-gray-200 dark:bg-gray-700 rounded-lg"></div>
        </div>

        <!-- Error State -->
        <div v-else-if="error" class="flex flex-col h-full justify-between">
             <div class="flex items-start justify-between">
                <div>
                    <p class="text-sm font-medium text-text-muted uppercase tracking-wider truncate">{{ label }}</p>
                    <h3 class="mt-2 text-lg font-medium text-red-600 dark:text-red-400 flex items-center gap-2">
                        <Icon name="alert-circle" :size="20" />
                        Error
                    </h3>
                </div>
            </div>
            <p class="text-xs text-red-500 dark:text-red-400 mt-2">{{ error }}</p>
        </div>

        <!-- Empty State -->
        <div v-else-if="empty || value === null || value === undefined" class="flex flex-col h-full justify-between">
            <div class="flex items-start justify-between">
                <div>
                    <p class="text-sm font-medium text-text-muted uppercase tracking-wider truncate">{{ label }}</p>
                    <h3 class="mt-2 text-2xl font-bold text-gray-300 dark:text-gray-600">--</h3>
                </div>
                 <div v-if="icon" class="p-3 rounded-lg flex items-center justify-center text-gray-300 dark:text-gray-600 bg-gray-50 dark:bg-gray-800">
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
                    <Icon v-if="trendDirection === 'up'" name="arrow-up" size="16" aria-hidden="true" />
                    <Icon v-else-if="trendDirection === 'down'" name="arrow-down" size="16" aria-hidden="true" />
                    <Icon v-else name="minus" size="16" aria-hidden="true" />

                    <span class="sr-only" v-if="trendDirection === 'up'">Trending up</span>
                    <span class="sr-only" v-else-if="trendDirection === 'down'">Trending down</span>
                    <span class="sr-only" v-else>No change</span>

                    {{ trend }}
                </span>
                <span class="text-text-muted opacity-80 ml-2 text-xs">vs last period</span>
            </div>
        </div>
    </div>
</template>
