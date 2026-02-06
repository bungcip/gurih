<script setup>
import { ref, onMounted, provide } from 'vue'
import DynamicPage from './components/DynamicPage.vue'
import DynamicForm from './components/DynamicForm.vue'
import ToastNotification from './components/ToastNotification.vue'
import Login from './components/Login.vue'
import Dashboard from './components/Dashboard.vue'

const menu = ref([])
const currentEntity = ref(null)
const viewMode = ref('home') // home, list, create, edit
const editId = ref(null)
const currentUser = ref(null)
const dashboardSchema = ref(null)

// Toast State
const toasts = ref([])

function showToast(message, type = 'success') {
    const id = Date.now()
    toasts.value.push({ id, message, type })
    
    // Auto remove after 3 seconds
    setTimeout(() => {
        toasts.value = toasts.value.filter(t => t.id !== id)
    }, 3000)
}

function removeToast(id) {
    toasts.value = toasts.value.filter(t => t.id !== id)
}

provide('showToast', showToast)
provide('removeToast', removeToast)
provide('currentUser', currentUser)

const API_BASE = 'http://localhost:3000/api'

async function fetchMenu() {
    try {
        const headers = currentUser.value && currentUser.value.token ? {
            'Authorization': `Bearer ${currentUser.value.token}`
        } : {}

        const res = await fetch(`${API_BASE}/ui/portal`, { headers })
        menu.value = await res.json()
        
        // Also fetch dashboard schema
        const dashRes = await fetch(`${API_BASE}/ui/dashboard/HRDashboard`, { headers })
        dashboardSchema.value = await dashRes.json()
    } catch (e) {
        console.error("Failed to fetch menu or dashboard", e)
    }
}

function handleLoginSuccess(user) {
    currentUser.value = user
    localStorage.setItem('user', JSON.stringify(user))
    fetchMenu()
}

function handleLogout() {
    currentUser.value = null
    localStorage.removeItem('user')
    viewMode.value = 'home'
    window.location.hash = ''
}

function syncHashToState() {
    const hash = window.location.hash.slice(1) // Remove #
    if (!hash) {
        viewMode.value = 'home'
        currentEntity.value = null
        return
    }

    // Expected formats: 
    // /app/:entity
    // /app/:entity/new
    // /app/:entity/:id
    const parts = hash.split('/').filter(p => p)
    if (parts[0] === 'app' && parts[1]) {
        currentEntity.value = parts[1]
        if (parts[2] === 'new') {
            viewMode.value = 'create'
            editId.value = null
        } else if (parts[2]) {
            viewMode.value = 'edit'
            editId.value = parts[2]
        } else {
            viewMode.value = 'list'
        }
    } else {
        viewMode.value = 'home'
        currentEntity.value = null
    }
}

function updateHash() {
    if (viewMode.value === 'home') {
        window.location.hash = ''
    } else if (viewMode.value === 'list') {
        window.location.hash = `/app/${currentEntity.value}`
    } else if (viewMode.value === 'create') {
        window.location.hash = `/app/${currentEntity.value}/new`
    } else if (viewMode.value === 'edit') {
        window.location.hash = `/app/${currentEntity.value}/${editId.value}`
    }
}

function navigateTo(entity) {
    currentEntity.value = entity
    viewMode.value = 'list'
    updateHash()
}

function onAction(action, id) {
    if (action === 'create') {
        viewMode.value = 'create'
        editId.value = null
    } else if (action === 'edit') {
        viewMode.value = 'edit'
        editId.value = id
    } else if (action === 'cancel' || action === 'saved') {
        viewMode.value = 'list'
    }
    updateHash()
}

onMounted(() => {
    const stored = localStorage.getItem('user')
    if (stored) {
        try {
            currentUser.value = JSON.parse(stored)
        } catch (e) {
            localStorage.removeItem('user')
        }
    }

    fetchMenu()
    syncHashToState()
    window.addEventListener('hashchange', syncHashToState)
})
</script>

<template>
  <Login v-if="!currentUser" @login-success="handleLoginSuccess" />

  <div v-else class="flex h-screen w-full bg-background overflow-hidden">
    <!-- Sidebar -->
    <aside class="w-64 bg-surface border-r border-border flex flex-col hidden md:flex">
        <div class="p-6 text-xl font-bold text-text-main flex items-center gap-2">
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
        <header class="h-16 bg-surface border-b border-border flex items-center justify-between px-8 shrink-0">
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
                <button @click="handleLogout" class="px-4 py-2 text-sm text-text-muted hover:text-text-main transition border-l border-border ml-2">
                     Logout
                </button>
             </div>
        </header>

        <div class="flex-1 overflow-y-auto p-8">
            <div v-if="!currentEntity && viewMode === 'home'" class="max-w-6xl mx-auto">
                <Dashboard v-if="dashboardSchema" :schema="dashboardSchema" />
                <div v-else class="flex items-center justify-center p-20 animate-pulse">
                    <div class="text-text-muted italic">Loading dashboard...</div>
                </div>
            </div>

            <div v-else-if="!currentEntity" class="max-w-6xl mx-auto">
                <div class="card p-12 text-center">
                    <h2 class="text-2xl font-bold mb-2">Welcome to GurihERP</h2>
                    <p class="text-text-muted">Select a module from the sidebar to manage your business data.</p>
                </div>
            </div>

            <div v-else class="max-w-6xl mx-auto h-full flex flex-col">
                <div v-if="viewMode === 'list'" class="flex-1 overflow-hidden flex flex-col">
                    <DynamicPage 
                        :entity="currentEntity" 
                        @edit="(id) => onAction('edit', id)" 
                        @create="onAction('create')"
                    />
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

  <!-- Toast Notification -->
  <ToastNotification
      :toasts="toasts"
      @remove="removeToast"
  />
</template>
