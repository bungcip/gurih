<script setup>
import { ref, watch, onMounted, inject } from 'vue'
import { request } from '../api.js'
import Button from './Button.vue'
import SelectInput from './SelectInput.vue'
import DatePicker from './DatePicker.vue'
import Switch from './Switch.vue'
import Tabs from './Tabs.vue'
import FileUpload from './FileUpload.vue'
import CurrencyInput from './CurrencyInput.vue'
import Alert from './Alert.vue'
import InputGrid from './InputGrid.vue'

const props = defineProps(['entity', 'id'])
const emit = defineEmits(['saved', 'cancel'])
const showToast = inject('showToast')
const currentUser = inject('currentUser')

const schema = ref(null)
const formData = ref({})
const loading = ref(false)
const saving = ref(false)

// Relation Options Cache
const relationOptions = ref({})
const errors = ref([])

const activeTab = ref(0)


async function fetchSchema() {
    try {
        const res = await request(`/ui/form/${props.entity}`)
        schema.value = await res.json()
        activeTab.value = 0

        // Initialize fetch for relation fields
        const targetsMap = new Map() // target -> [ {fieldName, fieldRef} ]

        const processField = (field, isGridColumn = false) => {
            if (field.widget === 'RelationPicker' || (isGridColumn && field.widget === 'Select' && !field.options)) {
                // Heuristic for relation name: assume ends with _id or is relation field
                let target = null
                if (field.target_entity) {
                    target = field.target_entity
                } else if (field.name.endsWith("_id")) {
                    target = field.name.replace("_id", "")
                    target = target.charAt(0).toUpperCase() + target.slice(1)
                }

                if (target) {
                    if (!targetsMap.has(target)) {
                        targetsMap.set(target, [])
                    }
                    targetsMap.get(target).push({ fieldName: field.name, fieldRef: field })
                }
            }
        }

        for (const section of schema.value.layout) {
            for (const field of section.fields) {
                processField(field)
                if (field.widget === 'InputGrid' && field.columns) {
                    for (const col of field.columns) {
                        processField(col, true)
                    }
                }
            }
        }

        await Promise.all(Array.from(targetsMap.entries()).map(([target, items]) =>
            fetchRelations(target, items)
        ))
    } catch (e) {
        console.error("Failed to fetch form schema", e)
    }
}

async function fetchRelations(targetEntity, items, query = "") {
    try {
        let url = `/${targetEntity}?limit=50`
        if (query) {
            url += `&_search=${encodeURIComponent(query)}`
        }

        const res = await request(url)
        if (res.ok) {
            const list = await res.json()
            const options = list.map(item => ({
                value: item.id,
                label: item.name || item.title || item.id
            }))

            for (const item of items) {
                // If it's a top-level field, store in relationOptions
                relationOptions.value[item.fieldName] = options
                // If it's a grid column (or any field really), inject options directly
                if (item.fieldRef) {
                    item.fieldRef.options = options
                }
            }
        }
    } catch (e) {
        errors.value.push(`Could not fetch relation for ${targetEntity}`)
    }
}

function onSearchRelation(field, query) {
    let target = field.target_entity
    if (!target && field.name.endsWith("_id")) {
        target = field.name.replace("_id", "")
        target = target.charAt(0).toUpperCase() + target.slice(1)
    }
    if (target) {
        fetchRelations(target, [{ fieldName: field.name, fieldRef: field }], query)
    }
}

async function fetchData() {
    if (!props.id) {
        formData.value = {}
        return
    }
    loading.value = true
    try {
        const res = await request(`/${props.entity}/${props.id}`)
        formData.value = await res.json()
    } catch (e) {
        console.error("Failed to fetch data", e)
    } finally {
        loading.value = false
    }
}

async function uploadFiles() {
    const updatedData = { ...formData.value }
    const uploadTasks = []

    for (const section of schema.value.layout) {
        for (const field of section.fields) {
            if (field.widget === 'FileUpload') {
                const val = formData.value[field.name]
                if (!val) continue

                const processFile = async (file) => {
                    if (!(file instanceof File)) return file

                    const formDataObj = new FormData()
                    formDataObj.append('file', file)

                    const res = await request(`/upload/${props.entity}/${field.name}`, {
                        method: 'POST',
                        body: formDataObj
                    })

                    if (res.ok) {
                        const data = await res.json()
                        return data.url
                    } else {
                        throw new Error(`Failed to upload ${field.name}`)
                    }
                }

                if (Array.isArray(val)) {
                    const task = Promise.all(val.map(processFile)).then(urls => {
                        updatedData[field.name] = urls
                    })
                    uploadTasks.push(task)
                } else if (val instanceof File) {
                    const task = processFile(val).then(url => {
                        updatedData[field.name] = url
                    })
                    uploadTasks.push(task)
                }
            }
        }
    }

    if (uploadTasks.length > 0) {
        await Promise.all(uploadTasks)
        formData.value = updatedData
    }
}

async function save() {
    const isEdit = !!props.id
    const url = isEdit ? `/${props.entity}/${props.id}` : `/${props.entity}`
    const method = isEdit ? 'PUT' : 'POST'

    saving.value = true
    try {
        // Upload any pending files first
        await uploadFiles()

        const res = await request(url, {
            method,
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
        showToast("Error: " + e.message, 'error')
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

                    <div v-if="errors.length" class="mb-6 space-y-2">
                        <Alert v-for="(error, i) in errors" :key="i" variant="danger" :title="error" closable
                            @close="errors.splice(i, 1)" />
                    </div>

                    <!-- Tabs -->
                    <Tabs v-model="activeTab" :items="schema.layout" />
                </div>

                <!-- Content Area -->
                <div class="flex-1 overflow-y-auto min-h-0 bg-[--color-surface] p-6 relative z-0">
                    <div v-for="(section, index) in schema.layout" :key="section.title">
                        <div v-if="activeTab === index"
                            class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">

                            <!-- Section Content -->
                            <div>

                                <div class="grid grid-cols-1 md:grid-cols-2 gap-x-12 gap-y-8">
                                    <div v-for="field in section.fields" :key="field.name">
                                        <label :for="field.name"
                                            class="block text-[13px] font-medium text-text-muted mb-2">
                                            {{ field.label }}
                                            <span v-if="field.required" class="text-red-500">*</span>
                                        </label>

                                        <div v-if="field.widget === 'TextInput'">
                                            <input :id="field.name" v-model="formData[field.name]" type="text"
                                                class="input-field" :required="field.required">
                                        </div>

                                        <div v-if="field.widget === 'NumberInput'">
                                            <input :id="field.name" v-model.number="formData[field.name]" type="number"
                                                class="input-field" :required="field.required">
                                        </div>

                                        <div v-if="field.widget === 'TextArea'">
                                            <textarea :id="field.name" v-model="formData[field.name]"
                                                class="input-field min-h-[120px] resize-y"
                                                :required="field.required"></textarea>
                                        </div>

                                        <div v-if="field.widget === 'DatePicker'">
                                            <DatePicker :id="field.name" v-model="formData[field.name]"
                                                :required="field.required" />
                                        </div>

                                        <div v-if="field.widget === 'DateTimePicker'">
                                            <input :id="field.name" v-model="formData[field.name]" type="datetime-local"
                                                class="input-field" :required="field.required">
                                        </div>

                                        <div v-if="field.widget === 'CurrencyInput'">
                                            <CurrencyInput :id="field.name" v-model="formData[field.name]" :label="null"
                                                :prefix="field.prefix || 'Rp'" :decimals="field.decimals ?? 0"
                                                :required="field.required" />
                                        </div>

                                        <div v-if="field.widget === 'FileUpload'">
                                            <FileUpload :id="field.name" v-model="formData[field.name]" :label="null"
                                                :required="field.required" :accept="field.accept"
                                                :multiple="field.multiple" />
                                        </div>

                                        <div v-if="field.widget === 'Checkbox'" class="flex items-center h-10">
                                            <Switch :id="field.name" v-model="formData[field.name]" label="Enabled" />
                                        </div>

                                        <div v-if="field.widget === 'RelationPicker' || field.widget === 'Select'">
                                            <SelectInput :id="field.name" v-model="formData[field.name]"
                                                :options="field.options || relationOptions[field.name] || []"
                                                :placeholder="'Select ' + field.label + '...'"
                                                :required="field.required"
                                                @search="(q) => onSearchRelation(field, q)" />
                                        </div>

                                        <div v-if="field.widget === 'InputGrid'" class="col-span-1 md:col-span-2">
                                            <InputGrid v-model="formData[field.name]" :columns="field.columns"
                                                :label="field.label" :target-entity="field.target_entity"
                                                :required="field.required" />
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
