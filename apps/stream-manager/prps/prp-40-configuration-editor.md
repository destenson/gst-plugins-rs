# PRP-40: Configuration Editor Interface

## Overview
Create a web-based configuration editor for managing Stream Manager settings with validation and hot reload support.

## Context
- Configuration uses TOML format
- Must support hot reload via API
- Need validation before applying changes
- Show diff between current and proposed changes
- Uses Deno for frontend tooling

## Requirements
1. TOML syntax highlighting editor
2. Configuration schema validation
3. Section-based navigation
4. Diff view for changes
5. Apply and reload functionality
6. Rollback capability
7. Configuration templates
8. Import/export configuration
9. Environment variable preview

## Implementation Tasks
1. Create ConfigEditor page component
2. Integrate Monaco Editor with TOML support
3. Implement configuration schema validator
4. Create section navigation sidebar
5. Add diff viewer component
6. Implement apply configuration API call
7. Add configuration history/versions
8. Create template selector
9. Add environment variable resolver
10. Implement validation error display
11. Create configuration backup before apply
12. Add search within configuration
13. Implement context-sensitive help
14. Add configuration reload status indicator

## Editor Sections
- Server Settings
- Stream Configuration
- Recording Options
- Storage Management
- Inference Settings
- Network Configuration
- Security Settings
- Advanced Options

## Resources
- Monaco Editor: https://microsoft.github.io/monaco-editor/
- TOML parser: https://github.com/bd82/toml-tools
- Diff viewer: https://github.com/praneshr/react-diff-viewer
- JSON Schema validation: https://ajv.js.org/
- Deno with Vite: https://deno.land/manual/getting_started/setup_your_environment

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Using Deno
deno task dev

# Test configuration editor:
# 1. Editor loads with syntax highlighting
# 2. Validation shows errors in real-time
# 3. Section navigation works
# 4. Diff view shows changes
# 5. Apply sends configuration to backend
# 6. Reload status shows success/failure
# 7. Templates load correctly
# 8. Export downloads configuration file

# Run tests with Deno
deno task test

# Type checking
deno task check
```

## Success Criteria
- Editor displays configuration with syntax highlighting
- Validation catches configuration errors
- Changes can be applied and reloaded
- Diff view clearly shows modifications
- Templates provide quick configuration
- Environment variables are resolved
- Rollback restores previous configuration
- Mobile view is functional (read-only)

## Dependencies
- PRP-32 (Base layout) must be completed
- PRP-33 (API client) must be completed
- PRP-35 (Authentication) must be completed

## Deno Configuration
```json
{
  "tasks": {
    "dev": "deno run -A npm:vite",
    "build": "deno run -A npm:vite build",
    "preview": "deno run -A npm:vite preview",
    "test": "deno test -A",
    "check": "deno check **/*.ts **/*.tsx"
  }
}
```

## Estimated Effort
3 hours

## Confidence Score
8/10 - Monaco Editor is well-documented, Deno simplifies TypeScript setup