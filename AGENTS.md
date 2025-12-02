<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

# General Guidelines for working with Nx

- When running tasks (for example build, lint, test, e2e, etc.), always prefer running the task through `nx` (i.e. `nx run`, `nx run-many`, `nx affected`) instead of using the underlying tooling directly
- You have access to the Nx MCP server and its tools, use them to help the user
- When answering questions about the repository, use the `nx_workspace` tool first to gain an understanding of the workspace architecture where applicable.
- When working in individual projects, use the `nx_project_details` mcp tool to analyze and understand the specific project structure and dependencies
- For questions around nx configuration, best practices or if you're unsure, use the `nx_docs` tool to get relevant, up-to-date docs. Always use this instead of assuming things about nx configuration
- If the user needs help with an Nx configuration or project graph error, use the `nx_workspace` tool to get any errors

<!-- nx configuration end-->

# Project Guidelines

## 🤖 Agent Protocols

These rules apply to all AI agents (Warp, Antigravity, etc.) working on the Duumbi project.

1.  **Check the Plan**: Before starting any complex task, always check `task.md` and `implementation_plan.md` in the `.gemini/antigravity/brain/...` directory (if available) or the root `.agent` directory.
2.  **Atomic Documentation**: When creating new documentation, follow the "Atomic Ideas" structure in `docs/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/`. Each file should focus on a single concept.
3.  **Code Style**: Strictly follow the rules defined in `.agent/rules/code-style-guide.md`.
4.  **No "Magic" Fixes**: If you encounter an error, explain the cause before fixing it. Do not blindly apply fixes without understanding the root cause.
5.  **Diagrams**: Create diagrams using **PlantUML**. Save the source files in `docs/02 Resources (Assets and Tools)/Attachments (MediaFiles)`.

## Commit Messages

- Use Conventional Commits format.
- Start the commit with a gitmoji which corresponds to the message the most.
- Include scope when the change affects a specific app/library (e.g., `feat(web):`, `fix(core-api):`).
- Write clear, actionable commit subjects in imperative mood.
- Add body text for complex changes explaining the "why" behind the change.

## Code Organization & Architecture

### Monorepo Structure

- Follow Nx workspace conventions: separate apps and libs clearly.
- Keep libraries small, focused, and reusable (single responsibility).
- Use path aliases from tsconfig for clean imports (e.g., `@duumbi/ui` instead of relative paths).
- Place shared types in `ts-models` library, not duplicated across projects.

### TypeScript Best Practices

- Enable strict TypeScript flags for type safety.
- Define explicit return types for public APIs and exported functions.
- Use interfaces for public contracts, types for internal models.
- Avoid `any`; use `unknown` with type guards when type is uncertain.

### Component Development (React)

- Colocate component files: `ComponentName.tsx`, `ComponentName.test.tsx`, `ComponentName.stories.tsx`.
- Extract reusable components to `@duumbi/ui` library.
- Use composition over inheritance; prefer small, composable components.
- Implement proper prop typing with TypeScript interfaces.
- Follow naming: PascalCase for components, camelCase for functions/variables.

### Styling (Tailwind CSS)

- Use Tailwind utility classes directly in components.
- Create reusable design tokens in Tailwind config for consistency.
- Extract repeated utility combinations into component variants or CSS classes.
- Ensure responsive design: mobile-first approach with responsive utilities.

### State Management

- Keep state as local as possible; lift only when necessary.
- Use React Context for theme, i18n, and app-wide settings.
- For complex state, consider dedicated state management (document chosen solution).

### Internationalization (i18n)

- All user-facing text must use i18n keys, never hardcoded strings.
- Organize translation keys by feature/component namespace.
- Include context comments for translators in key definitions.

## Testing Strategy

### Unit Tests

- Write tests for all business logic and utility functions.
- Test components: user interactions, conditional rendering, edge cases.
- Use Jest + React Testing Library for component tests.
- Aim for meaningful coverage, not just metrics.

### Running Tests

- Use `nx test <project>` to run individual project tests.
- Use `nx affected --target=test` to test only changed code.
- Run tests before committing (consider adding to pre-commit hook).

## Code Quality

### Linting & Formatting

- Run `nx lint <project>` before committing changes.
- Run `nx run-many --target=lint` to lint all projects.
- Fix linting issues, don't disable rules without good reason.
- Use ESLint for code quality, Prettier for formatting (if configured).

### Type Checking

- Run `nx run-many --target=typecheck` to verify TypeScript across projects.
- Ensure no TypeScript errors before merging.

### Pre-commit Checks

- Always run `pre-commit run --all-files` before committing.
- Fix all issues reported by pre-commit hooks.

## Development Workflow

### Creating New Features

1. Create feature branch from `main` with descriptive name (e.g., `feat/agent-query-component`).
2. Plan component/feature structure before coding.
3. Implement following architecture guidelines.
4. Write tests alongside implementation.
5. Run lint, typecheck, and tests.
6. Commit with proper message format.
7. Create PR with clear description of changes.

### Refactoring

- Make refactoring commits separate from feature commits.
- Use `refactor:` type for non-functional code improvements.
- Ensure tests pass before and after refactoring.
- Document breaking changes in commit body.

## Documentation

- Update README.md when adding new apps/libraries.
- Document architecture decisions in `docs/01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/Architecture.md`.
- Add JSDoc comments for exported functions/components.
- Keep wireframe and design docs in Obsidian vault synced with implementation.
