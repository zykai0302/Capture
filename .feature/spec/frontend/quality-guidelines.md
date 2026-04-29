# Quality Guidelines

> Code quality standards for frontend development.

---

## Overview

This project uses Vue 3 + TypeScript + Tauri. Frontend communicates with the Rust backend via Tauri Commands (`invoke`) and Events (`listen`). All type definitions mirror the Rust backend data structures.

---

## Forbidden Patterns

| Pattern | Why | Do Instead |
|---------|-----|------------|
| `console.log` / `console.debug` | Noisy in production | Remove before committing |
| `any` type for non-error variables | Loss of type safety | Use proper TypeScript types |
| Direct `invoke()` calls without timeout | Can hang forever | Use `invokeWithTimeout()` wrapper |
| Using `source.id` as sole argument to `startStream` | Backend needs full source info | Pass complete source object |

---

## Required Patterns

### Tauri Invoke with Timeout

```typescript
const INVOKE_TIMEOUT = 30000

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out`)), INVOKE_TIMEOUT)
    ),
  ])
}
```

### Composable Pattern

Each composable follows this structure:

```typescript
export function useXxx() {
  const data = ref(...)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function refresh() { ... }
  async function action() { ... }

  let refreshInterval: ReturnType<typeof setInterval> | null = null

  onMounted(async () => {
    await refresh()
    refreshInterval = setInterval(refresh, INTERVAL_MS)
  })

  onUnmounted(() => {
    if (refreshInterval) clearInterval(refreshInterval)
  })

  return { data, loading, error, refresh, action }
}
```

### Error Handling in Composables

```typescript
try {
  await invokeWithTimeout('command', args)
  await refreshStatus()
} catch (e: any) {
  error.value = String(e)
  throw e  // Re-throw for component-level handling
}
```

`catch (e: any)` is acceptable for Tauri invoke errors since the error type from the Rust side is a serialized string.

### Tauri Event Listener Pattern

```typescript
let unlisten: UnlistenFn | null = null

onMounted(async () => {
  unlisten = await listen('event-name', (event) => {
    // Handle event
  })
})

onUnmounted(() => {
  unlisten?.()
})
```

---

## Testing Requirements

- All composables must have unit tests in `src/__tests__/`
- Tests use `@vue/test-utils` + `happy-dom` + `vitest`
- Mock `@tauri-apps/api/core` and `@tauri-apps/api/event` in tests
- `vitest run` must pass with 0 failures

---

## Code Review Checklist

- [ ] No `console.log` / `console.debug` statements
- [ ] `invokeWithTimeout` used instead of bare `invoke`
- [ ] Composable follows the standard pattern (data, loading, error, refresh)
- [ ] Event listeners cleaned up in `onUnmounted`
- [ ] TypeScript types match Rust backend (check `src/types/index.ts`)
- [ ] Tauri Command parameter names follow camelCase convention
