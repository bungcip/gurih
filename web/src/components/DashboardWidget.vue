<script setup>
import { computed } from 'vue'
import { Bar, Pie, Line, Doughnut } from 'vue-chartjs'
import { Chart as ChartJS, Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, ArcElement, PointElement, LineElement } from 'chart.js'

ChartJS.register(Title, Tooltip, Legend, BarElement, CategoryScale, LinearScale, ArcElement, PointElement, LineElement)

const props = defineProps(['widget'])

const chartData = computed(() => {
    if (!props.widget.value || !Array.isArray(props.widget.value)) return { labels: [], datasets: [] }

    // Transform [{label: "A", value: 10}, ...] to ChartJS format
    const labels = props.widget.value.map(i => i.label)
    const data = props.widget.value.map(i => i.value)

    // Generate colors
    const colors = [
        '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#6366f1'
    ]

    return {
        labels,
        datasets: [{
            label: props.widget.label,
            data,
            backgroundColor: props.widget.type === 'line' ? '#3b82f6' : colors,
            borderColor: props.widget.type === 'line' ? '#3b82f6' : undefined,
            fill: false
        }]
    }
})

const chartOptions = {
    responsive: true,
    maintainAspectRatio: false
}
</script>

<template>
    <div class="card p-6 h-full flex flex-col">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-sm font-bold uppercase tracking-wider text-text-muted">{{ widget.label }}</h3>
            <!-- Simple Icon Placeholder if specific icon lib not integrated -->
            <div v-if="widget.icon" class="text-text-muted opacity-50">
                 <span>ICON: {{ widget.icon }}</span>
            </div>
        </div>

        <div class="flex-1 min-h-[150px] flex items-center justify-center">
            <!-- Stat Widget -->
            <div v-if="widget.type === 'stat'" class="text-4xl font-bold text-text-main">
                {{ widget.value }}
            </div>

            <!-- Chart Widgets -->
            <div v-else-if="widget.type === 'bar'" class="w-full h-full relative">
                <Bar :data="chartData" :options="chartOptions" />
            </div>
            <div v-else-if="widget.type === 'pie'" class="w-full h-full relative">
                <Pie :data="chartData" :options="chartOptions" />
            </div>
             <div v-else-if="widget.type === 'doughnut'" class="w-full h-full relative">
                <Doughnut :data="chartData" :options="chartOptions" />
            </div>
             <div v-else-if="widget.type === 'line'" class="w-full h-full relative">
                <Line :data="chartData" :options="chartOptions" />
            </div>

            <div v-else>
                Unsupported widget type: {{ widget.type }}
            </div>
        </div>
    </div>
</template>
