---
type: 'agent_requested'
description: 'Duumbi workspace code style and best practices'
---

# Duumbi Code Style Guide

## 1. TypeScript

- **Strict Mode**: Always adhere to `strict: true` with additional compiler flags:
  - `noUnusedLocals: true`
  - `noUnusedParameters: true`
  - `noFallthroughCasesInSwitch: true`
  - `forceConsistentCasingInFileNames: true`
- **No `any`**: Avoid `any` at all costs. Use `unknown` with type guards if the type is truly dynamic.
- **Interfaces vs Types**:
  - Use `interface` for defining object shapes and public API contracts.
  - Use `type` for defining unions, intersections, and primitives.
  - **Naming**: PascalCase for interfaces and types. Do NOT use `I` prefix (e.g., `User`, not `IUser`).
- **Explicit Returns**: Always define explicit return types for exported functions and component props.
- **Type Imports**: Use explicit type imports from external libraries (e.g., `import { User, Session } from '@supabase/supabase-js'`).

## 2. React

- **Functional Components**: Use functional components with hooks. Class components are forbidden.
- **Component Structure**:

  ```tsx
  // 1. Imports - External libraries first, then internal
  import { useState, useEffect } from 'react';
  import { useTranslation } from 'react-i18next';
  import { SomeComponent } from './some-component';

  // 2. Types/Interfaces - Define props interface
  interface MyComponentProps {
    title: string;
    onClose?: () => void;
  }

  // 3. Component Definition - Use named export with 'export default function'
  export default function MyComponent({ title, onClose }: MyComponentProps) {
    // 4. Hooks - Context hooks first, then state, then effects
    const { t } = useTranslation();
    const [isOpen, setIsOpen] = useState(false);

    useEffect(() => {
      // Effect logic
    }, []);

    // 5. Event Handlers
    const handleClick = () => {
      setIsOpen(!isOpen);
    };

    // 6. Render
    return (
      <div className="p-4">
        <h1>{title}</h1>
      </div>
    );
  }
  ```

- **Component Naming**: 
  - PascalCase for component files: `AgentQuery.tsx`, `TopMenu.tsx`
  - Component function name must match file name
  - Use `export default function ComponentName()` pattern
- **Hooks**: 
  - Extract complex logic into custom hooks (`useAuth.ts`, `useTheme.ts`)
  - Always call hooks at the top level, never inside conditions
- **Props**: 
  - Destructure props in the function signature
  - Mark optional props with `?` in interface definition
  - Provide default values using destructuring defaults when appropriate
- **Event Handlers**:
  - Prefix with `handle` for local handlers: `handleClick`, `handleSubmit`
  - Prefix with `on` for prop callbacks: `onClick`, `onNavigate`
  - Use proper TypeScript event types: `React.FormEvent`, `React.MouseEvent`, etc.

## 3. Context & State Management

- **Context Pattern**:
  - Create context with undefined default: `createContext<TypeName | undefined>(undefined)`
  - Export custom hook for consuming context: `export function useContextName()`
  - Throw error in hook if context is undefined
  - Example: See `AuthContext.tsx` for reference implementation
- **State Management**:
  - Keep state as local as possible
  - Use `useState` for component-level state
  - Use Context API for theme, auth, and app-wide settings
  - Type all state with explicit types or interfaces

## 4. Styling (Tailwind CSS)

- **Utility-First**: Use utility classes directly in JSX.
- **Custom Colors**: Use project color palette from `tailwind.config.js`:
  - `higashi-concrete-*`: Light theme backgrounds (100-900)
  - `higashi-kashmirblue-*`: Dark theme backgrounds and primary colors (100-900)
- **Dark Mode**: Always provide dark mode variants using `dark:` prefix:
  ```tsx
  <div className="bg-white dark:bg-higashi-kashmirblue-800 text-gray-900 dark:text-white">
  ```
- **Transitions**: Add smooth transitions for theme switching:
  ```tsx
  <div className="transition-colors duration-200">
  ```
- **Responsive Design**: Use mobile-first approach with responsive prefixes:
  - `md:` for tablet (768px)
  - `lg:` for desktop (1024px)
  - Hide on mobile: `hidden md:block`
  - Mobile only: `md:hidden`
- **Class Organization**: Order classes logically:
  1. Layout (flex, grid, block)
  2. Positioning (relative, absolute, fixed)
  3. Display & Sizing (w-full, h-screen, max-w-*)
  4. Spacing (p-4, m-2, space-x-4)
  5. Typography (text-sm, font-bold)
  6. Colors (bg-*, text-*, border-*)
  7. Effects (shadow-lg, rounded-md, transition-*)
  8. Interactive (hover:*, focus:*, dark:*)
- **No `@apply`**: Avoid using `@apply` in CSS files. Use inline Tailwind classes or extract to React components.
- **Custom CSS**: Only use custom CSS for animations or complex transitions (see `index.css` for sidebar transitions).

## 5. Internationalization (i18n)

- **Always Use Translation Keys**: Never hardcode user-facing strings:
  ```tsx
  // ❌ Bad
  <button>Sign in</button>
  
  // ✅ Good
  <button>{t('Sign in')}</button>
  ```
- **Translation Hook**: Import and use `useTranslation` hook from `react-i18next`:
  ```tsx
  const { t } = useTranslation();
  ```
- **Translation Keys**: 
  - Use descriptive English text as keys
  - Keep keys consistent across the codebase
  - Add new keys to all language files in `i18n.ts`
- **Supported Languages**: en, de, hu, es, pl, it

## 6. Icons & Assets

- **Icon Library**: Use `@heroicons/react/24/outline` for outline icons:
  ```tsx
  import { UserIcon, CogIcon } from '@heroicons/react/24/outline';
  ```
- **Icon Sizing**: Use consistent sizing with Tailwind classes:
  - Small icons: `h-4 w-4`
  - Medium icons: `h-5 w-5` or `h-6 w-6`
- **Icon Accessibility**: Always include `aria-hidden="true"` for decorative icons
- **Alternative**: For custom icons, use inline SVG components (see `AgentQuery.tsx` for examples)

## 7. Forms & Validation

- **Form Elements**:
  - Use semantic HTML: `<form>`, `<label>`, `<input>`
  - Include proper `htmlFor` on labels matching input `id`
  - Add `autoComplete` attributes for better UX
  - Use `type` attribute correctly (email, password, text)
- **Form State**:
  - Use individual `useState` for simple forms
  - Add loading state for async operations
  - Display error/success messages with proper styling
- **Form Handlers**:
  - Prevent default: `e.preventDefault()` in submit handlers
  - Type form events: `React.FormEvent`
  - Handle errors with try-catch and typed error messages

## 8. File Organization

- **Component Files**: Place in `src/components/`
  - One component per file
  - File name matches component name
- **Pages**: Place in `src/pages/`
- **Contexts**: Place in `src/context/`
- **Utilities**: Place in `src/lib/`
- **Shared UI**: Extract reusable components to `libs/ui/src/lib/`
- **Path Aliases**: Use TypeScript path aliases for cleaner imports:
  ```tsx
  import { ThemeProvider } from '@duumbi/ui-components';
  ```

## 9. Testing

- **Unit Tests**:
  - Place test files next to the source file: `MyComponent.tsx` -> `MyComponent.spec.tsx`
  - Use `describe` blocks to group tests by function/component
  - Use `it` or `test` for individual test cases
- **Testing Library**: Use `@testing-library/react` for component testing
  - Focus on user interactions (accessibility roles, text content)
  - Avoid testing implementation details
  - Query by role, label text, or test IDs

## 10. Accessibility

- **Semantic HTML**: Use proper semantic elements (`<button>`, `<nav>`, `<main>`, `<header>`)
- **ARIA Attributes**:
  - Add `aria-label` or `title` for icon-only buttons
  - Use `aria-hidden="true"` for decorative elements
- **Keyboard Navigation**: Ensure all interactive elements are keyboard accessible
- **Focus Management**: Use proper focus indicators and manage focus for modals/dropdowns

## 11. Performance

- **React Best Practices**:
  - Avoid inline function definitions in render when possible
  - Use `useCallback` for functions passed to child components
  - Use `useMemo` for expensive computations
- **Bundle Size**:
  - Import only what you need from libraries
  - Use tree-shakeable imports: `import { useState } from 'react'`
- **Images**: 
  - Use appropriate image formats (SVG for logos)
  - Provide `alt` text for all images
  - Use responsive images when appropriate

## 12. Code Quality

- **ESLint**: Run `nx lint <project>` before committing
- **TypeScript**: Run `nx run-many --target=typecheck` to catch type errors
- **Formatting**: Follow consistent formatting (spaces, line breaks)
- **Comments**: 
  - Avoid obvious comments
  - Document complex logic or non-obvious decisions
  - Use JSDoc for exported functions/components
- **Console Logs**: Remove `console.log` before committing (use proper logging for production)
