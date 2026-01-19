<script setup>
import { ref, watch, onMounted, computed } from 'vue'
import ConfirmModal from './ConfirmModal.vue'

const props = defineProps(['entity'])
const emit = defineEmits(['edit', 'create'])

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

const API_BASE = 'http://localhost:3000/api'

async function fetchConfig() {
    try {
        const res = await fetch(`${API_BASE}/ui/page/${props.entity}`)
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
        const res = await fetch(`${API_BASE}/${config.value.entity}`)
        data.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch data", e)
    } finally {
        loading.value = false
    }
}

async function loadPage() {
    await fetchConfig()
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
        if (row && row.id) {
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
        console.log("Navigating to", url);
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
        const res = await fetch(url.startsWith('http') ? url : `${API_BASE.replace('/api', '')}${url}`, { 
            method: action.method.toUpperCase(),
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(row || {}) 
        });
        
        if (res.ok) {
            const json = await res.json().catch(() => ({}));
            if(json.message) alert(json.message); // Optional: toast would be better
            fetchData(); // Refresh list
            emit('edit', null); 
        } else {
            const err = await res.json().catch(() => ({}));
            alert("Action failed: " + (err.error || res.statusText));
        }
    } catch (e) {
        console.error(e);
        alert("Action failed (network error)");
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
    <div class="flex-1 flex flex-col min-h-0 bg-white">
        <!-- Page Actions -->
        <div v-if="pageActions.length > 0" class="p-4 px-6 border-b border-border flex justify-end gap-3 shrink-0">
             <button 
                v-for="action in pageActions" 
                :key="action.label"
                @click="handleCustomAction(action)"
                class="btn-primary flex items-center gap-2 text-sm px-4 py-2"
                :class="action.variant === 'danger' ? 'bg-red-600 hover:bg-red-700 border-red-700' : ''"
            >
                <span v-if="action.icon === 'plus'" class="text-lg leading-none">+</span>
                {{ action.label }}
            </button>
        </div>

        <div v-if="loading" class="p-12 text-center text-text-muted">
            <div class="animate-pulse flex flex-col items-center">
                <div class="h-8 w-32 bg-gray-100 rounded mb-4"></div>
                <div class="text-sm">Loading records...</div>
            </div>
        </div>
        
        <div v-else-if="config" class="flex-1 flex flex-col min-h-0">
            <!-- Page Header -->
            <div class="p-6 px-8 border-b border-border bg-white flex justify-between items-center shrink-0">
                <div>
                    <div class="text-[10px] font-bold uppercase tracking-widest text-text-muted mb-1">{{ props.entity }}</div>
                    <h2 class="text-xl font-bold text-text-main">{{ config.title || config.name }}</h2>
                </div>
                <!-- Page Actions (moved inside header) -->
                <div v-if="pageActions.length > 0" class="flex gap-3">
                     <button 
                        v-for="action in pageActions" 
                        :key="action.label"
                        @click="handleCustomAction(action)"
                        class="btn-primary flex items-center gap-2 text-sm px-4 py-2"
                        :class="action.variant === 'danger' ? 'bg-red-600 hover:bg-red-700 border-red-700' : ''"
                    >
                        <span v-if="action.icon === 'plus'" class="text-lg leading-none">+</span>
                        {{ action.label }}
                    </button>
                </div>
            </div>

            <div class="flex-1 overflow-auto bg-white">
                <!-- Table View -->
                <template v-if="config.layout === 'TableView'">
                    <table class="w-full text-left border-collapse">
                        <thead class="bg-gray-50/50 sticky top-0 backdrop-blur-sm border-b border-border shadow-sm">
                            <tr>
                                <th v-for="col in config.columns" :key="col.key" class="p-4 px-8 font-bold text-[11px] uppercase tracking-wider text-text-muted">
                                    {{ col.label }}
                                </th>
                                <th class="p-4 px-8 font-bold text-[11px] uppercase tracking-wider text-text-muted text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-border">
                            <tr v-for="row in data" :key="row.id" class="group hover:bg-blue-50/30 transition-colors">
                                <td v-for="col in config.columns" :key="col.key" class="p-4 px-8 text-[14px] text-text-main">
                                    {{ row[col.key] }}
                                </td>
                                <td class="p-4 px-8 text-right">
                                    <div class="flex justify-end gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                        <template v-for="action in rowActions" :key="action.label">
                                            <button 
                                                @click="handleCustomAction(action, row)"
                                                class="px-3 py-1 text-[13px] font-semibold rounded-md transition"
                                                :class="action.variant === 'danger' ? 'text-red-500 hover:bg-red-50' : 'text-primary hover:bg-blue-50'"
                                            >
                                                {{ action.label }}
                                            </button>
                                        </template>
                                    </div>
                                </td>
                            </tr>
                            <tr v-if="data.length === 0">
                                <td :colspan="config.columns ? config.columns.length + 1 : 1" class="p-20 text-center">
                                    <div class="flex flex-col items-center text-text-muted">
                                        <div class="text-3xl mb-2">📁</div>
                                        <div class="font-medium">No records found</div>
                                        <div class="text-xs">Try adding a new record to get started.</div>
                                    </div>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </template>

                <!-- Dashboard View -->
                <template v-else-if="config.layout === 'Grid'">
                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 p-8">
                        <div v-for="widget in config.widgets" :key="widget.name" class="card p-8 flex flex-col justify-between hover:border-primary/30 transition-colors">
                            <div>
                                <div class="text-xs font-bold uppercase tracking-wider text-text-muted mb-4">{{ widget.label }}</div>
                                <div class="text-3xl font-bold text-text-main">{{ widget.value }}</div>
                            </div>
                            <div class="mt-4 flex justify-end">
                                <div class="w-10 h-10 bg-blue-50 rounded-lg flex items-center justify-center text-primary">
                                    <!-- Render icon if available -->
                                    <span class="text-xl">📊</span>
                                </div>
                            </div>
                        </div>
                    </div>
                </template>
            </div>
        </div>
    </div>
    <ConfirmModal 
        :is-open="modalState.isOpen"
        :title="modalState.title"
        :message="modalState.message"
        variant="danger"
        confirm-text="Delete"
        @confirm="confirmAction"
        @cancel="closeModal"
    />
</template>
