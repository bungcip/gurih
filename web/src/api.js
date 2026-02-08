const API_BASE = 'http://localhost:3000/api'

export async function request(path, options = {}) {
    const url = path.startsWith('http') ? path : `${API_BASE}${path.startsWith('/') ? '' : '/'}${path}`
    
    // Get token from localStorage
    const stored = localStorage.getItem('user')
    let token = null
    if (stored) {
        try {
            const user = JSON.parse(stored)
            token = user.token
        } catch (e) {
            localStorage.removeItem('user')
        }
    }

    // Set default headers
    const headers = {
        'Content-Type': 'application/json',
        ...options.headers
    }

    if (token) {
        headers['Authorization'] = `Bearer ${token}`
    }

    const res = await fetch(url, {
        ...options,
        headers
    })

    if (res.status === 401) {
        // Unauthorized: Clear user and reload to trigger login
        localStorage.removeItem('user')
        window.location.reload()
        throw new Error('Unauthorized')
    }

    return res
}
