<script setup>
import { ref, computed, watch } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
    src: {
        type: String,
        default: null
    },
    alt: {
        type: String,
        default: 'Avatar'
    },
    initials: {
        type: String,
        default: null
    },
    icon: {
        type: String,
        default: 'user'
    },
    size: {
        type: String,
        default: 'md',
        validator: (value) => ['xs', 'sm', 'md', 'lg', 'xl'].includes(value)
    },
    shape: {
        type: String,
        default: 'circle',
        validator: (value) => ['circle', 'square', 'rounded'].includes(value)
    },
    status: {
        type: String,
        default: null,
        validator: (value) => !value || ['online', 'offline', 'busy', 'away'].includes(value)
    },
    variant: {
        type: String,
        default: 'gray',
        validator: (value) => ['gray', 'primary', 'success', 'warning', 'danger', 'info'].includes(value)
    },
    loading: {
        type: Boolean,
        default: false
    }
})

const emit = defineEmits(['click', 'error'])

const hasError = ref(false)

watch(() => props.src, () => {
    hasError.value = false
})

function onError(e) {
    hasError.value = true
    emit('error', e)
}

const sizeClasses = computed(() => {
    const map = {
        xs: 'w-6 h-6 text-[10px]',
        sm: 'w-8 h-8 text-xs',
        md: 'w-10 h-10 text-sm',
        lg: 'w-12 h-12 text-base',
        xl: 'w-16 h-16 text-xl'
    }
    return map[props.size]
})

const shapeClasses = computed(() => {
    const map = {
        circle: 'rounded-full',
        square: 'rounded-none',
        rounded: 'rounded-md'
    }
    return map[props.shape]
})

const variantClasses = computed(() => {
    const map = {
        gray: 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300',
        primary: 'bg-blue-100 text-blue-600 dark:bg-blue-900/40 dark:text-blue-300',
        success: 'bg-green-100 text-green-600 dark:bg-green-900/40 dark:text-green-300',
        warning: 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900/40 dark:text-yellow-300',
        danger: 'bg-red-100 text-red-600 dark:bg-red-900/40 dark:text-red-300',
        info: 'bg-cyan-100 text-cyan-600 dark:bg-cyan-900/40 dark:text-cyan-300'
    }
    return map[props.variant]
})

const statusSizeClasses = computed(() => {
    const map = {
        xs: 'w-1.5 h-1.5',
        sm: 'w-2 h-2',
        md: 'w-2.5 h-2.5',
        lg: 'w-3 h-3',
        xl: 'w-3.5 h-3.5'
    }
    return map[props.size]
})

const statusColorClasses = computed(() => {
    const map = {
        online: 'bg-green-500',
        offline: 'bg-gray-400',
        busy: 'bg-red-500',
        away: 'bg-yellow-500'
    }
    return map[props.status]
})

const iconSize = computed(() => {
    const map = {
        xs: 12,
        sm: 14,
        md: 18,
        lg: 20,
        xl: 24
    }
    return map[props.size]
})

</script>

<template>
    <div class="relative inline-block align-middle group" @click="$emit('click')">
        <!-- Loading -->
        <div v-if="loading"
             :class="[sizeClasses, shapeClasses]"
             class="bg-gray-200 dark:bg-gray-700 animate-pulse"
        ></div>

        <!-- Avatar Container -->
        <div v-else
             :class="[
                sizeClasses,
                shapeClasses,
                variantClasses,
                'flex items-center justify-center overflow-hidden shrink-0 select-none relative ring-1 ring-black/5 dark:ring-white/10 transition-transform'
             ]"
        >
            <!-- Image -->
            <img
                v-if="src && !hasError"
                :src="src"
                :alt="alt"
                class="w-full h-full object-cover"
                @error="onError"
            />

            <!-- Initials -->
            <span v-else-if="initials" class="font-semibold uppercase leading-none tracking-wider">
                {{ initials.substring(0, 2) }}
            </span>

            <!-- Icon Fallback -->
            <Icon v-else :name="icon" :size="iconSize" />
        </div>

        <!-- Status Indicator -->
        <span v-if="status && !loading"
              :class="[
                statusColorClasses,
                statusSizeClasses,
                'absolute border-[1.5px] border-white dark:border-[#1e1e1e] rounded-full z-10 box-content'
              ]"
              :style="shape === 'circle' ? { bottom: '0', right: '0' } : { bottom: '-2px', right: '-2px' }"
        >
            <span class="sr-only">{{ status }}</span>
        </span>
    </div>
</template>
