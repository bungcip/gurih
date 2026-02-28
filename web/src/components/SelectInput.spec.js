import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import SelectInput from './SelectInput.vue'

describe('SelectInput', () => {
  it('renders correctly', () => {
    const wrapper = mount(SelectInput, {
      props: {
        modelValue: null,
        options: [{value: 1, label: "One"}],
        placeholder: 'Select an option'
      }
    })
    expect(wrapper.exists()).toBe(true)
  })
})
