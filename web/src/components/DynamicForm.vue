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

const activeTab = ref(0)

async function fetchSchema() {
    try {
        const res = await fetch(`${API_BASE}/ui/form/${props.entity}`)
        schema.value = await res.json()
        activeTab.value = 0
        
        // Initialize fetch for relation fields
        const targetsMap = new Map() // target -> [fieldNames]

        for(const section of schema.value.layout) {
             for(const field of section.fields) {
                 if(field.widget === 'RelationPicker') {
                     if(field.name.endsWith("_id")) {
                         let target = field.name.replace("_id", "")
                         target = target.charAt(0).toUpperCase() + target.slice(1)

                         if (!targetsMap.has(target)) {
                             targetsMap.set(target, [])
                         }
                         targetsMap.get(target).push(field.name)
                     }
                 }
             }
        }

        await Promise.all(Array.from(targetsMap.entries()).map(([target, fields]) =>
            fetchRelations(target, fields)
        ))
    } catch (e) {
        console.error("Failed to fetch form schema", e)
    }
}

async function fetchRelations(targetEntity, fieldNames) {
    try {
        const res = await fetch(`${API_BASE}/${targetEntity}`)
        if(res.ok) {
            const list = await res.json()
            const options = list.map(item => ({
                 value: item.id,
                 label: item.name || item.title || item.id 
             }))

             for(const fieldName of fieldNames) {
                 relationOptions.value[fieldName] = options
             }
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
    <div class="h-full flex flex-col">
        <div v-if="!schema || loading" class="card p-12 text-center text-text-muted">Loading Form...</div>
        
        <form v-else @submit.prevent="save" class="flex-1 flex flex-col gap-6 overflow-hidden">
            <!-- Header Card -->
            <div class="card p-6 pb-0 shrink-0">
                <div class="flex items-center gap-4 mb-6">
                    <div class="w-12 h-12 bg-blue-50 text-primary rounded-xl flex items-center justify-center font-bold text-lg">
                        {{ entity.charAt(0) }}
                    </div>
                    <div>
                        <h2 class="text-xl font-bold text-text-main">{{ schema.name }}</h2>
                        <p class="text-sm text-text-muted">Fill in the details below to save {{ entity }}</p>
                    </div>
                </div>

                <!-- Tabs -->
                <div class="flex gap-8 border-b border-border">
                    <button 
                        v-for="(section, index) in schema.layout" 
                        :key="section.title"
                        type="button"
                        @click="activeTab = index"
                        class="pb-3 text-sm font-medium transition-all relative"
                        :class="activeTab === index ? 'text-primary' : 'text-text-muted hover:text-text-main'"
                    >
                        {{ section.title }}
                        <div v-if="activeTab === index" class="absolute bottom-0 left-0 right-0 h-0.5 bg-primary rounded-full"></div>
                    </button>
                </div>
            </div>
            
            <!-- Content Area -->
            <div class="flex-1 overflow-y-auto min-h-0">
                <div v-for="(section, index) in schema.layout" :key="section.title">
                    <div v-if="activeTab === index" class="card p-8 space-y-8 animate-in fade-in slide-in-from-bottom-2 duration-300">
                        <div class="flex items-center gap-2">
                             <div class="w-1 h-4 bg-primary rounded-full"></div>
                             <h3 class="font-bold text-text-main">{{ section.title }}</h3>
                        </div>
                        
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-6">
                            <div v-for="field in section.fields" :key="field.name">
                                <label class="block text-[13px] font-semibold text-text-muted mb-2">
                                    {{ field.label }} 
                                    <span v-if="field.required" class="text-red-500">*</span>
                                </label>
                                
                                <div v-if="field.widget === 'TextInput'">
                                    <input v-model="formData[field.name]" type="text" class="input-field" :placeholder="'Enter ' + field.label" :required="field.required">
                                </div>
                                
                                <div v-if="field.widget === 'NumberInput'">
                                    <input v-model.number="formData[field.name]" type="number" class="input-field" :required="field.required">
                                </div>

                                <div v-if="field.widget === 'TextArea'">
                                    <textarea v-model="formData[field.name]" class="input-field min-h-[100px]" :placeholder="'Enter ' + field.label" :required="field.required"></textarea>
                                </div>

                                <div v-if="field.widget === 'DatePicker'">
                                    <input v-model="formData[field.name]" type="date" class="input-field" :required="field.required">
                                </div>

                                <div v-if="field.widget === 'DateTimePicker'">
                                    <input v-model="formData[field.name]" type="datetime-local" class="input-field" :required="field.required">
                                </div>

                                <div v-if="field.widget === 'Checkbox'" class="flex items-center h-10">
                                    <input v-model="formData[field.name]" type="checkbox" class="h-5 w-5 text-primary border-border rounded focus:ring-primary/20">
                                </div>
                                
                                <div v-if="field.widget === 'RelationPicker' || field.widget === 'Select'">
                                    <select v-model="formData[field.name]" class="input-field bg-white">
                                        <option :value="null">Select {{ field.label }}...</option>
                                        <option v-for="opt in (field.options || relationOptions[field.name] || [])" :key="opt.value" :value="opt.value">
                                            {{ opt.label }}
                                        </option>
                                    </select>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- Sticky Footer -->
            <div class="card p-4 flex justify-end gap-3 shrink-0">
                <button type="button" @click="$emit('cancel')" class="px-6 py-2 border border-border rounded-lg text-sm font-medium hover:bg-gray-50 transition">
                    Cancel
                </button>
                <button type="submit" class="btn-primary px-8 py-2">
                    {{ id ? 'Update' : 'Create' }} {{ entity }}
                </button>
            </div>
        </form>
    </div>
</template>
