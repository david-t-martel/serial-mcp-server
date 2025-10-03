Machine Control Protocol (MCP) Transport & Tooling Contract
===========================================================

Version: 2025-10-03 (synchronized with code at commit time)

Overview
--------

This server implements the emerging MCP JSON-RPC 2.0 based tool invocation model
using the `rust-mcp-sdk` and its stdio transport. The transport (as currently
implemented by `rust-mcp-transport`) reads newline-delimited JSON objects from
STDIN and writes newline-delimited JSON to STDOUT. Classic `Content-Length:`
framing is therefore OPTIONAL on the client side. The server never emits
`Content-Length` prefixed frames; it writes a single JSON object per line.

Initialization Sequence
-----------------------

1. (Optional) Heartbeat notification: Unless the environment variable
   `MCP_DISABLE_HEARTBEAT` is set, the server emits a one-line JSON notification:
   `{ "jsonrpc":"2.0", "method":"_heartbeat", "params":{} }`
   immediately after startup. Clients MAY ignore unknown notifications.
2. Client sends standard MCP initialize request (JSON-RPC 2.0 request object).
3. Server replies with `InitializeResult` including `protocol_version` set to
   the current SDK's `LATEST_PROTOCOL_VERSION` constant.
4. Client issues `list_tools` (method `tools/list` or via higher-level SDK helper).
5. Client invokes tools via `tools/call` JSON-RPC method.

Method Names
------------

The correct JSON-RPC method for invoking a tool is `tools/call`.
Legacy / deprecated aliases such as `callTool` are NOT supported and will
produce JSON-RPC error `-32601` (method not found).

Tool Invocation Shape
---------------------

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "tools/call",
  "params": {
    "name": "open_port",
    "arguments": {
      "port_name": "COM3",
      "baud_rate": 115200,
      "timeout_ms": 500,
      "data_bits": "eight",
      "parity": "none",
      "stop_bits": "one",
      "flow_control": "none",
      "terminator": "\n",
      "idle_disconnect_ms": 60000
    }
  }
}
```

Success Response (abridged):

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "result": {
    "content": [{"type":"text","text":"opened"}],
    "structured_content": { "port_name": "COM3" }
  }
}
```

Error Response (invalid argument example):

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "error": { "code": -32002, "message": "invalid arguments: baud_rate missing" }
}
```

Exact error codes are determined by `rust-mcp-sdk`'s `CallToolError` mapping;
`-32601` is reserved for method (not tool) unknown.

Available Tools
---------------

See `README.md` for a human friendly list. Programmatically, call `list_tools`.

Streaming / Incremental Output
------------------------------

Not yet supported. Future work may add subscription or streaming chunk tools.

Heartbeat Behavior
------------------

Purpose: Aid harnesses/tests in quickly detecting that the server process is
alive before the initialize request/response completes. Disable by exporting
`MCP_DISABLE_HEARTBEAT=1` in the environment to produce a quieter stdout.

Framing Compatibility
---------------------

Clients sending `Content-Length:` framed messages (per some JSON-RPC transport
conventions) will still function; the underlying reader strips either newline
delimited payloads or parses framed messages (depending on future library
evolution). This server emits only newline-delimited JSON (one object per line).

Backward / Forward Compatibility Notes
--------------------------------------

* Adding new tools is a backward compatible extension (clients MUST ignore
  unknown tools until they choose to invoke them).
* Removing or renaming tools is a breaking change (requires a major version bump).
* Structured content fields are additive—clients should tolerate unknown keys.
* The heartbeat notification is optional and may gain fields.

Session Persistence Semantics
-----------------------------

* `create_session` returns a JSON object with a unique `session_id` (UUID derived).
* `append_message` returns `message_id` (monotonically increasing per session) and ISO8601 `created_at`.
* Ordering: `message_id` is guaranteed to increase strictly by 1 per append inside a session.
* Feature tags are opaque strings separated by space or comma; the server does not enforce a taxonomy.

Idle Auto-Close Event
---------------------

When `idle_disconnect_ms` is configured and the threshold elapses with no
successful I/O, the next `read` returns an auto-close event:

```json
{
  "event": "auto_close",
  "reason": "idle_timeout",
  "idle_ms": 60000,
  "idle_close_count": 1
}
```

Clients may reopen with `open_port` immediately.

Testing Guidance
----------------

Integration tests parse either newline-delimited or framed JSON (defensive).
If authoring new tests prefer the canonical newline-delimited form for clarity.

Planned Extensions (Roadmap)
----------------------------

* Pagination cursors for `list_messages` / `filter_messages`.
* `describe_port` tool returning extended hardware metadata.
* Streaming read subscription (push model) to reduce polling.
* Structured error taxonomy (machine-readable codes for common serial failures).

Change Log (Protocol Doc)
------------------------

2025-10-03: Initial extraction of protocol details into PROTOCOL.md. Clarified
method naming (`tools/call`), heartbeat semantics, and framing expectations.

---
Generated: 2025-10-03
