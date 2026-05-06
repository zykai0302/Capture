# Cross-Layer Thinking Guide

> **Purpose**: Think through data flow across layers before implementing.

---

## The Problem

**Most bugs happen at layer boundaries**, not within layers.

Common cross-layer bugs:
- API returns format A, frontend expects format B
- Database stores X, service transforms to Y, but loses data
- Multiple layers implement the same logic differently

---

## Before Implementing Cross-Layer Features

### Step 1: Map the Data Flow

Draw out how data moves:

```
Source → Transform → Store → Retrieve → Transform → Display
```

For each arrow, ask:
- What format is the data in?
- What could go wrong?
- Who is responsible for validation?

### Step 2: Identify Boundaries

| Boundary | Common Issues |
|----------|---------------|
| API ↔ Service | Type mismatches, missing fields |
| Service ↔ Database | Format conversions, null handling |
| Backend ↔ Frontend | Serialization, date formats |
| Component ↔ Component | Props shape changes |

### Step 3: Define Contracts

For each boundary:
- What is the exact input format?
- What is the exact output format?
- What errors can occur?

---

## Common Cross-Layer Mistakes

### Mistake 1: Implicit Format Assumptions

**Bad**: Assuming date format without checking

**Good**: Explicit format conversion at boundaries

### Mistake 2: Scattered Validation

**Bad**: Validating the same thing in multiple layers

**Good**: Validate once at the entry point

### Mistake 3: Leaky Abstractions

**Bad**: Component knows about database schema

**Good**: Each layer only knows its neighbors

---

## Checklist for Cross-Layer Features

Before implementation:
- [ ] Mapped the complete data flow
- [ ] Identified all layer boundaries
- [ ] Defined format at each boundary
- [ ] Decided where validation happens

After implementation:
- [ ] Tested with edge cases (null, empty, invalid)
- [ ] Verified error handling at each boundary
- [ ] Checked data survives round-trip

---

## When to Create Flow Documentation

Create detailed flow docs when:
- Feature spans 3+ layers
- Multiple teams are involved
- Data format is complex
- Feature has caused bugs before

---

## Tauri-Specific Cross-Layer Contracts

This project has a Rust backend ↔ TypeScript frontend boundary via Tauri Commands and Events.

### Naming Convention

| Layer | Convention | Example |
|-------|-----------|---------|
| Rust struct fields | `snake_case` | `source_id`, `rtsp_url`, `is_streaming` |
| Rust command params | `snake_case` | `source_id: String` |
| TypeScript invoke args | `camelCase` (auto-mapped) | `sourceId`, `rtspUrl`, `isStreaming` |
| TypeScript type interfaces | `snake_case` (matches serde) | `source_id`, `rtsp_url` |

Tauri automatically converts `snake_case` command parameter names to `camelCase` in JavaScript. Struct field names in serialized JSON follow Rust's `snake_case` by default (via serde).

### Type Mapping Reference

| Rust Type | TypeScript Type | Notes |
|-----------|----------------|-------|
| `String` | `string` | |
| `u32` | `number` | |
| `bool` | `boolean` | |
| `Option<T>` | `T \| null` | |
| `Vec<T>` | `T[]` | |
| `enum` (unit variants) | `string` union | Serde serializes as string |
| `enum` (tuple variants) | `{ Variant: InnerType }` | E.g., `PipelineState::Error(String)` → `{ Error: string }` |
| `struct` | `interface` | Field names stay snake_case |

### Cross-Platform Type Extension Example

When adding platform-specific fields, extend both Rust and TypeScript in sync:

**Rust** (`encode/detector.rs`):
```rust
pub struct GpuCapability {
    pub has_amf: bool,
    pub has_nvenc: bool,
    pub has_videotoolbox: bool,   // macOS
    pub has_vaapi: bool,          // Linux
    pub vt_encoders: Vec<String>, // macOS VideoToolbox
    pub vaapi_encoders: Vec<String>, // Linux VAAPI
    // ...
}
```

**TypeScript** (`types/index.ts`):
```typescript
interface GpuCapability {
  has_amf: boolean
  has_nvenc: boolean
  has_videotoolbox: boolean   // macOS
  has_vaapi: boolean          // Linux
  vt_encoders: string[]       // macOS VideoToolbox
  vaapi_encoders: string[]    // Linux VAAPI
  // ...
}
```

### Contract Verification Checklist

When adding or modifying a Tauri Command:

- [ ] Rust command function signature matches TypeScript `invoke()` args
- [ ] Rust return type matches TypeScript expected type
- [ ] New enum variants are handled in TypeScript
- [ ] `snake_case` → `camelCase` parameter mapping is correct
- [ ] Event payload types match between Rust `emit()` and TS `listen()`
