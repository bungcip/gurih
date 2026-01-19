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
  <div class="flex h-screen w-full bg-background overflow-hidden">
    <!-- Sidebar -->
    <aside class="w-64 bg-white border-r border-border flex flex-col hidden md:flex">
        <div class="p-6 text-xl font-bold text-primary flex items-center gap-2">
            <div class="w-8 h-8 bg-primary rounded-lg flex items-center justify-center text-white text-sm">G</div>
            GurihERP
        </div>
        <nav class="flex-1 overflow-y-auto px-4 py-2 space-y-6">
            <div v-for="module in menu" :key="module.label">
                <div class="px-3 text-[10px] font-bold uppercase tracking-wider text-text-muted mb-2">{{ module.label }}</div>
                <div class="space-y-1">
                    <div v-for="item in module.items" :key="item.entity">
                        <button 
                            @click="navigateTo(item.entity)"
                            class="w-full sidebar-item"
                            :class="{'active': currentEntity === item.entity}"
                        >
                            {{ item.label }}
                        </button>
                    </div>
                </div>
            </div>
        </nav>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
        <header class="h-16 bg-white border-b border-border flex items-center justify-between px-8 shrink-0">
             <div class="flex items-center gap-4">
                 <h1 v-if="currentEntity" class="text-lg font-semibold text-text-main">{{ currentEntity }}</h1>
                 <h1 v-else class="text-lg font-semibold text-text-main">Dashboard</h1>
             </div>
             <div class="flex items-center gap-3">
                 <button v-if="viewMode === 'list' && currentEntity" @click="onAction('create')" class="btn-primary flex items-center gap-2 text-sm">
                    <span class="text-lg leading-none">+</span> New
                </button>
                <button v-if="viewMode !== 'list'" @click="onAction('cancel')" class="px-4 py-2 text-sm text-text-muted hover:text-text-main transition">
                    Back to List
                </button>
             </div>
        </header>

        <div class="flex-1 overflow-y-auto p-8">
            <div v-if="!currentEntity" class="max-w-6xl mx-auto">
                <div class="card p-12 text-center">
                    <h2 class="text-2xl font-bold mb-2">Welcome to GurihERP</h2>
                    <p class="text-text-muted">Select a module from the sidebar to manage your business data.</p>
                </div>
            </div>

            <div v-else class="max-w-6xl mx-auto h-full flex flex-col">
                <div v-if="viewMode === 'list'" class="card flex-1 overflow-hidden flex flex-col">
                    <DynamicPage :entity="currentEntity" @edit="(id) => onAction('edit', id)" />
                </div>
                
                <div v-if="viewMode === 'create' || viewMode === 'edit'" class="flex-1 overflow-y-auto pb-8">
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
