# Threading contract

The Python binding follows the SDK threading contract: the engine does not spawn
OS threads, and each public method runs on the OS thread that invoked it.

The engine handle's capability depends on the sync policy selected by the
builder:

- `with_full_sync()` allows concurrent calls on the same engine handle and also
  supports sequential calls from different OS threads.
- `with_local_sync()` keeps all calls on the OS thread that created the engine.
- `with_account_sync()` allows concurrent calls on the same handle when the
  caller pins each account to one processing chain (calls for the same account
  are never concurrent). Sequential cross-thread invocation is always safe.

## Recommended default

Prefer `with_local_sync()` when you do not explicitly work with multiple threads
yourself. It has zero synchronization overhead and is the right default for
embeddings that drive the engine from a single thread (synchronous code or one
asyncio loop). Use `with_full_sync()` when sharing the engine across threads
concurrently.

**Warning:** under `with_local_sync()` the engine handle must stay on the OS
thread that created it. The type system does not enforce this from Python, and
violating it is undefined behavior.

Python policy callbacks execute on the calling thread. Treat policy instances as
owned by the engine that registered them and protect any external state they
access with the caller's own synchronization model.

Snapshot semantics apply to submitted orders, reports, and account adjustments:
mutating the Python object after submission does not change the in-flight engine
operation.
