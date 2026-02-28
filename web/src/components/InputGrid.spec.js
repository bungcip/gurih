import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import InputGrid from './InputGrid.vue'
import { nextTick } from 'vue'

describe('InputGrid', () => {
  it('measures performance of inserting and removing rows', async () => {
    const columns = [
      { name: 'col1', label: 'Col 1', widget: 'TextInput' },
      { name: 'col2', label: 'Col 2', widget: 'NumberInput' },
      { name: 'col3', label: 'Col 3', widget: 'TextInput' },
      { name: 'col4', label: 'Col 4', widget: 'TextInput' },
    ]

    // Create 500 rows - enough to be measurable but won't overwhelm JSDOM
    const initialRows = Array.from({ length: 500 }, (_, i) => ({
      _id: i,
      col1: `val1-${i}`,
      col2: i,
      col3: `val3-${i}`,
      col4: `val4-${i}`,
    }))

    const wrapper = mount(InputGrid, {
      props: {
        modelValue: initialRows,
        columns
      }
    })

    // Warm up
    await wrapper.vm.$nextTick()

    const startTimeRemove = performance.now()

    // Test removing from beginning (forces Vue to update all indices if using index as key)
    let newRows = [...initialRows]
    newRows.splice(0, 1) // Remove first row

    await wrapper.setProps({ modelValue: newRows })

    const endTimeRemove = performance.now()

    console.log(`Time taken to update layout after removing 1st row (baseline): ${endTimeRemove - startTimeRemove}ms`)

    // Now adding row at the top
    const startTimeAdd = performance.now()

    newRows = [{
        _id: 'new-1',
        col1: 'new',
        col2: 0,
        col3: 'new',
        col4: 'new'
    }, ...newRows]

    await wrapper.setProps({ modelValue: newRows })

    const endTimeAdd = performance.now()

    console.log(`Time taken to update layout after adding row at top (baseline): ${endTimeAdd - startTimeAdd}ms`)
  }, 20000)
})
