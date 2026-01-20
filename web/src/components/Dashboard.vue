<script setup>
import { ref, onMounted, watch } from 'vue'
import DashboardWidget from './DashboardWidget.vue'

const props = defineProps(['name'])
const config = ref(null)
const loading = ref(true)

const API_BASE = 'http://localhost:3000/api'

async function fetchDashboard() {
    loading.value = true
    try {
        const res = await fetch(`${API_BASE}/ui/dashboard/${props.name}`)
        const json = await res.json()
        if (json.error) {
            console.error("Dashboard error:", json.error)
            config.value = null
        } else {
            config.value = json
        }
    } catch (e) {
        console.error("Failed to fetch dashboard", e)
    } finally {
        loading.value = false
    }
}

watch(() => props.name, fetchDashboard)

onMounted(fetchDashboard)
</script>

<template>
    <div class="flex-1 flex flex-col min-h-0 bg-background overflow-y-auto">
        <div v-if="loading" class="p-12 text-center text-text-muted">
            Loading dashboard...
        </div>

        <div v-else-if="config" class="p-8">
            <h2 class="text-2xl font-bold text-text-main mb-6">{{ config.title }}</h2>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <div v-for="widget in config.widgets" :key="widget.name"
                     :class="{'col-span-1': true, 'lg:col-span-2': widget.type === 'bar' || widget.type === 'line' || widget.type === 'chart'}">
                     <div class="h-64">
                        <DashboardWidget :widget="widget" />
                     </div>
                </div>
            </div>
        </div>

        <div v-else class="p-12 text-center text-text-muted">
            Dashboard not found.
        </div>
    </div>
</template>
