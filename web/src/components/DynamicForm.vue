<script setup>
import { ref, watch, onMounted } from 'vue'

const props = defineProps(['entity', 'id'])
const emit = defineEmits(['saved', 'cancel'])

const schema = ref(null)
const formData = ref({})
const loading = ref(false)

const API_BASE = 'http://localhost:3000/api'

// Relation Options Cache
const relationOptions = ref({})

async function fetchSchema() {
    try {
        const res = await fetch(`${API_BASE}/ui/form/${props.entity}`)
        schema.value = await res.json()
        
        // Initialize fetch for relation fields
        for(const section of schema.value.layout) {
             for(const field of section.fields) {
                 if(field.widget === 'RelationPicker') {
                     // TODO: Infer target entity from field definition?
                     // Currently UI Schema doesn't have target entity. 
                     // Need to improve FormEngine or infer from naming convention (e.g. department_id -> "Department")
                     // Simple heuristic: Remove "_id" and Capitalize
                     if(field.name.endsWith("_id")) {
                         let target = field.name.replace("_id", "")
                         target = target.charAt(0).toUpperCase() + target.slice(1)
                         fetchRelations(target, field.name)
                     }
                 }
             }
        }
    } catch (e) {
        console.error("Failed to fetch form schema", e)
    }
}

async function fetchRelations(targetEntity, fieldName) {
    try {
        const res = await fetch(`${API_BASE}/${targetEntity}`)
        if(res.ok) {
            const list = await res.json()
             // Map to options
             relationOptions.value[fieldName] = list.map(item => ({
                 value: item.id,
                 label: item.name || item.title || item.id // Fallback label
             }))
        }
    } catch(e) {
        console.log("Could not fetch relation for", targetEntity)
    }
}

async function fetchData() {
    if (!props.id) {
        formData.value = {}
        return
    }
    loading.value = true
    try {
        const res = await fetch(`${API_BASE}/${props.entity}/${props.id}`)
        formData.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch data", e)
    } finally {
        loading.value = false
    }
}

async function save() {
    const isEdit = !!props.id
    const url = isEdit ? `${API_BASE}/${props.entity}/${props.id}` : `${API_BASE}/${props.entity}`
    const method = isEdit ? 'PUT' : 'POST'

    try {
        const res = await fetch(url, {
            method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(formData.value)
        })
        if (res.ok) {
            emit('saved')
        } else {
            const err = await res.json()
            alert("Error: " + JSON.stringify(err))
        }
    } catch (e) {
        alert("Failed to save")
    }
}

watch(() => props.entity, () => {
    fetchSchema()
    formData.value = {}
})

watch(() => props.id, () => {
    fetchData()
})

onMounted(() => {
    fetchSchema()
    fetchData()
})
</script>

<template>
    <div class="bg-white rounded-lg shadow p-6 max-w-2xl mx-auto">
        <div v-if="!schema || loading" class="text-center text-gray-500">Loading Form...</div>
        
        <form v-else @submit.prevent="save" class="space-y-6">
            <div v-for="section in schema.layout" :key="section.title">
                <h3 class="text-lg font-medium text-gray-900 border-b pb-2 mb-4">{{ section.title }}</h3>
                
                <div class="grid grid-cols-1 gap-6">
                    <div v-for="field in section.fields" :key="field.name">
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            {{ field.label }} 
                            <span v-if="field.required" class="text-red-500">*</span>
                        </label>
                        
                        <div v-if="field.widget === 'TextInput'">
                            <input v-model="formData[field.name]" type="text" class="w-full border-gray-300 rounded-md shadow-sm focus:ring-accent focus:border-accent p-2 border" :required="field.required">
                        </div>
                        
                        <div v-if="field.widget === 'NumberInput'">
                            <input v-model.number="formData[field.name]" type="number" class="w-full border-gray-300 rounded-md shadow-sm focus:ring-accent focus:border-accent p-2 border" :required="field.required">
                        </div>

                        <div v-if="field.widget === 'Checkbox'">
                            <input v-model="formData[field.name]" type="checkbox" class="h-4 w-4 text-accent focus:ring-accent border-gray-300 rounded">
                        </div>
                        
                        <div v-if="field.widget === 'RelationPicker' || field.widget === 'Select'">
                            <select v-model="formData[field.name]" class="w-full border-gray-300 rounded-md shadow-sm focus:ring-accent focus:border-accent p-2 border">
                                <option :value="null">Select...</option>
                                <option v-for="opt in relationOptions[field.name] || []" :key="opt.value" :value="opt.value">
                                    {{ opt.label }}
                                </option>
                            </select>
                        </div>
                    </div>
                </div>
            </div>

            <div class="flex justify-end space-x-3 pt-4 border-t">
                <button type="button" @click="$emit('cancel')" class="px-4 py-2 border rounded-md text-gray-700 hover:bg-gray-50">Cancel</button>
                <button type="submit" class="px-4 py-2 bg-accent text-white rounded-md hover:bg-blue-600 font-medium">Save</button>
            </div>
        </form>
    </div>
</template>
