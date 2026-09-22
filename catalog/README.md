# Zero Mod Manager compatibility catalog

The production catalog is deliberately maintained as a separate public
repository. Client code in this repository accepts only a UTF-8 JSON payload
conforming to `schema/compatibility-catalog.schema.json`, wrapped as:

```json
{ "payload": "{...exact catalog JSON...}", "signature": "base64-ed25519-signature" }
```

The release public key is compiled into the client through
`ZERO_MOD_MANAGER_CATALOG_PUBLIC_KEY`; private signing material must never enter
this repository or release CI logs. The client verifies the exact payload bytes,
schema version, expiry, and monotonic catalog version before replacing its
last-known-good cache. Failed updates leave the cache untouched.

Community changes should arrive through reviewed pull requests with an evidence
URL. Author declarations, local analysis, user rules, and signed community rules
remain distinct provenance classes in the UI.
