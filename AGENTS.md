# Agent Guidelines for Steward Discord Bot

## Build/Lint/Test Commands
- **Start**: `bun run src/main.ts`
- **Type check**: `npm run typecheck` or `tsc --noEmit`
- **Lint**: `npm run check` or `biome check`
- **Fix**: `npm run fix` or `biome check --write --unsafe`
- **No test framework configured**

## Code Style Guidelines

### TypeScript
- Strict mode enabled with custom overrides (noUnusedLocals/parameters disabled)
- Use `import type` for type-only imports
- Path mapping: `@/*` → `./src/*`
- ESNext target with ES modules

### Formatting
- 2-space indentation (Biome formatter)
- Biome linting with recommended rules (useImportExtensions disabled)
- Organize imports automatically enabled

### Naming & Structure
- `const` for all variables (no `let`/`var`)
- Arrow functions preferred
- Interfaces for object types
- `Record<string, T>` for dynamic objects
- Async/await for asynchronous code

### Error Handling
- Early returns for guard clauses
- Null checks: `!= null` and `== null`
- Optional chaining (`?.`) and nullish coalescing (`??`)
- Try/catch blocks for expected errors
- Throw `new Error()` for fatal errors

### Imports
- Node.js built-ins: `import fs from "node:fs"`
- JSON imports: `import data from "@/data.json" with { type: "json" }`
- Group imports by type (built-ins, external, internal)