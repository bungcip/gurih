<script setup>
import { ref, watch, onMounted } from 'vue'

const props = defineProps(['entity'])
const emit = defineEmits(['edit'])

const config = ref(null)
const data = ref([])
const loading = ref(false)

const API_BASE = 'http://localhost:3000/api'

async function fetchConfig() {
    try {
        const res = await fetch(`${API_BASE}/ui/page/${props.entity}`)
        config.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch page config", e)
    }
}

async function fetchData() {
    loading.value = true
    try {
        const res = await fetch(`${API_BASE}/${props.entity}`)
        data.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch data", e)
    } finally {
        loading.value = false
    }
}

async function deleteItem(id) {
    if(!confirm("Are you sure?")) return;
    try {
        await fetch(`${API_BASE}/${props.entity}/${id}`, { method: 'DELETE' })
        fetchData()
    } catch (e) {
        alert("Failed to delete")
    }
}

watch(() => props.entity, () => {
    fetchConfig()
    fetchData()
})

onMounted(() => {
    fetchConfig()
    fetchData()
})
</script>

<template>
    <div class="flex-1 flex flex-col min-h-0 bg-white">
        <div v-if="loading" class="p-12 text-center text-text-muted">
            <div class="animate-pulse flex flex-col items-center">
                <div class="h-8 w-32 bg-gray-100 rounded mb-4"></div>
                <div class="text-sm">Loading records...</div>
            </div>
        </div>
        
        <div v-else-if="config" class="flex-1 overflow-auto">
            <table class="w-full text-left border-collapse">
                <thead class="bg-gray-50/50 sticky top-0 backdrop-blur-sm border-b border-border">
                    <tr>
                        <th v-for="col in config.columns" :key="col.key" class="p-4 px-6 font-bold text-[11px] uppercase tracking-wider text-text-muted">
                            {{ col.label }}
                        </th>
                        <th class="p-4 px-6 font-bold text-[11px] uppercase tracking-wider text-text-muted text-right">Actions</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-border">
                    <tr v-for="row in data" :key="row.id" class="group hover:bg-blue-50/30 transition-colors">
                        <td v-for="col in config.columns" :key="col.key" class="p-4 px-6 text-[14px] text-text-main">
                            {{ row[col.key] }}
                        </td>
                        <td class="p-4 px-6 text-right">
                            <div class="flex justify-end gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                <button @click="$emit('edit', row.id)" class="px-3 py-1 text-[13px] font-semibold text-primary hover:bg-blue-50 rounded-md transition">
                                    Edit
                                </button>
                                <button @click="deleteItem(row.id)" class="px-3 py-1 text-[13px] font-semibold text-red-500 hover:bg-red-50 rounded-md transition">
                                    Delete
                                </button>
                            </div>
                        </td>
                    </tr>
                    <tr v-if="data.length === 0">
                        <td :colspan="config.columns.length + 1" class="p-20 text-center">
                            <div class="flex flex-col items-center text-text-muted">
                                <div class="text-3xl mb-2">📁</div>
                                <div class="font-medium">No records found</div>
                                <div class="text-xs">Try adding a new record to get started.</div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>
        </div>
    </div>
</template>
