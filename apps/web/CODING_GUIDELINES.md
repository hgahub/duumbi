# Duumbi Web Frontend Coding Guidelines (`apps/web`)

Version: 1.0
Date: 2025-10-28

## 1. Introduction and Core Principles 🎯

This document outlines the development guidelines for the Duumbi platform's React frontend application (`apps/web`). Our goal is to write **understandable, modular, well-documented, testable, and modern** code, adhering to the principles of **Clean Code**. These guidelines help maintain code quality and facilitate collaboration with AI assistants (GitHub Copilot, Claude, Gemini, etc.).

**Core Principles:**

- **Readability:** Code should be easy for other developers (and AI) to understand.
- **Modularity:** Small, reusable components and functions with clear responsibilities.
- **Documentation:** Comments where necessary (explaining the _why_), JSDoc for more complex parts.
- **Testability:** Code should be written in a way that is easy to test (unit, integration).
- **Modernity:** Utilize features of modern JavaScript (ES2020+), TypeScript, and React (19+).
- **Consistency:** Follow unified style and naming conventions.

---

## 2. Technology Stack and Tools 🛠️

- **Framework:** React 18+ (will migrate to React 19 when stable and ecosystem matures)
- **Language:** TypeScript 5.2+ (Strict mode enabled)
- **Build Tool:** Vite
- **Styling:** Tailwind CSS 3.4+, CSS Modules (for complex cases if needed)
- **UI Components:** shadcn/ui, Radix UI primitives
- **Icons:** Lucide React
- **Styling Utilities:** Class Variance Authority (CVA), `clsx`, `tailwind-merge`
- **API Communication:** tRPC client
- **Linting/Formatting:** ESLint, Prettier (configured in project root, with specific rules in `apps/web/eslint.config.js`)

---

## 3. Component Design ✨

- **Functional Components:** Exclusively use functional components with React Hooks (e.g., `useState`, `useEffect`, `useContext`). Do not use class-based components.
- **Size and Responsibility:** Keep components small and focused (Single Responsibility Principle - SRP). A component should do one thing well. If a component grows too large, refactor it into smaller ones.
- **Props:**
  - Use TypeScript `interface`s or `type`s to define component props.
  - Use destructuring within the props object.
  - Provide `defaultProps` or use default values during destructuring where possible and meaningful.
  - Avoid passing too many props; consider using the `children` prop or the Context API.
- **State Management:**
  - For local state, use the `useState` and `useReducer` hooks.
  - For global or shared state, use the React Context API (for simpler cases) or a dedicated state management library (e.g., Zustand, if project complexity warrants it).
- **Composition:** Prefer composition over inheritance. Use the `children` prop and Render Props pattern where appropriate.

---

## 4. TypeScript Usage 🟦

- **Strict Mode:** The project runs with `strict: true` enabled. Leverage TypeScript's type safety.
- **Avoid `any`:** Use the `any` type only as a last resort. Prefer more specific types, `unknown`, or generics instead.
- **Type Definitions:** Define `interface`s or `type`s for props, API responses, and any complex data structures. Place these within the component file or in separate `types.ts` files (e.g., within the component's folder or a shared `types` folder).
- **Utility Types:** Utilize built-in utility types (e.g., `Partial`, `Pick`, `Omit`, `ReturnType`) for type manipulation.

---

## 5. Styling (Tailwind CSS & shadcn/ui) 🎨

- **shadcn/ui:** Follow shadcn/ui conventions for creating and using components (CLI, composition).
- **Tailwind Utility Classes:** Use Tailwind utility classes directly in JSX. Avoid using `@apply` in CSS files unless there's a compelling reason (e.g., global styles in `index.css`).
- **Class Management:** Use `clsx` (or similar) for conditional classes and `tailwind-merge` for merging classes without conflicts.
- **Variants:** Use the CVA (Class Variance Authority) library for component variants.
- **Configuration:** Define project-specific colors (e.g., `higashi-kashmirblue`, `higashi-concrete`), fonts, and other customizations in `tailwind.config.js`.

---

## 6. Naming Conventions 🏷️

- **Components:** `PascalCase` (e.g., `UserProfileCard.tsx`, `SettingsModal.tsx`). Filename should match the component name.
- **Functions, Variables, Hooks:** `camelCase` (e.g., `fetchUserData`, `userName`, `useAuth`).
- **Interfaces, Types:** `PascalCase` (e.g., `interface UserProfileProps`, `type Status = 'loading' | 'success' | 'error'`).
- **Constants:** `UPPER_SNAKE_CASE` (e.g., `const API_BASE_URL = '...'`).
- **Boolean Variables/Props:** Use prefixes like `is`, `has`, `should`, `can` (e.g., `isOpen`, `hasNotifications`, `shouldUpdate`).
- **CSS Classes:** Follow Tailwind conventions.

---

## 7. Documentation and Comments ✍️

- **JSDoc:** Use JSDoc comments (`/** ... */`) to document components, hooks, complex functions, and types. Explain the purpose, parameters, and return values.
- **Inline Comments:** Use inline comments (`//`) only when the code itself is not self-explanatory. The comment should explain the _why_, not the _what_.
- **TODO/FIXME:** Use these markers to indicate future tasks or issues needing attention, but strive to resolve them promptly.
- **Up-to-date:** Keep comments and documentation current with the code.

---

## 8. Testing ✅

- **Tools:** Use modern testing tools like Vitest and React Testing Library (RTL).
- **Unit Tests:** Write unit tests for utility functions (`utils`), hooks (`hooks`), and component parts containing complex logic.
- **Integration Tests:** Test how components work together and user flows (e.g., filling and submitting a form). Focus on simulating user interactions (RTL philosophy).
- **Coverage:** Aim for reasonable test coverage, especially for critical business logic and UI interactions. The goal is confidence in the code's correctness, not necessarily 100% coverage.

---

## 9. API Communication (tRPC) ↔️

- **Hooks:** Use the tRPC-generated client-side hooks (e.g., `api.useQuery`, `api.useMutation`) for fetching and modifying data.
- **Type Safety:** Leverage the end-to-end type safety provided by tRPC.
- **State Handling:** Explicitly handle loading (`isLoading`), error (`isError`, `error`), and success (`isSuccess`, `data`) states of queries/mutations in the UI (e.g., show loading indicators, display error messages).

---

## 10. Code Structure 📁

- **Logical Organization:** Use a logical directory structure within the `src` folder, e.g.:
  - `components/`: Reusable UI components (can have subdirs like `ui/` for basic elements, `features/` for more specific ones).
  - `hooks/`: Custom React hooks.
  - `utils/`: General utility functions.
  - `pages/` or `routes/`: Components representing individual pages/views.
  - `contexts/`: React Context definitions.
  - `types/`: Global TypeScript type definitions (if needed).
  - `assets/`: Images, icons, etc.
- **Colocation:** Keep files that belong together close to each other (e.g., a component, its styles, types, and tests in the same folder).

---

## 11. Clean Code Principles 🧹

- **DRY (Don't Repeat Yourself):** Avoid code duplication. Extract common logic into reusable functions, hooks, or components.
- **KISS (Keep It Simple, Stupid):** Strive for simple, easy-to-understand solutions. Don't overcomplicate things.
- **YAGNI (You Ain't Gonna Need It):** Implement only what is actually needed. Don't add features or abstractions prematurely.
- **Meaningful Names:** Use descriptive names for variables, functions, components. The name should reflect the purpose.
- **Small Functions/Components:** Keep functions and components short, with a single responsibility.
- **Avoid Magic Values:** Use named constants instead of string literals or numbers with special meanings.
- **Formatting:** Adhere to the consistent code formatting enforced by Prettier. ESLint helps catch potential errors and style issues.

---

## 12. Accessibility (a11y) ♿

- **Semantic HTML:** Use appropriate HTML elements to structure content (e.g., `<nav>`, `<main>`, `<button>`, `<article>`).
- **Keyboard Navigation:** Ensure all interactive elements are reachable and operable via keyboard only. Pay attention to focus order and visibility.
- **ARIA Attributes:** Use ARIA attributes where semantic HTML is insufficient to convey role, state, or property (e.g., `aria-label`, `aria-hidden`, `role`). See examples in `TopMenu.tsx`.
- **Contrast:** Check color contrast between text and background to meet WCAG recommendations.
- **Images:** All images must have an `alt` attribute (even if empty for decorative images).

---

## 13. Performance 🚀

- **Memoization:** Use the `useMemo` hook for expensive calculations and `React.memo` to prevent unnecessary re-renders of components. Use them only where genuinely needed and measurable improvement is observed.
- **Bundle Size:** Be mindful of dependency sizes. Use code splitting (e.g., with `React.lazy`) and lazy loading to reduce initial load time.
- **List Optimization:** When rendering large lists, use virtualization (e.g., `react-window` or `react-virtualized`) and provide unique `key` props.

---

## 14. Error Handling & Error Boundaries 🛡️

- **Error Boundaries:** Implement React Error Boundaries to catch and handle JavaScript errors in the component tree. This prevents the entire app from crashing due to errors in isolated components.
  - Create a reusable `ErrorBoundary` component in `src/components/ErrorBoundary.tsx`.
  - Wrap critical sections of the app (e.g., entire routes, complex features) with the Error Boundary.
  - Error Boundaries should:
    - Display a user-friendly fallback UI when an error occurs.
    - Log errors to an error tracking service (e.g., Sentry, LogRocket) in production.
    - Provide a way to recover (e.g., "Try again" button that resets the error state).

- **Error Boundary Placement:**
  - **Top-level:** Wrap the entire app to catch catastrophic errors.
  - **Route-level:** Wrap individual routes/pages to isolate errors to specific views.
  - **Feature-level:** Wrap complex, critical features (e.g., chat interface, listing form) to prevent isolated failures from affecting the whole page.

- **Error Types:**
  - **Component Errors:** Caught by Error Boundaries (rendering errors, lifecycle errors).
  - **Async Errors:** Handle explicitly in `try-catch` blocks, Promise `.catch()`, or error states from hooks (e.g., tRPC's `isError`).
  - **Event Handler Errors:** Won't be caught by Error Boundaries; wrap in `try-catch` or handle at the call site.

- **User Communication:**
  - Display clear, actionable error messages to users.
  - Avoid technical jargon; explain what went wrong in simple terms.
  - Provide recovery options where possible (e.g., retry, go back, contact support).

- **Error Logging:**
  - In development: Log errors to console with full stack traces.
  - In production: Send errors to a monitoring service with context (user ID, route, browser info).
  - Never expose sensitive information in error messages or logs.

- **Graceful Degradation:**
  - When a non-critical feature fails, show a fallback UI or hide it gracefully.
  - The app should remain functional even if individual features encounter errors.

**Example Error Boundary Implementation:**

```tsx
// src/components/ErrorBoundary.tsx
import { Component, ReactNode } from 'react'

interface Props {
  children: ReactNode
  fallback?: ReactNode
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Log to error tracking service
    console.error('Error caught by boundary:', error, errorInfo)
    this.props.onError?.(error, errorInfo)
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      return (
        this.props.fallback || (
          <div className="flex min-h-screen items-center justify-center p-4">
            <div className="text-center">
              <h2 className="text-2xl font-bold mb-4">Something went wrong</h2>
              <p className="text-gray-600 mb-6">
                We're sorry for the inconvenience. Please try again.
              </p>
              <button
                onClick={this.handleReset}
                className="px-4 py-2 bg-higashi-kashmirblue-200 text-white rounded-lg hover:bg-higashi-kashmirblue-300"
              >
                Try Again
              </button>
            </div>
          </div>
        )
      )
    }

    return this.props.children
  }
}
```

**Usage Examples:**

```tsx
// Wrap entire app
<ErrorBoundary>
  <App />
</ErrorBoundary>

// Wrap specific route
<ErrorBoundary fallback={<RouteErrorFallback />}>
  <ChatPage />
</ErrorBoundary>

// Wrap critical feature with custom error handling
<ErrorBoundary onError={(error) => logToSentry(error)}>
  <ListingForm />
</ErrorBoundary>
```

---
