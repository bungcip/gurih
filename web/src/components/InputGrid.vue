<script setup>
import { computed } from 'vue'
import Button from './Button.vue'
import SelectInput from './SelectInput.vue'
import CurrencyInput from './CurrencyInput.vue'

const props = defineProps({
  modelValue: { type: Array, default: () => [] },
  columns: { type: Array, default: () => [] },
  label: { type: String, default: '' },
  targetEntity: { type: String, default: '' },
  required: { type: Boolean, default: false }
})

const emit = defineEmits(['update:modelValue'])

const rows = computed({
  get: () => props.modelValue || [],
  set: (val) => emit('update:modelValue', val)
})

const addRow = () => {
  const newRow = {}
  props.columns.forEach(col => {
      if (col.widget === 'NumberInput' || col.widget === 'CurrencyInput') {
          newRow[col.name] = 0
      } else {
          newRow[col.name] = null
      }
  })

  rows.value = [...rows.value, newRow]
}

const removeRow = (index) => {
  const newRows = [...rows.value]
  newRows.splice(index, 1)
  rows.value = newRows
}
</script>

<template>
  <div class="space-y-3">
    <div class="flex justify-between items-center">
        <label class="block text-[13px] font-medium text-text-muted">{{ label }}</label>
        <Button size="sm" variant="ghost-primary" icon="lucide:plus" @click="addRow">Add Line</Button>
    </div>

    <div class="overflow-x-auto border border-border rounded-lg bg-[--color-surface]">
      <table class="w-full text-left text-sm">
        <thead class="bg-gray-50/50 dark:bg-gray-800/50 border-b border-border">
          <tr>
            <th v-for="col in columns" :key="col.name" class="p-3 font-medium text-text-muted whitespace-nowrap">
              {{ col.label }}
              <span v-if="col.required" class="text-red-500">*</span>
            </th>
            <th class="p-3 w-10"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border">
          <tr v-for="(row, index) in rows" :key="index" class="group hover:bg-gray-50/50 dark:hover:bg-gray-800/50">
            <td v-for="col in columns" :key="col.name" class="p-2 align-top min-w-[150px]">

                <div v-if="col.widget === 'TextInput'">
                    <input v-model="row[col.name]" type="text" class="input-field h-9 text-sm" :required="col.required">
                </div>

                <div v-else-if="col.widget === 'NumberInput'">
                    <input v-model.number="row[col.name]" type="number" class="input-field h-9 text-sm" :required="col.required">
                </div>

                <div v-else-if="col.widget === 'CurrencyInput'">
                     <CurrencyInput v-model="row[col.name]" :label="null" :prefix="col.prefix || 'Rp'" :required="col.required" class="h-9 text-sm" />
                </div>

                <div v-else-if="col.widget === 'Select' || col.widget === 'RelationPicker'">
                    <SelectInput
                        v-model="row[col.name]"
                        :options="col.options || []"
                        class="h-9 text-sm"
                        :required="col.required"
                    />
                </div>

                <div v-else>
                    <input v-model="row[col.name]" type="text" class="input-field h-9 text-sm">
                </div>
            </td>
            <td class="p-2 align-top text-center pt-3">
               <button type="button" @click="removeRow(index)" class="text-gray-400 hover:text-red-500 transition-colors">
                  <span class="sr-only">Remove</span>
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
               </button>
            </td>
          </tr>
          <tr v-if="rows.length === 0">
              <td :colspan="columns.length + 1" class="p-8 text-center text-text-muted text-sm">
                  No items added. <span class="text-primary cursor-pointer hover:underline" @click="addRow">Add one</span>.
              </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
