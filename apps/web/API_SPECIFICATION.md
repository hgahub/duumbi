# Duumbi API Specification

This document outlines the backend API endpoints required for the Duumbi Chat application.

## Base URL

```
https://api.duumbi.com/v1
```

## Authentication

All authenticated endpoints require a Bearer token in the Authorization header:

```
Authorization: Bearer <token>
```

---

## 1. Authentication & User Management

### POST /auth/register

Register a new user account.

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "securePassword123",
  "name": "John Doe"
}
```

**Response (201 Created):**

```json
{
  "user": {
    "id": "user_123",
    "email": "user@example.com",
    "name": "John Doe",
    "avatar": "https://cdn.duumbi.com/avatars/default.png",
    "createdAt": "2025-10-28T12:00:00Z"
  },
  "token": ""
}
```

### POST /auth/login

Authenticate existing user.

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "securePassword123"
}
```

**Response (200 OK):**

```json
{
  "user": {
    "id": "user_123",
    "email": "user@example.com",
    "name": "John Doe",
    "avatar": "https://cdn.duumbi.com/avatars/user_123.png"
  },
  "token": ""
}
```

### GET /auth/me

Get current authenticated user profile.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "id": "user_123",
  "email": "user@example.com",
  "name": "John Doe",
  "avatar": "https://cdn.duumbi.com/avatars/user_123.png",
  "createdAt": "2025-10-28T12:00:00Z"
}
```

### PUT /users/profile

Update user profile information.

**Headers:** Authorization required

**Request Body:**

```json
{
  "name": "Jane Doe",
  "avatar": "https://cdn.duumbi.com/avatars/new_avatar.png"
}
```

**Response (200 OK):**

```json
{
  "id": "user_123",
  "email": "user@example.com",
  "name": "Jane Doe",
  "avatar": "https://cdn.duumbi.com/avatars/new_avatar.png",
  "updatedAt": "2025-10-28T13:00:00Z"
}
```

---

## 2. Chat Management

### GET /chats

Get list of user's chat conversations.

**Headers:** Authorization required

**Query Parameters:**

- `limit` (optional): Number of chats to return (default: 20)
- `offset` (optional): Pagination offset (default: 0)

**Response (200 OK):**

```json
{
  "chats": [
    {
      "id": "chat_456",
      "title": "Project Plan Brainstorm",
      "preview": "Let's discuss the project timeline...",
      "createdAt": "2025-10-27T10:00:00Z",
      "updatedAt": "2025-10-27T15:30:00Z",
      "messageCount": 12
    },
    {
      "id": "chat_457",
      "title": "Marketing Email Draft",
      "preview": "Can you help me write...",
      "createdAt": "2025-10-26T14:00:00Z",
      "updatedAt": "2025-10-26T16:00:00Z",
      "messageCount": 8
    }
  ],
  "total": 15,
  "hasMore": true
}
```

### POST /chats

Create a new chat conversation.

**Headers:** Authorization required

**Request Body:**

```json
{
  "title": "New Chat Session",
  "initialMessage": "Hello, I need help with..."
}
```

**Response (201 Created):**

```json
{
  "id": "chat_458",
  "title": "New Chat Session",
  "createdAt": "2025-10-28T12:00:00Z",
  "messages": [
    {
      "id": "msg_789",
      "role": "user",
      "content": "Hello, I need help with...",
      "timestamp": "2025-10-28T12:00:00Z"
    }
  ]
}
```

### GET /chats/:chatId

Get specific chat conversation with messages.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "id": "chat_456",
  "title": "Project Plan Brainstorm",
  "createdAt": "2025-10-27T10:00:00Z",
  "updatedAt": "2025-10-27T15:30:00Z",
  "messages": [
    {
      "id": "msg_101",
      "role": "user",
      "content": "Let's discuss the project timeline",
      "timestamp": "2025-10-27T10:00:00Z"
    },
    {
      "id": "msg_102",
      "role": "assistant",
      "content": "I'd be happy to help you plan the timeline...",
      "timestamp": "2025-10-27T10:00:15Z"
    }
  ]
}
```

### POST /chats/:chatId/messages

Send a new message in a chat.

**Headers:** Authorization required

**Request Body:**

```json
{
  "content": "What are the next steps?"
}
```

**Response (201 Created):**

```json
{
  "id": "msg_103",
  "chatId": "chat_456",
  "role": "user",
  "content": "What are the next steps?",
  "timestamp": "2025-10-27T15:30:00Z"
}
```

### DELETE /chats/:chatId

Delete a chat conversation.

**Headers:** Authorization required

**Response (204 No Content)**

---

## 3. Listings Management

### GET /listings

Get user's property listings.

**Headers:** Authorization required

**Query Parameters:**

- `status` (optional): Filter by status (draft, published, archived)
- `limit` (optional): Number of listings to return (default: 20)
- `offset` (optional): Pagination offset (default: 0)

**Response (200 OK):**

```json
{
  "listings": [
    {
      "id": "listing_789",
      "title": "Beautiful 3BR Home in Downtown",
      "description": "Spacious home with modern amenities...",
      "price": 450000,
      "status": "published",
      "address": {
        "street": "123 Main St",
        "city": "Austin",
        "state": "TX",
        "zipCode": "78701"
      },
      "images": ["https://cdn.duumbi.com/listings/listing_789/img1.jpg"],
      "createdAt": "2025-10-20T10:00:00Z",
      "updatedAt": "2025-10-25T14:00:00Z"
    }
  ],
  "total": 5,
  "hasMore": false
}
```

### POST /listings

Create a new property listing.

**Headers:** Authorization required

**Request Body:**

```json
{
  "title": "Beautiful 3BR Home in Downtown",
  "description": "Spacious home with modern amenities...",
  "price": 450000,
  "address": {
    "street": "123 Main St",
    "city": "Austin",
    "state": "TX",
    "zipCode": "78701"
  },
  "propertyType": "house",
  "bedrooms": 3,
  "bathrooms": 2,
  "squareFeet": 2000,
  "images": ["https://cdn.duumbi.com/temp/img1.jpg"]
}
```

**Response (201 Created):**

```json
{
  "id": "listing_790",
  "title": "Beautiful 3BR Home in Downtown",
  "status": "draft",
  "createdAt": "2025-10-28T12:00:00Z",
  "url": "/listings/listing_790"
}
```

### GET /listings/:listingId

Get specific listing details.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "id": "listing_789",
  "title": "Beautiful 3BR Home in Downtown",
  "description": "Spacious home with modern amenities...",
  "price": 450000,
  "status": "published",
  "address": {
    "street": "123 Main St",
    "city": "Austin",
    "state": "TX",
    "zipCode": "78701"
  },
  "propertyType": "house",
  "bedrooms": 3,
  "bathrooms": 2,
  "squareFeet": 2000,
  "images": [
    "https://cdn.duumbi.com/listings/listing_789/img1.jpg",
    "https://cdn.duumbi.com/listings/listing_789/img2.jpg"
  ],
  "createdAt": "2025-10-20T10:00:00Z",
  "updatedAt": "2025-10-25T14:00:00Z"
}
```

### PUT /listings/:listingId

Update a property listing.

**Headers:** Authorization required

**Request Body:** Same as POST /listings

**Response (200 OK):**

```json
{
  "id": "listing_789",
  "title": "Updated Title",
  "status": "published",
  "updatedAt": "2025-10-28T12:30:00Z"
}
```

### DELETE /listings/:listingId

Delete a property listing.

**Headers:** Authorization required

**Response (204 No Content)**

### POST /listings/:listingId/publish

Publish a draft listing.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "id": "listing_789",
  "status": "published",
  "publishedAt": "2025-10-28T12:00:00Z"
}
```

---

## 4. Notifications

### GET /notifications

Get user's notifications.

**Headers:** Authorization required

**Query Parameters:**

- `unreadOnly` (optional): Return only unread notifications (default: false)
- `limit` (optional): Number of notifications to return (default: 20)

**Response (200 OK):**

```json
{
  "notifications": [
    {
      "id": "notif_123",
      "type": "listing_inquiry",
      "title": "New inquiry on your listing",
      "message": "Someone is interested in your property at 123 Main St",
      "isRead": false,
      "createdAt": "2025-10-28T11:00:00Z",
      "data": {
        "listingId": "listing_789"
      }
    },
    {
      "id": "notif_124",
      "type": "chat_message",
      "title": "New message",
      "message": "You have a new message in Project Plan Brainstorm",
      "isRead": true,
      "createdAt": "2025-10-27T15:00:00Z",
      "data": {
        "chatId": "chat_456"
      }
    }
  ],
  "unreadCount": 3
}
```

### PUT /notifications/:notificationId/read

Mark a notification as read.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "id": "notif_123",
  "isRead": true,
  "readAt": "2025-10-28T12:00:00Z"
}
```

### PUT /notifications/read-all

Mark all notifications as read.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "markedAsRead": 5
}
```

---

## 5. Settings

### GET /settings

Get user settings and preferences.

**Headers:** Authorization required

**Response (200 OK):**

```json
{
  "notifications": {
    "email": true,
    "push": true,
    "chatMessages": true,
    "listingInquiries": true
  },
  "preferences": {
    "theme": "dark",
    "language": "en"
  }
}
```

### PUT /settings

Update user settings.

**Headers:** Authorization required

**Request Body:**

```json
{
  "notifications": {
    "email": false,
    "push": true
  },
  "preferences": {
    "theme": "light"
  }
}
```

**Response (200 OK):**

```json
{
  "notifications": {
    "email": false,
    "push": true,
    "chatMessages": true,
    "listingInquiries": true
  },
  "preferences": {
    "theme": "light",
    "language": "en"
  },
  "updatedAt": "2025-10-28T12:00:00Z"
}
```

---

## Error Responses

All endpoints may return the following error responses:

### 400 Bad Request

```json
{
  "error": "Bad Request",
  "message": "Invalid input data",
  "details": {
    "field": "email",
    "issue": "Invalid email format"
  }
}
```

### 401 Unauthorized

```json
{
  "error": "Unauthorized",
  "message": "Authentication required"
}
```

### 403 Forbidden

```json
{
  "error": "Forbidden",
  "message": "You don't have permission to access this resource"
}
```

### 404 Not Found

```json
{
  "error": "Not Found",
  "message": "Resource not found"
}
```

### 500 Internal Server Error

```json
{
  "error": "Internal Server Error",
  "message": "An unexpected error occurred"
}
```

---

## WebSocket Events (Real-time Features)

### Connection

```
wss://api.duumbi.com/ws?token=<auth_token>
```

### Events

#### chat:message

Receive new chat message in real-time.

```json
{
  "event": "chat:message",
  "data": {
    "chatId": "chat_456",
    "message": {
      "id": "msg_105",
      "role": "assistant",
      "content": "Here's my response...",
      "timestamp": "2025-10-28T12:00:00Z"
    }
  }
}
```

#### notification:new

Receive new notification.

```json
{
  "event": "notification:new",
  "data": {
    "id": "notif_125",
    "type": "listing_inquiry",
    "title": "New inquiry",
    "message": "Someone is interested in your property",
    "createdAt": "2025-10-28T12:00:00Z"
  }
}
```

---

## Rate Limiting

- 100 requests per minute per user for standard endpoints
- 1000 requests per minute per user for WebSocket messages
- Rate limit headers included in responses:
  - `X-RateLimit-Limit`: Maximum requests allowed
  - `X-RateLimit-Remaining`: Requests remaining in current window
  - `X-RateLimit-Reset`: Time when limit resets (Unix timestamp)

---

## Notes

1. All timestamps are in ISO 8601 format (UTC)
2. All endpoints return JSON unless otherwise specified
3. File uploads for images should use multipart/form-data
4. Maximum image size: 10MB per image
5. Maximum of 20 images per listing
