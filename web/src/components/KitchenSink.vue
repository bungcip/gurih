<script setup>
import { ref } from 'vue'
import Button from './Button.vue'
import SelectInput from './SelectInput.vue'
import DatePicker from './DatePicker.vue'
import RadioButton from './RadioButton.vue'
import Switch from './Switch.vue'
import Tabs from './Tabs.vue'
import Password from './Password.vue'
import FileUpload from './FileUpload.vue'
import CurrencyInput from './CurrencyInput.vue'
import StatusBadge from './StatusBadge.vue'
import Modal from './Modal.vue'
import Timeline from './Timeline.vue'
import Steps from './Steps.vue'
import MetricCard from './MetricCard.vue'
import DescriptionList from './DescriptionList.vue'
import ActionCard from './ActionCard.vue'
import { inject } from 'vue'

const selectValue = ref(null)
const dateValue = ref('')
const passwordValue = ref('')
const singleFile = ref(null)
const multipleFiles = ref([])
const amountValue = ref(1500000)
const isModalOpen = ref(false)
const loading = ref(false)

const showToast = inject('showToast')

function triggerToast(type) {
    if (type === 'success') showToast('Success message!', 'success')
    if (type === 'error') showToast('Something went wrong.', 'error')
    if (type === 'info') showToast('Here is some information.')
}

const options = [
    { label: 'Option 1', value: 1 },
    { label: 'Option 2', value: 2 },
    { label: 'Option 3', value: 3 },
]

const radioValue = ref(1)
const switchValue = ref(false)
const activeTab = ref(0)
const tabItems = ['Account', 'Password', 'Notifications']

const timelineItems = [
  {
    title: 'Order Delivered',
    description: 'Package delivered to reception.',
    date: 'Oct 12, 2023 14:30',
    variant: 'success',
    icon: 'check-circle'
  },
  {
    title: 'Out for Delivery',
    description: 'Courier is on the way.',
    date: 'Oct 12, 2023 09:15',
    variant: 'info',
    icon: 'clock'
  },
  {
    title: 'Payment Issue',
    description: 'Payment failed initially, retrying.',
    date: 'Oct 11, 2023 18:00',
    variant: 'danger',
    icon: 'alert-circle'
  },
  {
    title: 'Order Placed',
    description: 'Order #12345 confirmed.',
    date: 'Oct 11, 2023 17:45',
    variant: 'gray',
    icon: 'plus'
  }
]

const stepsItems = [
  { label: 'Draft', description: 'Initial draft' },
  { label: 'Review', description: 'Manager review' },
  { label: 'Approval', description: 'Final approval' },
  { label: 'Payment', description: 'Processing payment' },
  { label: 'Completed', description: 'Done' }
]
const clickableStep = ref(2)

const employeeDetails = [
  { label: 'Full Name', value: 'Budi Santoso' },
  { label: 'Employee ID', value: 'EMP-2023-001', type: 'code' },
  { label: 'Department', value: 'Engineering' },
  { label: 'Role', value: 'Senior Frontend Engineer' },
  { label: 'Status', value: 'Active', type: 'status', variant: 'success' },
  { label: 'Join Date', value: '2022-03-15', type: 'date' },
  { label: 'Monthly Salary', value: 18500000, type: 'currency' },
  { label: 'Email', value: 'budi.santoso@company.com', type: 'link', href: 'mailto:budi.santoso@company.com' },
  { label: 'Notes', value: 'High performer. Currently leading the UI migration project.', span: 2 }
]

const systemInfo = [
  { label: 'Version', value: 'v2.4.0', type: 'code' },
  { label: 'Environment', value: 'Production', type: 'status', variant: 'warning' },
  { label: 'Last Deployed', value: '2023-10-20', type: 'date' },
  { label: 'Uptime', value: '99.99%' }
]

const actionCards = [
  {
    title: 'Invoice #INV-2023-001',
    description: 'Software License Renewal for Q4 2023',
    status: { label: 'Pending Approval', variant: 'warning' },
    meta: [
      { label: 'Amount', value: '$5,000.00', icon: 'dollar-sign' },
      { label: 'Vendor', value: 'Adobe Inc.', icon: 'briefcase' },
      { label: 'Date', value: 'Oct 24, 2023', icon: 'calendar' }
    ],
    actions: [
      { label: 'Reject', variant: 'ghost-danger', value: 'reject' },
      { label: 'Approve', variant: 'primary', value: 'approve' }
    ]
  },
  {
    title: 'Leave Request',
    description: 'Annual Leave for 2 weeks in December.',
    status: { label: 'Approved', variant: 'success' },
    meta: [
       { label: 'Employee', value: 'Sarah Jenkins', icon: 'user' },
       { label: 'Duration', value: '10 Days', icon: 'clock' }
    ],
    actions: [
       { label: 'View Details', variant: 'secondary', value: 'view' }
    ]
  },
   {
    title: 'Server Alert',
    description: 'High CPU usage detected on prod-db-01.',
    status: { label: 'Critical', variant: 'danger' },
    meta: [
       { label: 'Metric', value: '98% CPU', icon: 'activity' },
       { label: 'Region', value: 'us-east-1', icon: 'map-pin' }
    ],
    actions: [
       { label: 'Acknowledge', variant: 'outline', value: 'ack' },
       { label: 'Restart', variant: 'danger', value: 'restart' }
    ]
  }
]

function onCardAction(action) {
    console.log('Card Action:', action)
    triggerToast('info')
}

function toggleLoading() {
    loading.value = !loading.value
}
</script>

<template>
    <div class="p-8 max-w-4xl mx-auto space-y-8">
        <h1 class="text-3xl font-bold text-gray-800">Kitchen Sink</h1>
        <p class="text-gray-500">Component demonstration and test ground.</p>

        <!-- Buttons -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Buttons</h2>
            <div class="flex flex-wrap gap-4">
                <Button variant="primary">Primary</Button>
                <Button variant="secondary">Secondary</Button>
                <Button variant="outline">Outline</Button>
                <Button variant="danger">Danger</Button>
                <Button variant="ghost">Ghost</Button>
                <Button variant="primary" :loading="true">Loading</Button>
                <Button variant="primary" disabled>Disabled</Button>
            </div>
        </section>

        <!-- Inputs -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Inputs</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Standard Input</label>
                    <input type="text" class="input-field" placeholder="Type something...">
                </div>
                <div>
                     <label class="block text-sm font-medium text-gray-700 mb-1">Select Input (New)</label>
                     <SelectInput 
                        v-model="selectValue" 
                        :options="options" 
                        placeholder="Choose an option"
                     />
                     <div class="mt-1 text-xs text-gray-500">Selected Value: {{ selectValue }}</div>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Date Picker (New)</label>
                    <DatePicker v-model="dateValue" />
                    <div class="mt-1 text-xs text-gray-500">Selected Date: {{ dateValue }}</div>
                </div>
                <div>
                     <label class="block text-sm font-medium text-gray-700 mb-1">Password Input (New)</label>
                     <Password v-model="passwordValue" placeholder="Enter password" />
                     <div class="mt-2">
                        <Password v-model="passwordValue" placeholder="Disabled password" disabled />
                     </div>
                </div>
                <div>
                     <label class="block text-sm font-medium text-gray-700 mb-1">Disabled Input</label>
                     <input type="text" class="input-field" placeholder="Disabled input..." disabled>
                </div>
                <div>
                     <CurrencyInput v-model="amountValue" label="Currency Input (New)" />
                     <div class="mt-1 text-xs text-gray-500">Raw Numeric Value: {{ amountValue }}</div>
                </div>
            </div>
        </section>

        <!-- Status Badges -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Status Badges</h2>
            <div class="flex flex-wrap gap-3">
                <StatusBadge label="Active" variant="success" />
                <StatusBadge label="Pending" variant="warning" />
                <StatusBadge label="Overdue" variant="danger" />
                <StatusBadge label="Draft" variant="gray" />
                <StatusBadge label="In Progress" variant="info" />
            </div>
        </section>

        <!-- Modals & Contextual UI -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Modals & Feedback</h2>
            <div class="flex flex-wrap gap-4">
                <Button variant="outline" @click="isModalOpen = true">Open Generic Modal</Button>
                <Button variant="primary" @click="triggerToast('success')">Trigger Success Toast</Button>
                <Button variant="danger" @click="triggerToast('error')">Trigger Error Toast</Button>
                <Button variant="secondary" @click="triggerToast('info')">Trigger Info Toast</Button>
            </div>

            <Modal 
                :isOpen="isModalOpen" 
                title="Generic Modal Example" 
                @close="isModalOpen = false"
            >
                <div class="space-y-4">
                    <p class="text-gray-600">This is a generic modal component that can hold any content. It's built with Teleport and smooth transitions.</p>
                    <div class="p-4 bg-gray-50 rounded-lg border border-gray-100">
                        <p class="text-sm">You can put forms, tables, or any other components inside here.</p>
                    </div>
                </div>
                <template #footer>
                    <Button variant="secondary" @click="isModalOpen = false">Close</Button>
                    <Button variant="primary" @click="isModalOpen = false">Understood</Button>
                </template>
            </Modal>
        </section>



        <!-- File Upload -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">File Upload</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <FileUpload 
                        v-model="singleFile" 
                        label="Profile Photo (Single, Image only)" 
                        accept="image/*"
                    />
                </div>
                <div>
                     <FileUpload 
                        v-model="multipleFiles" 
                        label="Documents (Multiple, PDF/Docs)" 
                        multiple
                        :maxSize="10 * 1024 * 1024"
                    />
                </div>
                <div>
                     <FileUpload 
                        label="Disabled Upload" 
                        disabled
                    />
                </div>
            </div>
        </section>
        <!-- Selection Controls -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Selection Controls</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="space-y-3">
                    <label class="block text-sm font-medium text-gray-700">Radio Buttons</label>
                    <RadioButton 
                        v-model="radioValue" 
                        :options="options" 
                    />
                    <div class="mt-1 text-xs text-gray-500">Selected Radio: {{ radioValue }}</div>
                    
                    <div class="pt-4">
                        <label class="block text-sm font-medium text-gray-700 mb-2">Vertical Radio</label>
                        <RadioButton 
                            v-model="radioValue" 
                            :options="options" 
                            vertical
                        />
                    </div>
                </div>
                
                <div class="space-y-3">
                    <label class="block text-sm font-medium text-gray-700">Switches</label>
                    <div class="flex flex-col gap-4">
                        <Switch v-model="switchValue" label="Toggle me" />
                        <Switch :modelValue="true" label="Always On" />
                        <Switch :modelValue="false" label="Always Off" />
                    </div>
                    <div class="mt-1 text-xs text-gray-500">Switch State: {{ switchValue }}</div>
                </div>
            </div>
        </section>

        <!-- Tabs -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Tabs</h2>
            <Tabs v-model="activeTab" :items="tabItems" />
            <div class="p-4 bg-gray-50 rounded border border-gray-200 mt-0">
                <div v-if="activeTab === 0">Account Settings Content</div>
                <div v-if="activeTab === 1">Password Change Content</div>
                <div v-if="activeTab === 2">Notification Preferences Content</div>
            </div>
        </section>

        <!-- Cards & Typography -->
        <section class="card p-6 space-y-4">
             <h2 class="text-xl font-semibold">Card & Typography</h2>
             <p class="text-text-main">This is main text color.</p>
             <p class="text-text-muted">This is muted text color.</p>
             <div class="p-4 bg-gray-50 rounded border border-gray-200">
                 Panel content
             </div>
        </section>

        <!-- Metric Cards -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Metric Cards</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                <MetricCard
                    label="Total Revenue"
                    value="$45,231.89"
                    trend="+20.1%"
                    trendDirection="up"
                    variant="success"
                />
                 <MetricCard
                    label="Active Users"
                    value="2,345"
                    trend="+180"
                    trendDirection="up"
                    icon="users"
                    variant="primary"
                />
                 <MetricCard
                    label="Pending Issues"
                    value="12"
                    trend="-2"
                    trendDirection="down"
                    icon="alert-circle"
                    variant="warning"
                />
                <MetricCard
                    label="Avg. Response"
                    value="2m 30s"
                    trend="0%"
                    trendDirection="neutral"
                    icon="clock"
                    variant="info"
                />
                 <MetricCard
                    label="Loading Example"
                    value="0"
                    loading
                />
                 <MetricCard
                    label="Error Example"
                    error="Failed to fetch data from API"
                />
                 <MetricCard
                    label="Empty Example"
                    icon="dashboard"
                    empty
                />
                 <MetricCard
                    label="Null Value"
                    icon="calendar"
                />
            </div>
        </section>

        <!-- Timeline -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Timeline & History</h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                <div>
                    <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Standard History</h3>
                    <Timeline :items="timelineItems" />
                </div>
                <div>
                    <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Loading State</h3>
                    <Timeline loading />
                </div>
                 <div>
                    <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Empty State</h3>
                    <Timeline :items="[]" emptyText="No activity logs found." />
                </div>
            </div>
        </section>

        <!-- Workflow Steps -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Workflow Steps</h2>
            <div class="space-y-8">
                <!-- Standard Horizontal -->
                <div>
                     <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Horizontal Process (Current: Review)</h3>
                     <Steps :items="stepsItems" :current="1" />
                </div>

                 <!-- Clickable -->
                <div>
                     <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Interactive (Click to change)</h3>
                     <Steps :items="stepsItems" :current="clickableStep" clickable @change="clickableStep = $event" />
                </div>

                <!-- Vertical & Loading -->
                <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
                    <div>
                         <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Vertical</h3>
                         <Steps :items="stepsItems.slice(0, 3)" :current="1" vertical />
                    </div>
                     <div>
                         <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Loading / Empty</h3>
                         <div class="space-y-8">
                            <Steps loading />
                            <Steps :items="[]" />
                         </div>
                    </div>
                </div>
            </div>
        </section>

        <!-- Action Cards -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Workflow & Action Cards</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <!-- Standard Cards -->
                <ActionCard
                    v-for="card in actionCards"
                    :key="card.title"
                    v-bind="card"
                    @action="onCardAction"
                />

                <!-- Loading State -->
                <ActionCard
                    title="Loading Example"
                    loading
                />

                <!-- Error State -->
                <ActionCard
                    title="Error Example"
                    error="Failed to load task details."
                />

                <!-- Variant: Bordered -->
                <ActionCard
                    title="Bordered Variant"
                    description="This card uses the 'bordered' variant for a cleaner look."
                    variant="bordered"
                    :actions="[{ label: 'Dismiss', variant: 'ghost' }]"
                />
            </div>
        </section>

        <!-- Description Lists -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Data Display</h2>
            <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
                <!-- Example 1: Employee Profile (2 Cols) -->
                <div>
                    <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Entity Details (2 Cols)</h3>
                    <DescriptionList
                        title="Employee Profile"
                        :items="employeeDetails"
                        :columns="2"
                    >
                        <template #header-action>
                             <Button size="sm" variant="ghost-primary">Edit</Button>
                        </template>
                    </DescriptionList>
                </div>

                <div class="space-y-8">
                    <!-- Example 2: System Info (1 Col) -->
                    <div>
                        <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">Simple List (1 Col)</h3>
                        <DescriptionList
                            title="System Info"
                            :items="systemInfo"
                        />
                    </div>

                    <!-- Example 3: States -->
                    <div>
                         <h3 class="text-sm font-medium text-gray-500 mb-4 uppercase tracking-wider">States (Loading / Empty / Error)</h3>
                         <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                             <DescriptionList
                                :loading="true"
                                :columns="1"
                             />
                             <DescriptionList
                                :items="[]"
                                emptyText="No data found for this record."
                             />
                             <div class="md:col-span-2">
                                <DescriptionList
                                    error="Failed to load employee data. Please try again later."
                                />
                             </div>
                         </div>
                    </div>
                </div>
            </div>
        </section>
    </div>
</template>
