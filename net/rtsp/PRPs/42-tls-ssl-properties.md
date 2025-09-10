# PRP-RTSP-42: TLS/SSL Security Properties Implementation

## Overview
Implement TLS/SSL security properties (`tls-database`, `tls-interaction`, `tls-validation-flags`) to match original rtspsrc secure connection capabilities for RTSP over TLS.

## Context
The original rtspsrc provides comprehensive TLS/SSL configuration through properties for certificate authority databases, user interaction objects, and certificate validation flags. These are essential for secure RTSP connections (rtsps://) and certificate validation.

## Research Context
- Original rtspsrc TLS properties in `~/repos/gstreamer/subprojects/gst-plugins-good/gst/rtsp/gstrtspsrc.c`
- GIO TLS documentation: https://docs.gtk.org/gio/tls-overview.html
- GTlsCertificateFlags enumeration values
- GTlsDatabase and GTlsInteraction object types
- X.509 certificate validation in GStreamer applications

## Scope
This PRP implements ONLY the property infrastructure:
1. Add `tls-database` object property for GTlsDatabase (default: null)
2. Add `tls-interaction` object property for GTlsInteraction (default: null)  
3. Add `tls-validation-flags` flags property for GTlsCertificateFlags (default: validate-all)
4. Add property validation and object reference management

Does NOT implement:
- Actual TLS connection establishment
- Certificate validation logic
- User interaction for certificate prompts
- TLS handshake processing

## Implementation Tasks
1. Add TLS security fields to RtspSrcSettings struct
2. Implement `tls-database` object property with GTlsDatabase type
3. Implement `tls-interaction` object property with GTlsInteraction type
4. Implement `tls-validation-flags` with GTlsCertificateFlags enumeration
5. Add object reference counting and cleanup
6. Add property change restrictions (changeable only in NULL or READY state)
7. Document certificate validation flag combinations

## Files to Modify
- `net/rtsp/src/rtspsrc/imp.rs` - Property definitions and object handling
- May need GObject property wrapper utilities
- Cargo.toml - Add GIO/GLib dependencies for TLS types

## Validation Gates
```bash
# Syntax/Style
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Unit Tests
cargo test rtspsrc_tls_properties --all-targets --all-features -- --nocapture

# TLS Flags Test
cargo test test_tls_validation_flags --all-targets --all-features -- --nocapture
```

## Expected Behavior
After implementation, `gst-inspect-1.0 rtspsrc2` should show:
```
tls-database        : TLS database with anchor certificate authorities used to validate the server certificate
                      flags: readable, writable, changeable only in NULL or READY state
                      Object of type "GTlsDatabase"

tls-interaction     : A GTlsInteraction object to prompt the user for password or certificate  
                      flags: readable, writable, changeable only in NULL or READY state
                      Object of type "GTlsInteraction"

tls-validation-flags: TLS certificate validation flags used to validate the server certificate
                      flags: readable, writable, changeable only in NULL or READY state
                      Flags "GTlsCertificateFlags" Default: 0x0000007f, "validate-all"
                         (0x00000000): no-flags         - G_TLS_CERTIFICATE_NO_FLAGS
                         (0x00000001): unknown-ca       - G_TLS_CERTIFICATE_UNKNOWN_CA
                         (0x00000002): bad-identity     - G_TLS_CERTIFICATE_BAD_IDENTITY  
                         (0x00000004): not-activated    - G_TLS_CERTIFICATE_NOT_ACTIVATED
                         (0x00000008): expired          - G_TLS_CERTIFICATE_EXPIRED
                         (0x00000010): revoked          - G_TLS_CERTIFICATE_REVOKED
                         (0x00000020): insecure         - G_TLS_CERTIFICATE_INSECURE
                         (0x00000040): generic-error    - G_TLS_CERTIFICATE_GENERIC_ERROR
                         (0x0000007f): validate-all     - G_TLS_CERTIFICATE_VALIDATE_ALL
```

## Property Behavior Details
- **tls-database**: Certificate authority database for validating server certificates
- **tls-interaction**: User interaction object for prompting certificate/password dialogs
- **tls-validation-flags**: Bitwise flags controlling which certificate checks to perform

## GTlsCertificateFlags Values
- **no-flags (0x00)**: Accept any certificate
- **unknown-ca (0x01)**: Reject certificates from unknown CAs
- **bad-identity (0x02)**: Reject certificates with wrong identity  
- **not-activated (0x04)**: Reject certificates not yet valid
- **expired (0x08)**: Reject expired certificates
- **revoked (0x10)**: Reject revoked certificates
- **insecure (0x20)**: Reject insecure certificates
- **generic-error (0x40)**: Reject certificates with generic errors
- **validate-all (0x7f)**: Enable all validation checks (default)

## Object Property Handling
- Object properties require proper reference counting
- Objects can be null (default state)
- Need cleanup in element finalization
- Thread-safe object access considerations

## Dependencies
- **GIO/GLib**: GTlsDatabase, GTlsInteraction, GTlsCertificateFlags types
- **Object management**: GObject reference counting utilities

## Success Criteria
- [ ] All three properties visible in gst-inspect output
- [ ] Object properties accept null and valid object values
- [ ] tls-validation-flags shows all flag combinations correctly
- [ ] Object reference counting works properly (no leaks)
- [ ] Properties changeable only in NULL/READY states  
- [ ] Property defaults match original rtspsrc (null, null, validate-all)
- [ ] No actual TLS connection logic implemented (out of scope)

## Risk Assessment
**MEDIUM-HIGH RISK** - Object property handling and GObject integration complexity.

## Estimated Effort
4-5 hours (object properties and reference management)

## Confidence Score
6/10 - Object properties and GObject integration add significant complexity.