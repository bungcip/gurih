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
    <div class="bg-white rounded-lg shadow overflow-hidden">
        <div v-if="loading" class="p-8 text-center text-gray-500">Loading...</div>
        
        <table v-else-if="config" class="w-full text-left">
            <thead class="bg-gray-100 border-b">
                <tr>
                    <th v-for="col in config.columns" :key="col.key" class="p-4 font-semibold text-sm text-gray-600">
                        {{ col.label }}
                    </th>
                    <th class="p-4 font-semibold text-sm text-gray-600 text-right">Actions</th>
                </tr>
            </thead>
            <tbody>
                <tr v-for="row in data" :key="row.id" class="border-b hover:bg-gray-50">
                    <td v-for="col in config.columns" :key="col.key" class="p-4 text-gray-800">
                        {{ row[col.key] }}
                    </td>
                    <td class="p-4 text-right space-x-2">
                        <button @click="$emit('edit', row.id)" class="text-blue-600 hover:text-blue-800 text-sm font-medium">Edit</button>
                        <button @click="deleteItem(row.id)" class="text-red-500 hover:text-red-700 text-sm font-medium">Delete</button>
                    </td>
                </tr>
                <tr v-if="data.length === 0">
                    <td :colspan="config.columns.length + 1" class="p-8 text-center text-gray-400">
                        No records found.
                    </td>
                </tr>
            </tbody>
        </table>
    </div>
</template>
