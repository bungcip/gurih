<script setup>
import { ref, onMounted } from 'vue'
import DynamicPage from './components/DynamicPage.vue'
import DynamicForm from './components/DynamicForm.vue'

const menu = ref([])
const currentEntity = ref(null)
const viewMode = ref('home') // home, list, create, edit
const editId = ref(null)

const API_BASE = 'http://localhost:3000/api'

async function fetchMenu() {
    try {
        const res = await fetch(`${API_BASE}/ui/portal`)
        menu.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch menu", e)
    }
}

function navigateTo(entity) {
    currentEntity.value = entity
    viewMode.value = 'list'
}

function onAction(action, id) {
    if (action === 'create') {
        viewMode.value = 'create'
        editId.value = null
    } else if (action === 'edit') {
        viewMode.value = 'edit'
        editId.value = id
    } else if (action === 'cancel') {
        viewMode.value = 'list'
    } else if (action === 'saved') {
        viewMode.value = 'list'
    }
}

onMounted(() => {
    fetchMenu()
})
</script>

<template>
  <div class="flex h-screen w-full">
    <!-- Sidebar -->
    <aside class="w-64 bg-primary text-white flex flex-col">
        <div class="p-4 text-xl font-bold bg-secondary">GurihERP</div>
        <nav class="flex-1 overflow-y-auto">
            <div v-for="module in menu" :key="module.label" class="py-2">
                <div class="px-4 text-xs font-semibold uppercase text-gray-400 mb-1">{{ module.label }}</div>
                <div v-for="item in module.items" :key="item.entity">
                    <button 
                        @click="navigateTo(item.entity)"
                        class="w-full text-left px-6 py-2 hover:bg-white/10 transition"
                        :class="{'bg-accent': currentEntity === item.entity}"
                    >
                        {{ item.label }}
                    </button>
                </div>
            </div>
        </nav>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 bg-gray-50 overflow-y-auto">
        <div class="p-8">
            <div v-if="!currentEntity" class="text-center text-gray-500 mt-20">
                <h2 class="text-2xl font-bold">Welcome to GurihERP</h2>
                <p>Select a module to get started.</p>
            </div>

            <div v-else>
                <div class="flex justify-between items-center mb-6">
                    <h1 class="text-3xl font-bold text-gray-800">{{ currentEntity }}</h1>
                    <button v-if="viewMode === 'list'" @click="onAction('create')" class="bg-accent hover:bg-blue-600 text-white px-4 py-2 rounded shadow">
                        + New {{ currentEntity }}
                    </button>
                    <button v-if="viewMode !== 'list'" @click="onAction('cancel')" class="text-gray-500 hover:text-gray-700">
                        Back to List
                    </button>
                </div>

                <div v-if="viewMode === 'list'">
                    <DynamicPage :entity="currentEntity" @edit="(id) => onAction('edit', id)" />
                </div>
                
                <div v-if="viewMode === 'create' || viewMode === 'edit'">
                    <DynamicForm 
                        :entity="currentEntity" 
                        :id="editId" 
                        @saved="onAction('saved')"
                        @cancel="onAction('cancel')"
                    />
                </div>
            </div>
        </div>
    </main>
  </div>
</template>
