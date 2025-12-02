# ts-models

**Shared TypeScript Interfaces for Duumbi Domain Entities**

This library contains the core data models used across the application to ensure type safety and consistency.

## Key Models

- **Property**: Represents a real estate property (physical attributes).
- **Listing**: Represents a property listing (price, description, active status).
- **User**: Represents a registered user of the platform.

## Usage

```typescript
import { Listing, Property } from '@duumbi/ts-models';

const myListing: Listing = {
  // ...
};
```
