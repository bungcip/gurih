<script setup>
import { ref, watch, onMounted, computed, inject } from 'vue'
import { request } from '../api.js'
import ConfirmModal from './ConfirmModal.vue'
import DataTable from './DataTable.vue'
import Button from './Button.vue'
import Dashboard from './Dashboard.vue'

const props = defineProps(['entity'])
const emit = defineEmits(['edit', 'create'])
const showToast = inject('showToast')
const currentUser = inject('currentUser')

const config = ref(null)
const data = ref([])
const loading = ref(false)

const pageActions = computed(() => {
    if (!config.value || !config.value.actions) return []
    return config.value.actions.filter(a => {
        const to = a.to || "";
        return !to.includes(":id")
    })
})

const rowActions = computed(() => {
    if (!config.value || !config.value.actions) return []
    return config.value.actions.filter(a => {
        const to = a.to || "";
        return to.includes(":id")
    })
})


async function fetchConfig() {
    try {
        const res = await request(`/ui/page/${props.entity}`)
        const json = await res.json()
        if (json.error) {
            console.error("Config error:", json.error)
            config.value = null
            return
        }
        config.value = json
    } catch (e) {
        console.error("Failed to fetch page config", e)
    }
}

async function fetchData() {
    if (!config.value || !config.value.entity) return
    loading.value = true
    try {
        const res = await request(`/${config.value.entity}`)
        data.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch data", e)
    } finally {
        loading.value = false
    }
}

async function loadPage() {
    await fetchConfig()
    if (config.value && config.value.layout === 'Grid') {
        return
    }
    await fetchData()
}



const modalState = ref({
    isOpen: false,
    title: '',
    message: '',
    action: null,
    row: null
})

async function handleCustomAction(action, row) {
    if (action.to) {
        let url = action.to;
        if (url.includes(':id')) {
            if (!row || !row.id) {
                if (showToast) showToast('Record ID missing. Action cannot be performed.', 'error')
                console.error('Cannot navigate: row ID is missing for action', action.label)
                return
            }
            url = url.replace(':id', row.id);
        }

        // Handle explicit Method (POST, DELETE, PUT)
        if (action.method && action.method.toUpperCase() !== 'GET') {
            if (action.variant === 'danger') {
                // Open Modal
                modalState.value = {
                    isOpen: true,
                    title: `Confirm ${action.label}`,
                    message: `Are you sure you want to ${action.label.toLowerCase()} this item? This action cannot be undone.`,
                    action: { ...action, url }, // Pass computed URL
                    row
                }
                return;
            }

            await executeAction(action, url, row);
            return;
        }

        if (action.to.includes("/new")) {
            emit('create')
            return
        }

        // Emit edit if it looks like a resource edit
        if (url.includes(row?.id) && (action.label.toLowerCase() === 'edit' || url.endsWith('/edit'))) {
            emit('edit', row.id)
            return
        }

        // Default: Navigation
    }
}

async function confirmAction() {
    const { action, row } = modalState.value;
    if (action) {
        await executeAction(action, action.url, row);
    }
    closeModal();
}

function closeModal() {
    modalState.value.isOpen = false;
    setTimeout(() => {
        modalState.value.action = null;
        modalState.value.row = null;
    }, 200);
}

async function executeAction(action, url, row) {
    try {
        const res = await request(url, {
            method: action.method.toUpperCase(),
            body: JSON.stringify(row || {})
        });

        if (res.ok) {
            const json = await res.json().catch(() => ({}));

            if (json.message) {
                showToast(json.message, 'success');
            } else {
                showToast(`${action.label} successful`, 'success');
            }
            fetchData();
        } else {
            const err = await res.json().catch(() => ({}));
            showToast("Action failed: " + (err.error || res.statusText), 'error');
        }
    } catch (e) {
        console.error(e);
        showToast("Action failed (network error)", 'error');
    }
}

watch(() => props.entity, () => {
    loadPage()
})

onMounted(() => {
    loadPage()
})
</script>

<template>
    <div class="flex-1 flex flex-col min-h-0">
        <div v-if="loading" class="p-12 text-center text-text-muted">
            <div class="animate-pulse flex flex-col items-center">
                <div class="h-8 w-32 bg-gray-100 rounded mb-4"></div>
                <div class="text-sm">Loading records...</div>
            </div>
        </div>

        <div v-else-if="config" class="flex-1 flex flex-col min-h-0">
            <!-- Dashboard Layout (Grid) -->
            <template v-if="config.layout === 'Grid'">
                <Dashboard :schema="config" />
            </template>

            <!-- Table/List Layout (Standard Card Look) -->
            <template v-else>
                <div class="card overflow-hidden bg-[--color-surface] flex-1 flex flex-col min-h-0">
                    <!-- Page Header -->
                    <div
                        class="p-6 px-8 border-b border-border bg-[--color-surface] flex justify-between items-center shrink-0">
                        <div>
                            <div class="text-[10px] font-bold uppercase tracking-widest text-text-muted mb-1">{{
                                props.entity }}</div>
                            <h2 class="text-xl font-bold text-text-main">{{ config.title || config.name }}</h2>
                        </div>
                        <!-- Page Actions -->
                        <div v-if="pageActions.length > 0" class="flex gap-3">
                            <Button v-for="action in pageActions" :key="action.label"
                                @click="handleCustomAction(action)"
                                :variant="action.variant === 'danger' ? 'danger' : 'primary'" :icon="action.icon">
                                {{ action.label }}
                            </Button>
                        </div>
                    </div>

                    <div class="flex-1 overflow-auto bg-[--color-surface]">
                        <!-- Table View -->
                        <template v-if="config.layout === 'TableView'">
                            <DataTable :columns="config.columns" :data="data" :actions="rowActions"
                                @action="handleCustomAction" />
                        </template>
                    </div>
                </div>
            </template>
        </div>
    </div>
    <ConfirmModal :is-open="modalState.isOpen" :title="modalState.title" :message="modalState.message" variant="danger"
        confirm-text="Delete" @confirm="confirmAction" @cancel="closeModal" />
</template>
