<script setup>
import { ref, watch, onMounted, inject } from 'vue'
import Button from './Button.vue'
import SelectInput from './SelectInput.vue'
import DatePicker from './DatePicker.vue'
import Switch from './Switch.vue'
import Tabs from './Tabs.vue'
import FileUpload from './FileUpload.vue'
import CurrencyInput from './CurrencyInput.vue'

const props = defineProps(['entity', 'id'])
const emit = defineEmits(['saved', 'cancel'])
const showToast = inject('showToast')
const currentUser = inject('currentUser')

const schema = ref(null)
const formData = ref({})
const loading = ref(false)
const saving = ref(false)

const API_BASE = 'http://localhost:3000/api'

// Relation Options Cache
const relationOptions = ref({})

const activeTab = ref(0)

function getAuthHeaders() {
    return currentUser.value && currentUser.value.token ? {
        'Authorization': `Bearer ${currentUser.value.token}`
    } : {}
}

async function fetchSchema() {
    try {
        const res = await fetch(`${API_BASE}/ui/form/${props.entity}`, {
            headers: getAuthHeaders()
        })
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
        const res = await fetch(`${API_BASE}/${targetEntity}`, {
            headers: getAuthHeaders()
        })
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
        const res = await fetch(`${API_BASE}/${props.entity}/${props.id}`, {
            headers: getAuthHeaders()
        })
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

    saving.value = true
    try {
        const headers = {
            'Content-Type': 'application/json',
            ...getAuthHeaders()
        }
        const res = await fetch(url, {
            method,
            headers,
            body: JSON.stringify(formData.value)
        })
        if (res.ok) {
            showToast('Saved successfully!', 'success')
            emit('saved')
        } else {
            const err = await res.json()
            showToast("Error: " + JSON.stringify(err), 'error')
        }
    } catch (e) {
        showToast("Failed to save", 'error')
    } finally {
        saving.value = false
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
            <!-- Header Card with Integrated Tabs -->
            <div class="card p-0 flex-1 flex flex-col overflow-hidden">
                <div class="p-6 pb-0 border-b border-gray-100 dark:border-gray-700">
                    <div class="flex items-center gap-4 mb-6">
                         <h2 class="text-xl font-bold text-text-main">{{ schema.name }} Form</h2>
                    </div>

                    <!-- Tabs -->
                    <Tabs 
                        v-model="activeTab" 
                        :items="schema.layout" 
                    />
                </div>
            
                <!-- Content Area -->
                <div class="flex-1 overflow-y-auto min-h-0 bg-[--color-surface] p-6 relative z-0">
                    <div v-for="(section, index) in schema.layout" :key="section.title">
                        <div v-if="activeTab === index" class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
                        
                        <!-- Section Content -->
                        <div>
                        
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-8">
                            <div v-for="field in section.fields" :key="field.name">
                                <label :for="field.name" class="block text-[13px] font-medium text-text-muted mb-2">
                                    {{ field.label }} 
                                    <span v-if="field.required" class="text-red-500">*</span>
                                </label>
                                
                                <div v-if="field.widget === 'TextInput'">
                                    <input
                                        :id="field.name"
                                        v-model="formData[field.name]"
                                        type="text"
                                        class="input-field"
                                        :required="field.required"
                                    >
                                </div>
                                
                                <div v-if="field.widget === 'NumberInput'">
                                    <input
                                        :id="field.name"
                                        v-model.number="formData[field.name]"
                                        type="number"
                                        class="input-field"
                                        :required="field.required"
                                    >
                                </div>

                                <div v-if="field.widget === 'TextArea'">
                                    <textarea
                                        :id="field.name"
                                        v-model="formData[field.name]"
                                        class="input-field min-h-[120px] resize-y"
                                        :required="field.required"
                                    ></textarea>
                                </div>

                                <div v-if="field.widget === 'DatePicker'">
                                    <DatePicker
                                        :id="field.name"
                                        v-model="formData[field.name]"
                                        :required="field.required"
                                    />
                                </div>

                                <div v-if="field.widget === 'DateTimePicker'">
                                    <input
                                        :id="field.name"
                                        v-model="formData[field.name]"
                                        type="datetime-local"
                                        class="input-field"
                                        :required="field.required"
                                    >
                                </div>

                                <div v-if="field.widget === 'CurrencyInput'">
                                    <CurrencyInput 
                                        :id="field.name"
                                        v-model="formData[field.name]" 
                                        :label="null"
                                        :prefix="field.prefix || 'Rp'"
                                        :decimals="field.decimals ?? 0"
                                        :required="field.required"
                                    />
                                </div>

                                <div v-if="field.widget === 'FileUpload'">
                                    <FileUpload 
                                        :id="field.name"
                                        v-model="formData[field.name]" 
                                        :label="null"
                                        :required="field.required"
                                        :accept="field.accept"
                                        :multiple="field.multiple"
                                    />
                                </div>

                                <div v-if="field.widget === 'Checkbox'" class="flex items-center h-10">
                                    <Switch 
                                        :id="field.name"
                                        v-model="formData[field.name]" 
                                        label="Enabled"
                                    />
                                </div>
                                
                                <div v-if="field.widget === 'RelationPicker' || field.widget === 'Select'">
                                    <SelectInput 
                                        :id="field.name"
                                        v-model="formData[field.name]" 
                                        :options="field.options || relationOptions[field.name] || []"
                                        :placeholder="'Select ' + field.label + '...'"
                                        :required="field.required"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            </div>
        </div>


            <!-- Sticky Footer -->
            <div class="p-4 px-6 border-t border-border bg-[--color-surface] flex justify-end gap-3 shrink-0">
                <Button type="button" variant="outline" @click="$emit('cancel')">
                    Cancel
                </Button>
                <Button type="submit" variant="primary" :loading="saving">
                    {{ id ? 'Save Changes' : 'Submit' }}
                </Button>
            </div>
        </form>
    </div>
</template>
