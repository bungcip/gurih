<script setup>
import { ref, onMounted, watch, computed } from 'vue'
import Icon from './Icon.vue'
import StatusBadge from './StatusBadge.vue'

const props = defineProps({
  schema: {
    type: Object,
    required: true
  }
})

const dashboardData = ref(null)
const loading = ref(true)

// Mock fetching actual data based on widgets
async function fetchDashboardData() {
  if (!props.schema || !props.schema.widgets) {
    loading.value = false
    return
  }
  loading.value = true
  // In a real implementation, this would fetch from /api/dashboard/:name
  // which executes the queries defined in KDL.
  setTimeout(() => {
    dashboardData.value = {
      widgets: (props.schema.widgets || []).map(w => ({
        ...w,
        resolvedValue: Math.floor(Math.random() * 100) // Mock value
      })),
      recentActivity: [
        { id: 1, user: 'John Doe', action: 'Requested Leave', time: '2 hours ago', status: 'Pending' },
        { id: 2, user: 'Jane Smith', action: 'Updated Profile', time: '4 hours ago', status: 'Success' },
        { id: 3, user: 'System', action: 'Monthly Payroll Generated', time: '1 day ago', status: 'Info' }
      ]
    }
    loading.value = false
  }, 800)
}

onMounted(fetchDashboardData)

watch(() => props.schema, (newSchema) => {
  if (newSchema) {
    fetchDashboardData()
  }
}, { immediate: true })

function getStatColor(color) {
  switch (color) {
    case 'warning': return 'text-yellow-600 bg-yellow-100'
    case 'danger': return 'text-red-600 bg-red-100'
    case 'success': return 'text-green-600 bg-green-100'
    case 'info': return 'text-blue-600 bg-blue-100'
    default: return 'text-primary bg-blue-50'
  }
}

const statWidgets = computed(() => {
  if (!dashboardData.value || !dashboardData.value.widgets) return []
  return dashboardData.value.widgets.filter(w => w.type === 'stat')
})
</script>

<template>
  <div v-if="schema" class="space-y-8 animate-in fade-in duration-500">
    <!-- Header -->
    <div class="flex items-center justify-between">
        <div>
            <h2 class="text-2xl font-bold text-text-main">{{ schema?.title || 'Dashboard' }}</h2>
            <p class="text-text-muted mt-1">Welcome back! Here's what's happening today.</p>
        </div>
        <div class="flex gap-2">
            <button @click="fetchDashboardData" class="p-2 text-text-muted hover:text-primary transition-colors rounded-lg hover:bg-white dark:hover:bg-gray-800" title="Refresh">
                <Icon name="lucide:clock" :size="20" :class="{ 'animate-spin': loading }" />
            </button>
        </div>
    </div>

    <!-- Stats Grid -->
    <div v-if="dashboardData" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <div 
            v-for="widget in statWidgets"
            :key="widget.label"
            class="card p-6 flex items-start justify-between hover:shadow-lg transition-all border-none shadow-sm bg-[--color-surface]"
        >
            <div class="space-y-1">
                <p class="text-sm font-medium text-text-muted uppercase tracking-wider">{{ widget.label }}</p>
                <div class="flex items-baseline gap-2">
                    <h3 class="text-3xl font-bold text-text-main tabular-nums">{{ widget.resolvedValue }}</h3>
                    <span class="text-xs font-medium text-green-600 dark:text-green-400 flex items-center bg-green-50 dark:bg-green-900/20 px-1.5 py-0.5 rounded">+12%</span>
                </div>
            </div>
            <div :class="['p-3 rounded-2xl', getStatColor(widget.color)]">
                <Icon :name="widget.icon || 'lucide:layout-dashboard'" :size="24" />
            </div>
        </div>
    </div>

    <!-- Main Content Area -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
        <!-- Recent Activity -->
        <div class="lg:col-span-2 card p-6 bg-[--color-surface] border-none shadow-sm flex flex-col">
            <h3 class="text-lg font-bold text-text-main mb-6">Recent Activity</h3>
            <div class="space-y-6">
                <div v-for="item in dashboardData?.recentActivity" :key="item.id" class="flex gap-4 group cursor-default">
                    <div class="relative">
                        <div class="w-10 h-10 rounded-full bg-gray-100 dark:bg-gray-800 flex items-center justify-center text-gray-500 dark:text-gray-400 font-bold text-sm shrink-0 uppercase">
                            {{ item.user.charAt(0) }}
                        </div>
                        <div v-if="item.id !== dashboardData.recentActivity.length" class="absolute top-10 left-1/2 -translate-x-1/2 w-0.5 h-6 bg-gray-100 dark:bg-gray-800"></div>
                    </div>
                    <div class="flex-1 pb-2">
                        <div class="flex items-center justify-between mb-0.5">
                            <span class="font-bold text-text-main text-sm group-hover:text-primary transition-colors">{{ item.user }}</span>
                            <span class="text-xs text-text-muted tabular-nums">{{ item.time }}</span>
                        </div>
                        <p class="text-sm text-text-muted">{{ item.action }}</p>
                        <div class="mt-2">
                            <StatusBadge :label="item.status" :variant="item.status.toLowerCase()" />
                        </div>
                    </div>
                </div>
            </div>
            <button class="mt-8 text-sm font-bold text-primary hover:underline self-start">View all activity</button>
        </div>

        <!-- Distribution Chart Placeholder -->
        <div class="card p-6 bg-[--color-surface] border-none shadow-sm">
             <h3 class="text-lg font-bold text-text-main mb-6">Employee Distribution</h3>
             <div class="aspect-square flex flex-col items-center justify-center relative">
                 <!-- Simple Circle CSS Chart -->
                 <div class="w-48 h-48 rounded-full border-[16px] border-primary border-t-yellow-400 border-r-red-400 border-b-green-400 flex items-center justify-center">
                    <div class="text-center">
                        <span class="block text-2xl font-bold text-text-main">84%</span>
                        <span class="text-[10px] text-text-muted uppercase font-bold tracking-widest">Efficiency</span>
                    </div>
                 </div>
                 
                 <div class="mt-8 w-full space-y-3">
                     <div class="flex items-center justify-between text-xs">
                         <div class="flex items-center gap-2 text-text-muted">
                             <div class="w-3 h-3 rounded-full bg-primary"></div> Permanent
                         </div>
                         <span class="font-bold text-text-main">120</span>
                     </div>
                     <div class="flex items-center justify-between text-xs">
                         <div class="flex items-center gap-2 text-text-muted">
                             <div class="w-3 h-3 rounded-full bg-yellow-400"></div> Probation
                         </div>
                         <span class="font-bold text-text-main">34</span>
                     </div>
                     <div class="flex items-center justify-between text-xs">
                         <div class="flex items-center gap-2 text-text-muted">
                             <div class="w-3 h-3 rounded-full bg-red-400"></div> Resigned
                         </div>
                         <span class="font-bold text-text-main">12</span>
                     </div>
                 </div>
             </div>
        </div>
    </div>
  </div>
</template>

<style scoped>
.animate-in {
  animation-duration: 0.5s;
  animation-fill-mode: both;
}
.fade-in {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
