<script setup>
import { ref, computed, onUnmounted } from 'vue'

const props = defineProps({
    modelValue: {
        type: [Object, Array, null],
        default: null
    },
    label: {
        type: String,
        default: ''
    },
    accept: {
        type: String,
        default: '*/*'
    },
    multiple: {
        type: Boolean,
        default: false
    },
    maxSize: {
        type: Number, // in bytes
        default: 5 * 1024 * 1024 // 5MB default
    },
    error: {
        type: String,
        default: ''
    },
    disabled: {
        type: Boolean,
        default: false
    },
    id: {
        type: String,
        default: null
    }
})

const emit = defineEmits(['update:modelValue', 'change'])

const isDragging = ref(false)
const fileInput = ref(null)
const localError = ref('')

const files = computed(() => {
    if (!props.modelValue) return []

    const wrap = (val) => {
        if (typeof val === 'string') {
            const url = val
            const name = url.split('/').pop()
            return { name, url, isUrl: true, size: 0 }
        }
        return val
    }

    if (Array.isArray(props.modelValue)) {
        return props.modelValue.map(wrap)
    }
    return [wrap(props.modelValue)]
})

const fileIds = new WeakMap()
let idCounter = 0

function getFileKey(file, index) {
    if (!file) return index
    if (typeof file === 'string') return file
    if (file.url) return file.url
    if (file.id) return file.id

    // WeakMap only accepts objects (File is an object, but strings are not)
    if (typeof file === 'object') {
        let id = fileIds.get(file)
        if (!id) {
            id = `file-local-${idCounter++}`
            // Only set if file is a non-null object
            if (file) fileIds.set(file, id)
        }
        return id
    }
    return index
}

function formatSize(bytes) {
    if (!bytes || bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

function validateFile(file) {
    if (props.maxSize && file.size > props.maxSize) {
        return `File ${file.name} is too large (max ${formatSize(props.maxSize)})`
    }

    // Check file type based on accept prop
    if (props.accept && props.accept !== '*/*') {
        const acceptedTypes = props.accept.split(',').map(t => t.trim().toLowerCase())
        const fileType = (file.type || '').toLowerCase()
        const fileName = file.name.toLowerCase()

        const isValid = acceptedTypes.some(type => {
            if (type === '*/*') return true
            if (type.endsWith('/*')) {
                // e.g. image/*
                const prefix = type.slice(0, -2)
                return fileType.startsWith(prefix)
            }
            if (type.startsWith('.')) {
                // e.g. .jpg
                return fileName.endsWith(type)
            }
            // e.g. image/jpeg
            return fileType === type
        })

        if (!isValid) {
            return `File type not allowed: ${file.name}`
        }
    }

    return null
}

function handleFiles(newFiles) {
    localError.value = ''
    if (props.disabled) return

    const validFiles = []
    const errors = []

    Array.from(newFiles).forEach(file => {
        const error = validateFile(file)
        if (error) {
            errors.push(error)
        } else {
            validFiles.push(file)
        }
    })

    if (errors.length > 0) {
        localError.value = errors[0] // Show first error
    }

    if (validFiles.length > 0) {
        if (props.multiple) {
            const current = Array.isArray(props.modelValue) ? props.modelValue : (props.modelValue ? [props.modelValue] : [])
            emit('update:modelValue', [...current, ...validFiles])
        } else {
            emit('update:modelValue', validFiles[0])
        }
        emit('change', validFiles)
    }
}

function onDrop(e) {
    isDragging.value = false
    handleFiles(e.dataTransfer.files)
}

function onChange(e) {
    handleFiles(e.target.files)
    // reset input so same file can be selected again if needed (after removal)
    if (fileInput.value) fileInput.value.value = ''
}

const previewUrls = new Map()

function removeFile(index) {
    if (props.disabled) return

    if (props.multiple) {
        const newFiles = [...files.value]
        const fileToRemove = newFiles[index]

        // Clean up the object URL to avoid memory leaks
        if (fileToRemove && previewUrls.has(fileToRemove)) {
            URL.revokeObjectURL(previewUrls.get(fileToRemove))
            previewUrls.delete(fileToRemove)
        }

        newFiles.splice(index, 1)
        emit('update:modelValue', newFiles)
    } else {
        // Clean up the single file URL
        const fileToRemove = props.modelValue
        if (fileToRemove && previewUrls.has(fileToRemove)) {
            URL.revokeObjectURL(previewUrls.get(fileToRemove))
            previewUrls.delete(fileToRemove)
        }
        emit('update:modelValue', null)
    }
}

// Optimization: Cache object URLs to prevent creating new ones on every render
// and to allow proper cleanup.
function getPreviewUrl(file) {
    if (file && file.type && file.type.startsWith('image/')) {
        if (previewUrls.has(file)) {
            return previewUrls.get(file)
        }
        const url = URL.createObjectURL(file)
        previewUrls.set(file, url)
        return url
    }
    return null
}

onUnmounted(() => {
    // Revoke all URLs to prevent memory leaks when component is destroyed
    previewUrls.forEach(url => URL.revokeObjectURL(url))
    previewUrls.clear()
})
</script>

<template>
    <div class="w-full space-y-2">
        <!-- Note: If parent provides label via prop, we use it here. If parent provides label via slot/outside, it should use 'for' pointing to this ID. -->
        <label v-if="label" :for="id" class="block text-sm font-medium text-text-muted">
            {{ label }}
        </label>

        <div class="relative border-2 border-dashed rounded-lg p-6 transition-colors text-center cursor-pointer" :class="[
            isDragging ? 'border-primary bg-blue-50 dark:bg-blue-900/20' : 'border-gray-300 dark:border-gray-600 hover:border-gray-400 dark:hover:border-gray-500',
            disabled ? 'opacity-60 cursor-not-allowed bg-[--color-background]' : 'bg-[--color-surface]'
        ]" @dragenter.prevent="!disabled && (isDragging = true)" @dragleave.prevent="isDragging = false"
            @dragover.prevent @drop.prevent="onDrop" @click="!disabled && fileInput.click()"
            @keydown.enter="!disabled && fileInput.click()" @keydown.space="!disabled && fileInput.click()" tabindex="0"
            role="button" :aria-label="label ? 'Upload ' + label : 'Upload file'" :aria-disabled="disabled">
            <input :id="id" ref="fileInput" type="file" class="hidden" :accept="accept" :multiple="multiple"
                :disabled="disabled" @change="onChange" tabindex="-1">

            <div class="space-y-2 pointer-events-none">
                <div class="text-text-muted mx-auto">
                    <!-- Cloud Upload Icon -->
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5"
                        stroke="currentColor" class="w-10 h-10 mx-auto">
                        <path stroke-linecap="round" stroke-linejoin="round"
                            d="M12 16.5V9.75m0 0l3 3m-3-3l-3 3M6.75 19.5a4.5 4.5 0 01-1.41-8.775 5.25 5.25 0 0110.233-2.33 3 3 0 013.758 3.848A3.752 3.752 0 0118 19.5H6.75z" />
                    </svg>
                </div>
                <div class="text-sm text-text-muted">
                    <span class="font-semibold text-primary">Click to upload</span> or drag and drop
                </div>
                <div class="text-xs text-text-muted">
                    {{ multiple ? 'Multiple files allowed' : 'Single file' }} • Max {{ formatSize(maxSize) }}
                </div>
            </div>
        </div>

        <!-- Error Message -->
        <div v-if="localError || error" class="text-sm text-red-500 mt-1">
            {{ localError || error }}
        </div>

        <!-- File List -->
        <div v-if="files.length > 0" class="space-y-2 mt-3">
            <div v-for="(file, index) in files" :key="getFileKey(file, index)"
                class="flex items-center gap-3 p-3 bg-[--color-surface] border border-gray-200 dark:border-gray-700 rounded-lg group">
                <!-- Preview or Icon -->
                <div
                    class="w-10 h-10 shrink-0 rounded bg-gray-100 dark:bg-gray-800 flex items-center justify-center overflow-hidden border border-gray-200 dark:border-gray-700">
                    <img v-if="getPreviewUrl(file)" :src="getPreviewUrl(file)" class="w-full h-full object-cover"
                        alt="preview">
                    <svg v-else xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5"
                        stroke="currentColor" class="w-5 h-5 text-text-muted">
                        <path stroke-linecap="round" stroke-linejoin="round"
                            d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
                    </svg>
                </div>

                <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium text-text-main truncate">{{ file.name }}</p>
                    <p class="text-xs text-text-muted">{{ formatSize(file.size) }}</p>
                </div>

                <button type="button" @click="removeFile(index)" :disabled="disabled"
                    class="p-1 text-gray-400 hover:text-red-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    aria-label="Remove file">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5"
                        stroke="currentColor" class="w-5 h-5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>
        </div>
    </div>
</template>
