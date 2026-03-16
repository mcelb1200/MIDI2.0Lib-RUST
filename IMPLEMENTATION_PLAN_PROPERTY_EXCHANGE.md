# Phase C.2 Implementation Plan: MIDI-CI Property Exchange (PE)

## 1. Objective
To fully design the **Property Exchange (PE)** component of the MIDI-CI framework within `el_core` and the `el_ci_sim` simulator. Property Exchange enables devices to discover, get, set, and subscribe to parameters (often JSON encoded) via Universal SysEx. Because PE payloads can be large and transactional, this design enforces strict MISRA-2023 `no_std` constraints, utilizing zero dynamic allocation and bounded buffers.

## 2. Core Constraints & Challenges
- **JSON Serialization:** True JSON serialization/deserialization requires dynamic memory (`alloc`), which is forbidden in our `no_std` hot paths.
- **Transactional State:** PE relies on `Request IDs` to match Inquiries to Replies across multiple SysEx chunks.
- **Subscriptions:** Devices can subscribe to property updates, requiring state tracking of active subscribers.

## 3. Architecture & Data Structures (`el_core::ci::pe`)

### 3.1 Transaction Tracking (Request IDs)
To map asynchronous PE Replies to their original PE Inquiries without allocating a hash map, we use a statically bounded array of active transactions.

```rust
/// Represents an active, pending PE Request
#[derive(Copy, Clone, Debug)]
pub struct PendingPeRequest {
    pub request_id: u8,
    pub destination_muid: u32,
    pub timestamp: u32, // For timeout tracking
}

/// A fixed-size transaction pool (e.g., max 16 concurrent requests)
pub struct PeTransactionManager<const MAX_CONCURRENT: usize> {
    active_requests: [Option<PendingPeRequest>; MAX_CONCURRENT],
    next_request_id: u8,
}

impl<const MAX_CONCURRENT: usize> PeTransactionManager<MAX_CONCURRENT> {
    /// Allocates a new Request ID from the static array. Returns None if pool is full.
    pub fn start_request(&mut self, muid: u32) -> Option<u8>;

    /// Matches an incoming reply Request ID and clears it from the pool.
    pub fn complete_request(&mut self, request_id: u8) -> Result<(), CiErrorType>;
}
```

### 3.2 Property Exchange Headers & Payloads
PE messages consist of a Header (always JSON) and Body data. Instead of parsing the JSON into a dynamic DOM object, the `el_core` library will treat headers and payloads as strictly bounded byte slices, validating JSON structure via a lightweight, zero-allocation SAX-style tokenizer (e.g., a `no_std` fork of `serde_json_core` or custom token iterator).

```rust
/// Represents a parsed PE Header block, pointing to slices of the underlying reassembled SysEx buffer.
pub struct PeHeader<'a> {
    pub raw_json: &'a [u8],
    pub resource_str: Option<&'a str>, // Extracted via zero-copy tokenization
    pub status_code: Option<u16>,
}

pub struct PeMessage<'a> {
    pub request_id: u8,
    pub header: PeHeader<'a>,
    pub body: Option<&'a [u8]>,
}
```

### 3.3 Chunking Large Properties (MT=0x3 / MT=0x5)
When a Responder needs to send a large Property (e.g., a 4KB preset file), it must chunk the PE Reply.

```rust
/// An iterator that chunks a combined PE Header and Body into sequential SysEx UMPs.
pub struct PeReplyChunker<'a> {
    header: &'a [u8],
    body: &'a [u8],
    request_id: u8,
    offset: usize,
    chunk_size: u16, // Configured by MIDI-CI negotiation
}

impl<'a> Iterator for PeReplyChunker<'a> {
    type Item = Ump;
    fn next(&mut self) -> Option<Self::Item>; // Handles Start, Continue, End status
}
```

### 3.4 Subscription State Machine
If a device supports Subscriptions, it must track which MUIDs are subscribed to which resources.

```rust
pub struct PeSubscription {
    pub subscriber_muid: u32,
    // Store a hash of the resource string instead of allocating the string itself
    pub resource_hash: u32,
}

/// A fixed-size array of active subscriptions
pub struct SubscriptionManager<const MAX_SUBS: usize> {
    subscriptions: [Option<PeSubscription>; MAX_SUBS],
}

impl<const MAX_SUBS: usize> SubscriptionManager<MAX_SUBS> {
    pub fn add_subscription(&mut self, muid: u32, resource_hash: u32) -> Result<(), CiErrorType>;
    pub fn remove_subscription(&mut self, muid: u32, resource_hash: u32);

    /// Returns an iterator of MUIDs that need to be notified when a resource changes.
    pub fn get_subscribers(&self, resource_hash: u32) -> impl Iterator<Item = u32> + '_;
}
```

## 4. Simulator Integration (`el_ci_sim`)

### 4.1 Inquiry Simulation
The CLI simulator will support generating specific PE requests:
- `el_ci_sim pe get --resource "DeviceInfo"`
- `el_ci_sim pe set --resource "Channel_1_Volume" --value 100`
- `el_ci_sim pe subscribe --resource "PresetList"`

### 4.2 Reassembly & Execution Flow
1. **Receive UMPs:** Parse UMPs into the `CiSysExReassembler<1024>` (defined in Phase C/Architecture).
2. **Decode PE Type:** Identify Inquiry vs Reply, extract Request ID.
3. **Validate:** Check `PeTransactionManager` to see if the Request ID matches a pending inquiry.
4. **Tokenize Header:** Use a zero-copy JSON tokenizer to find the `"resource"` string.
5. **Route:** Pass the PE Header and Body slices to the specific component callback.

## 5. Testing & Verification

1. **Transactional Integrity:** Simulate 16 concurrent PE Inquiries. Assert the `PeTransactionManager` correctly matches the 16 incoming PE Replies, and correctly rejects a 17th request due to the bounded pool.
2. **Zero-Copy JSON Extraction:** Feed a raw `&[u8]` containing a valid PE JSON header. Assert the tokenizer successfully extracts the `resource` value slice `&str` without performing any memory allocations.
3. **Subscription Hashing:** Verify that subscribing to `"DeviceInfo"` correctly hashes to a `u32` and stores in `SubscriptionManager`, and a subsequent Notification matches that hash.