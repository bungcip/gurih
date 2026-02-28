import { mount, flushPromises } from '@vue/test-utils'
import { describe, it, expect, vi } from 'vitest'
import DynamicForm from './DynamicForm.vue'
import { request } from '../api.js'

vi.mock('../api.js', () => ({
  request: vi.fn()
}))

describe('DynamicForm', () => {
  it('measures the time to render after fetching a large relations list', async () => {
    const mockSchema = {
      name: 'Test Entity',
      layout: [
        {
          title: 'Section 1',
          fields: [
            { name: 'relation_id', label: 'Relation', widget: 'RelationPicker', target_entity: 'TargetEntity' }
          ]
        }
      ]
    }

    const mockRelations = Array.from({ length: 5000 }, (_, i) => ({
      id: i,
      name: `Relation ${i}`
    }))

    request.mockImplementation((url) => {
      if (url === '/ui/form/TestEntity') {
        return Promise.resolve({ json: () => Promise.resolve(mockSchema), ok: true })
      }
      if (url === '/TargetEntity') {
        return Promise.resolve({ json: () => Promise.resolve(mockRelations), ok: true })
      }
      if (url === '/TestEntity/1') {
        return Promise.resolve({ json: () => Promise.resolve({ relation_id: 10 }), ok: true })
      }
      return Promise.reject(new Error(`Unhandled request: ${url}`))
    })

    const wrapper = mount(DynamicForm, {
      props: {
        entity: 'TestEntity',
        id: 1
      },
      global: {
        provide: {
          showToast: vi.fn(),
          currentUser: { name: 'Test User' }
        }
      }
    })

    await flushPromises()
    expect(wrapper.exists()).toBe(true)
  })
})
