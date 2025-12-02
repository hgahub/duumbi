---
trigger: always_on
glob: "**/*.{ts,tsx,js,jsx,css,scss}"
description: "Duumbi workspace code style and best practices"
---

# Duumbi Code Style Guide

## 1. TypeScript

- **Strict Mode**: Always adhere to `strict: true`.
- **No `any`**: Avoid `any` at all costs. Use `unknown` with type guards if the type is truly dynamic.
- **Interfaces vs Types**:
  - Use `interface` for defining object shapes and public API contracts.
  - Use `type` for defining unions, intersections, and primitives.
  - **Naming**: PascalCase for interfaces and types. Do NOT use `I` prefix (e.g., `User`, not `IUser`).
- **Explicit Returns**: Always define explicit return types for exported functions and component props.

## 2. React

- **Functional Components**: Use functional components with hooks. Class components are forbidden.
- **Component Structure**:
  ```tsx
  // 1. Imports
  import { useState } from 'react';
  import { SomeComponent } from './some-component';

  // 2. Types/Interfaces
  interface MyComponentProps {
    title: string;
  }

  // 3. Component Definition
  export function MyComponent({ title }: MyComponentProps) {
    // 4. Hooks
    const [isOpen, setIsOpen] = useState(false);

    // 5. Render
    return (
      <div className="p-4">
        <h1>{title}</h1>
      </div>
    );
  }
  ```
- **Hooks**: Extract complex logic into custom hooks (`useMyLogic.ts`).
- **Props**: Destructure props in the function signature.

## 3. Styling (Tailwind CSS)

- **Utility-First**: Use utility classes directly in JSX.
- **No `@apply`**: Avoid using `@apply` in CSS files unless creating a reusable component class that cannot be a React component.
- **Consistency**: Use the design tokens defined in `tailwind.config.js` (colors, spacing, etc.).
- **Ordering**: Follow a logical order for classes (Layout -> Box Model -> Typography -> Visuals -> Misc). *Tip: Use the Prettier plugin if available.*

## 4. Testing

- **Unit Tests**:
  - Place test files next to the source file: `MyComponent.tsx` -> `MyComponent.spec.tsx`.
  - Use `describe` blocks to group tests by function/component.
  - Use `it` or `test` for individual test cases.
- **Testing Library**: Use `@testing-library/react` for component testing. Focus on user interactions (accessibility roles, text content) rather than implementation details.
