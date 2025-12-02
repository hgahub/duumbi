# @duumbi/ui-components

**Duumbi Design System & UI Library**

This library houses the reusable UI components and design system tokens for the Duumbi platform. It is built with **React** and **Tailwind CSS**.

## Design System

The design system is enforced via the `tailwind.config.js` presets and the `ThemeProvider`.

## Components

Commonly used components include:

- **Button**: Standard interactive buttons.
- **Card**: Container for content (e.g., Listing Card).
- **Input / Select**: Form elements.
- **Typography**: Headings and text styles.

## Usage

```tsx
import { Button, Card } from '@duumbi/ui-components';

export function MyPage() {
  return (
    <Card>
      <h1>Hello</h1>
      <Button variant="primary">Click Me</Button>
    </Card>
  );
}
```
