<script setup>
import Button from './Button.vue'
import StatusBadge from './StatusBadge.vue'

const props = defineProps({
    columns: { type: Array, required: true },
    data: { type: Array, default: () => [] },
    actions: { type: Array, default: () => [] },
    loading: { type: Boolean, default: false }
})

const emit = defineEmits(['action'])
</script>

<template>
  <div class="flex-1 overflow-auto bg-white relative">
      <div v-if="loading" class="absolute inset-0 flex items-center justify-center bg-white/50 z-10">
          <div class="animate-pulse flex flex-col items-center">
                <div class="h-8 w-32 bg-gray-100 rounded mb-4"></div>
                <div class="text-sm text-text-muted">Loading records...</div>
          </div>
      </div>

      <table class="w-full text-left border-collapse">
          <thead class="bg-gray-50/50 sticky top-0 backdrop-blur-sm border-b border-border shadow-sm">
              <tr>
                  <th v-for="col in columns" :key="col.key" class="p-4 px-8 font-bold text-[11px] uppercase tracking-wider text-text-muted">
                      {{ col.label }}
                  </th>
                  <th v-if="actions.length" class="p-4 px-8 font-bold text-[11px] uppercase tracking-wider text-text-muted text-right">Actions</th>
              </tr>
          </thead>
          <tbody class="divide-y divide-border">
              <tr v-for="row in data" :key="row.id" class="group hover:bg-blue-50/30 transition-colors">
                  <td v-for="col in columns" :key="col.key" class="p-4 px-8 text-[14px] text-text-main">
                      <template v-if="col.type === 'status'">
                          <StatusBadge 
                            :label="row[col.key]" 
                            :variant="col.variant_map ? col.variant_map[row[col.key]] : 'gray'" 
                          />
                      </template>
                      <template v-else-if="col.type === 'currency'">
                          {{ new Intl.NumberFormat('id-ID', { style: 'currency', currency: col.currencyCode || 'IDR', minimumFractionDigits: 0 }).format(row[col.key] || 0) }}
                      </template>
                      <template v-else>
                          {{ row[col.key] }}
                      </template>
                  </td>
                  <td v-if="actions.length" class="p-4 px-8 text-right">
                      <div class="flex justify-end gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                          <Button
                              v-for="action in actions"
                              :key="action.label"
                              size="sm"
                              :variant="action.variant === 'danger' ? 'ghost-danger' : 'ghost-primary'"
                              @click="$emit('action', action, row)"
                          >
                              {{ action.label }}
                          </Button>
                      </div>
                  </td>
              </tr>
              <tr v-if="data.length === 0 && !loading">
                  <td :colspan="columns.length + (actions.length ? 1 : 0)" class="p-20 text-center">
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
</template>
