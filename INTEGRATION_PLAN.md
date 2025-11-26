# MCP Handler PortService Integration Plan

## Overview
This document outlines the plan to refactor the MCP handler to use a shared PortService layer instead of directly manipulating PortState.

## Current State Analysis

### MCP Handler Methods (src/mcp.rs)
The SerialServerHandler currently implements these port operation methods:
- `open_port_impl()` - Opens port, updates PortState directly
- `write_impl()` - Writes data, updates metrics in PortState
- `read_impl()` - Reads data, handles timeouts, updates metrics
- `close_impl()` - Closes port, resets state
- `status_impl()` - Returns current PortState status
- `metrics_impl()` - Returns metrics from PortState
- `reconfigure_port_impl()` - Closes and reopens with new config

### Expected PortService Interface
Based on the code analysis, the PortService should provide:

```rust
pub struct PortService {
    state: AppState,
}

impl PortService {
    pub fn new(state: AppState) -> Self;

    pub fn open(&self, config: OpenConfig) -> Result<(), ServiceError>;
    pub fn write(&self, data: String) -> Result<WriteMetrics, ServiceError>;
    pub fn read(&self) -> Result<ReadResult, ServiceError>;
    pub fn close(&self) -> Result<(), ServiceError>;
    pub fn status(&self) -> Result<PortStatus, ServiceError>;
    pub fn metrics(&self) -> Result<PortMetrics, ServiceError>;
    pub fn reconfigure(&self, config: OpenConfig) -> Result<(), ServiceError>;
}
```

### Error Mapping Required
ServiceError → CallToolError mapping needed for MCP responses

## Integration Steps

### Step 1: Update SerialServerHandler Structure
```rust
pub struct SerialServerHandler {
    pub service: Arc<PortService>,
    pub sessions: SessionStore,
}
```

### Step 2: Update Constructor
Constructor should accept PortService instead of AppState

### Step 3: Refactor Each Method
Each `*_impl` method should delegate to PortService and map responses

### Step 4: Update main.rs
Create PortService and pass to both REST and MCP handlers

### Step 5: Update rest_api.rs
Update REST handlers to use PortService (separate task)

## Key Considerations

1. **Backward Compatibility**: MCP response JSON structure must remain unchanged
2. **Shared State**: Service must be Arc-wrapped for sharing
3. **Error Mapping**: ServiceError must map cleanly to CallToolError
4. **Metrics Preservation**: All existing metrics must be maintained
5. **Session Independence**: Session handling remains separate from port operations

## Testing Requirements

1. Verify all MCP tools still work correctly
2. Ensure JSON response structure is unchanged
3. Test concurrent access via REST and MCP
4. Validate error messages are preserved
5. Confirm metrics accuracy

## Status
Waiting for PortService module to be created in src/service/mod.rs
