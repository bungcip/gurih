<script setup>
import { ref, watch, onMounted } from 'vue'
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
import ProgressBar from './ProgressBar.vue'
import DescriptionList from './DescriptionList.vue'
import ActionCard from './ActionCard.vue'
import Alert from './Alert.vue'
import Drawer from './Drawer.vue'
import TreeView from './TreeView.vue'
import DiscussionPanel from './DiscussionPanel.vue'
import EmptyState from './EmptyState.vue'
import { inject } from 'vue'

const isDarkMode = ref(false)

watch(isDarkMode, (val) => {
  if (val) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
})

onMounted(() => {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
        isDarkMode.value = true
    }
})

const selectValue = ref(null)
const dateValue = ref('')
const passwordValue = ref('')
const singleFile = ref(null)
const multipleFiles = ref([])
const amountValue = ref(1500000)
const isModalOpen = ref(false)
const loading = ref(false)
const showAlert = ref(true)
const isDrawerOpen = ref(false)
const drawerPlacement = ref('right')
const isDrawerLoading = ref(false)
const drawerMode = ref('details')
const drawerTitle = ref('Employee Details')

const showToast = inject('showToast')

function openDrawer(mode) {
    isDrawerOpen.value = true
    drawerMode.value = mode

    if (mode === 'left') {
        drawerPlacement.value = 'left'
        drawerTitle.value = 'Left Drawer'
        drawerMode.value = 'details'
        isDrawerLoading.value = false
    } else {
        drawerPlacement.value = 'right'

        if (mode === 'loading') {
            drawerTitle.value = 'Loading Data...'
            isDrawerLoading.value = true
        } else if (mode === 'empty') {
            drawerTitle.value = 'Empty State'
            isDrawerLoading.value = false
        } else {
            drawerTitle.value = 'Employee Details'
            isDrawerLoading.value = false
        }
    }
}

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
const tabItems = [
    { label: 'Account', badge: 5 },
    { label: 'Security' },
    { label: 'Notifications', badge: 12 }
]

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

const treeData = [
  {
    id: 'root',
    label: 'Corporate Headquarters',
    icon: 'dashboard',
    children: [
      {
        id: 'finance',
        label: 'Finance & Legal',
        icon: 'users',
        children: [
          { id: 'acc', label: 'Accounting', icon: 'user' },
          { id: 'audit', label: 'Internal Audit', icon: 'user' },
          { id: 'legal', label: 'Legal Counsel', icon: 'user' }
        ]
      },
      {
        id: 'tech',
        label: 'Technology Division',
        icon: 'settings',
        children: [
          {
            id: 'eng',
            label: 'Engineering',
            icon: 'users',
            children: [
                { id: 'fe', label: 'Frontend Guild', icon: 'user' },
                { id: 'be', label: 'Backend Guild', icon: 'user' }
            ]
          },
          { id: 'infra', label: 'Infrastructure', icon: 'settings' }
        ]
      }
    ]
  }
]
const treeExpanded = ref(['root', 'tech', 'eng'])
const treeSelected = ref('fe')

const discussionItems = ref([
    { id: 1, author: 'Alice Smith', date: '2 hours ago', content: 'Customer requested a callback regarding the Q4 invoice.' },
    { id: 2, author: 'Bob Jones', date: '1 hour ago', content: 'I tried calling but no answer. Left a voicemail.' },
    { id: 3, author: 'Charlie Day', date: 'Just now', content: 'They just emailed back. Updating the ticket status.', avatar: 'CD' }
])

const progressValue = ref(65)

function onDiscussionSubmit(text) {
    discussionItems.value.push({
        id: Date.now(),
        author: 'You',
        date: 'Just now',
        content: text
    })
    triggerToast('success')
}
</script>

<template>
    <div class="p-8 max-w-4xl mx-auto space-y-8">
        <div class="flex justify-between items-center">
            <div>
                <h1 class="text-3xl font-bold text-text-main">Kitchen Sink</h1>
                <p class="text-text-muted">Component demonstration and test ground.</p>
            </div>
            <Switch v-model="isDarkMode" label="Dark Mode" />
        </div>

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
            
            <div class="flex flex-wrap gap-4 pt-4 border-t border-gray-100 dark:border-gray-700">
                <Button variant="primary" icon="lucide:plus">Add Item</Button>
                <Button variant="outline" icon="lucide:settings">Settings</Button>
                <Button variant="danger" icon="lucide:trash-2" iconPosition="right">Delete</Button>
                <Button variant="secondary" icon="lucide:users">View Users</Button>
                <Button variant="ghost-primary" icon="lucide:calendar">Pick Date</Button>
            </div>
        </section>

        <!-- Inputs -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Inputs</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Standard Input</label>
                    <input type="text" class="input-field" placeholder="Type something...">
                </div>
                <div>
                     <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Select Input (New)</label>
                     <SelectInput 
                        v-model="selectValue" 
                        :options="options" 
                        placeholder="Choose an option"
                     />
                     <div class="mt-1 text-xs text-text-muted">Selected Value: {{ selectValue }}</div>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Date Picker (New)</label>
                    <DatePicker v-model="dateValue" />
                    <div class="mt-1 text-xs text-text-muted">Selected Date: {{ dateValue }}</div>
                </div>
                <div>
                     <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Password Input (New)</label>
                     <Password v-model="passwordValue" placeholder="Enter password" />
                     <div class="mt-2">
                        <Password v-model="passwordValue" placeholder="Disabled password" disabled />
                     </div>
                </div>
                <div>
                     <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Disabled Input</label>
                     <input type="text" class="input-field" placeholder="Disabled input..." disabled>
                </div>
                <div>
                     <CurrencyInput v-model="amountValue" label="Currency Input (New)" />
                     <div class="mt-1 text-xs text-text-muted">Raw Numeric Value: {{ amountValue }}</div>
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

        <!-- Alerts -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Alerts</h2>
            <div class="space-y-4">
                <Alert title="Information" description="This is a standard info alert." />
                <Alert variant="success" title="Success" description="Your changes have been saved successfully." />
                <Alert variant="warning" title="Warning" description="Your account subscription expires in 3 days." />
                <Alert variant="danger" title="Error" description="Failed to connect to the database." />

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                     <Alert variant="info" title="Loading State" loading />
                     <Alert v-if="showAlert" variant="success" title="Closable Alert" description="Click the X to dismiss me." closable @close="showAlert = false" />
                     <div v-else class="h-full min-h-[60px] flex items-center justify-center bg-gray-50 dark:bg-gray-800 border border-dashed rounded text-sm text-text-muted">
                        <Button size="sm" variant="ghost-primary" @click="showAlert = true">Reset Alert</Button>
                     </div>
                </div>

                <Alert title="With Action Slot">
                    <p>You have unsaved changes. Do you want to save them now?</p>
                    <template #action>
                        <div class="flex gap-2">
                             <Button size="sm" variant="ghost-danger">Discard</Button>
                             <Button size="sm" variant="outline">Save Draft</Button>
                        </div>
                    </template>
                </Alert>
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
                    <p class="text-text-muted">This is a generic modal component that can hold any content. It's built with Teleport and smooth transitions.</p>
                    <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-100 dark:border-gray-700">
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
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Radio Buttons</label>
                    <RadioButton 
                        v-model="radioValue" 
                        :options="options" 
                    />
                    <div class="mt-1 text-xs text-text-muted">Selected Radio: {{ radioValue }}</div>
                    
                    <div class="pt-4">
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Vertical Radio</label>
                        <RadioButton 
                            v-model="radioValue" 
                            :options="options" 
                            vertical
                        />
                    </div>
                </div>
                
                <div class="space-y-3">
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Switches</label>
                    <div class="flex flex-col gap-4">
                        <Switch v-model="switchValue" label="Toggle me" />
                        <Switch :modelValue="true" label="Always On" />
                        <Switch :modelValue="false" label="Always Off" />
                    </div>
                    <div class="mt-1 text-xs text-text-muted">Switch State: {{ switchValue }}</div>
                </div>
            </div>
        </section>

        <!-- Tabs -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Tabs</h2>
            <Tabs v-model="activeTab" :items="tabItems" />
            <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700 mt-0">
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
             <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700">
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

        <!-- Progress Indicators -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Progress Indicators</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
                <div class="space-y-6">
                    <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">Standard Variants</h3>
                    <ProgressBar :value="75" label="Project Completion" variant="primary" showValue />
                    <ProgressBar :value="50" label="Budget Utilization" variant="warning" showValue />
                    <ProgressBar :value="92" label="Server CPU Load" variant="danger" showValue />
                    <ProgressBar :value="100" label="Onboarding Task" variant="success" showValue />
                </div>
                <div class="space-y-6">
                    <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">Styles & Sizes</h3>
                    <ProgressBar :value="progressValue" label="Processing Data (Striped)" variant="info" striped showValue />
                    <ProgressBar :value="20" label="Small Size" size="sm" />
                    <ProgressBar :value="60" label="Large Size" size="lg" showValue />
                    <ProgressBar :value="80" label="Extra Large (Text Inside)" size="xl" showValue variant="primary" striped />
                </div>
                <div class="md:col-span-2 grid grid-cols-1 md:grid-cols-2 gap-8">
                     <div class="space-y-6">
                        <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">States</h3>
                        <ProgressBar :loading="true" label="Calculating storage..." />
                        <ProgressBar error="Failed to fetch quota limits." label="Storage Quota" :value="0" />
                     </div>
                </div>
            </div>
        </section>

        <!-- Timeline -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Timeline & History</h2>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-8">
                <div>
                    <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Standard History</h3>
                    <Timeline :items="timelineItems" />
                </div>
                <div>
                    <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Loading State</h3>
                    <Timeline loading />
                </div>
                 <div>
                    <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Empty State</h3>
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
                     <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Horizontal Process (Current: Review)</h3>
                     <Steps :items="stepsItems" :current="1" />
                </div>

                 <!-- Clickable -->
                <div>
                     <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Interactive (Click to change)</h3>
                     <Steps :items="stepsItems" :current="clickableStep" clickable @change="clickableStep = $event" />
                </div>

                <!-- Vertical & Loading -->
                <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
                    <div>
                         <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Vertical</h3>
                         <Steps :items="stepsItems.slice(0, 3)" :current="1" vertical />
                    </div>
                     <div>
                         <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Loading / Empty</h3>
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
                    <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Entity Details (2 Cols)</h3>
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
                        <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Simple List (1 Col)</h3>
                        <DescriptionList
                            title="System Info"
                            :items="systemInfo"
                        />
                    </div>

                    <!-- Example 3: States -->
                    <div>
                         <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">States (Loading / Empty / Error)</h3>
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

        <!-- Empty States -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Empty & Feedback States</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <!-- Standard -->
                <EmptyState
                    title="No Team Members"
                    description="Get started by inviting your colleagues."
                    icon="users"
                    actionLabel="Invite Member"
                    @action="triggerToast('success')"
                />

                <!-- Simple -->
                <EmptyState
                    title="No Data Available"
                    description="There are no records to display at this time."
                    icon="dashboard"
                />

                <!-- Loading -->
                <EmptyState loading />

                <!-- Error -->
                <EmptyState
                    title="Failed to Load Data"
                    description="There was a problem connecting to the server."
                    icon="alert-circle"
                    error
                    actionLabel="Retry Connection"
                    @action="triggerToast('info')"
                />

                 <!-- Custom Slot -->
                <EmptyState
                    title="System Offline"
                    description="Maintenance is currently in progress."
                    icon="settings"
                >
                    <template #action>
                        <div class="flex gap-2">
                             <Button size="sm" variant="outline">Check Status</Button>
                             <Button size="sm" variant="ghost">Dismiss</Button>
                        </div>
                    </template>
                </EmptyState>
            </div>
        </section>

        <!-- Drawers -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Drawers & Slide-overs</h2>
            <div class="flex flex-wrap gap-4">
                <Button variant="primary" @click="openDrawer('details')">Open Right Drawer</Button>
                <Button variant="outline" @click="openDrawer('left')">Open Left Drawer</Button>
                <Button variant="secondary" @click="openDrawer('loading')">Open Loading Drawer</Button>
                <Button variant="ghost" @click="openDrawer('empty')">Open Empty Drawer</Button>
            </div>

            <Drawer
                :isOpen="isDrawerOpen"
                :title="drawerTitle"
                :placement="drawerPlacement"
                :loading="isDrawerLoading"
                @close="isDrawerOpen = false"
            >
                <div v-if="drawerMode === 'details' || drawerMode === 'left'" class="space-y-6">
                    <Alert variant="info" title="Context Preserved" description="Drawers allow users to view details without losing their place in the list." />

                    <DescriptionList
                        title="Personal Information"
                        :items="employeeDetails"
                        :columns="1"
                    />

                    <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700">
                         <h4 class="font-medium text-text-main mb-2">History</h4>
                         <Timeline :items="timelineItems.slice(0, 2)" />
                    </div>
                </div>

                <div v-else-if="drawerMode === 'empty'" class="flex flex-col items-center justify-center h-64 text-center">
                    <div class="text-4xl mb-4">📭</div>
                    <h3 class="font-medium text-text-main">No Content</h3>
                    <p class="text-text-muted">This drawer is currently empty.</p>
                </div>

                <template #footer>
                    <Button variant="ghost" @click="isDrawerOpen = false">Close</Button>
                    <Button v-if="drawerMode === 'details' || drawerMode === 'left'" variant="primary" @click="isDrawerOpen = false; triggerToast('success')">Save Changes</Button>
                </template>
            </Drawer>
        </section>

        <!-- Tree View -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Hierarchical Data (Tree View)</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
                <!-- Interactive Tree -->
                <div class="space-y-4">
                    <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">Organizational Structure</h3>
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg min-h-[300px]">
                        <TreeView
                            :nodes="treeData"
                            v-model="treeSelected"
                            v-model:expandedKeys="treeExpanded"
                        />
                    </div>
                    <div class="text-xs text-text-muted">
                        Selected: <span class="font-mono">{{ treeSelected }}</span>
                    </div>
                </div>

                <!-- Loading State -->
                <div class="space-y-4">
                    <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">Loading State</h3>
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg min-h-[300px]">
                        <TreeView loading />
                    </div>
                </div>

                <!-- Empty State -->
                <div class="space-y-4">
                    <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">Empty State</h3>
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg min-h-[300px]">
                        <TreeView :nodes="[]" emptyText="No organization data found." />
                    </div>
                </div>

                <!-- Error State -->
                <div class="space-y-4">
                    <h3 class="text-sm font-medium text-text-muted uppercase tracking-wider">Error State</h3>
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg min-h-[300px]">
                        <TreeView error="Failed to load hierarchy." />
                    </div>
                </div>
            </div>
        </section>

        <!-- Collaboration & Notes -->
        <section class="card p-6 space-y-4">
            <h2 class="text-xl font-semibold">Collaboration & Notes</h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
                <!-- Interactive -->
                <div class="h-[400px]">
                    <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Interactive Discussion</h3>
                    <DiscussionPanel
                        title="Ticket Notes"
                        :items="discussionItems"
                        @submit="onDiscussionSubmit"
                    />
                </div>

                <!-- Read Only -->
                <div class="h-[400px]">
                     <h3 class="text-sm font-medium text-text-muted mb-4 uppercase tracking-wider">Read Only</h3>
                     <DiscussionPanel
                        title="Audit Log (Read Only)"
                        :items="discussionItems.slice(0, 2)"
                        readOnly
                    />
                </div>

                 <!-- Loading / Empty / Error -->
                 <div class="space-y-8 h-[400px] flex flex-col">
                     <div class="flex-1 min-h-0">
                         <h3 class="text-sm font-medium text-text-muted mb-2 uppercase tracking-wider">Loading State</h3>
                         <DiscussionPanel loading />
                     </div>
                     <div class="flex-1 min-h-0">
                         <h3 class="text-sm font-medium text-text-muted mb-2 uppercase tracking-wider">Empty State</h3>
                         <DiscussionPanel :items="[]" />
                     </div>
                </div>
            </div>
        </section>
    </div>
</template>
