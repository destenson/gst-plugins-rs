# PRP-35: Authentication UI and Flow

## Overview
Implement authentication UI components and flow for securing the Stream Manager web interface.

## Context
- Backend uses token-based authentication
- Auth configuration in src/api/middleware.rs
- Need login page and session management
- Must handle token storage and refresh
- Support for both development (no auth) and production modes

## Requirements
1. Login page with form validation
2. Secure token storage
3. Authentication context and hooks
4. Protected route components
5. Session timeout handling
6. Logout functionality
7. Remember me option
8. Password visibility toggle
9. Error message display

## Implementation Tasks
1. Create login page component with form
2. Implement form validation (email/username and password)
3. Create AuthContext for authentication state
4. Implement secure token storage (httpOnly cookies preferred)
5. Create ProtectedRoute wrapper component
6. Add authentication check on app startup
7. Implement logout with token cleanup
8. Add session timeout warning modal
9. Create useAuth hook for components
10. Add "Remember Me" with persistent storage
11. Implement password field with show/hide toggle
12. Add loading states during authentication
13. Create development mode bypass

## UI Components
- LoginPage (full-screen login form)
- ProtectedRoute (wrapper for authenticated routes)
- SessionTimeoutModal (warning before logout)
- UserMenu (dropdown with user info and logout)

## Resources
- React Hook Form: https://react-hook-form.com/
- JWT best practices: https://hasura.io/blog/best-practices-of-using-jwt-with-graphql/
- Secure token storage: https://www.rdegges.com/2018/please-stop-using-local-storage/
- Protected routes pattern: https://ui.dev/react-router-protected-routes-authentication

## Validation Gates
```bash
cd apps/stream-manager/web-ui

# Run development server
deno run dev

# Test authentication flow:
# 1. Should redirect to login when not authenticated
# 2. Login form should validate inputs
# 3. Successful login should redirect to dashboard
# 4. Token should persist across page refresh
# 5. Logout should clear token and redirect to login

# Run tests
deno test

# Type checking
deno run type-check
```

## Success Criteria
- Login page renders at /login
- Form validates email and password
- Successful login stores token securely
- Protected routes redirect to login when not authenticated
- Logout clears authentication state
- Session persists across page refresh (if Remember Me)
- Error messages display for failed login
- Development mode can bypass authentication

## Dependencies
- PRP-32 (Base layout) must be completed
- PRP-33 (API client) must be completed

## Security Considerations
- Never store tokens in localStorage (XSS vulnerable)
- Use httpOnly cookies if possible
- Implement CSRF protection
- Add rate limiting for login attempts
- Clear sensitive data on logout

## Estimated Effort
3 hours

## Confidence Score
8/10 - Standard authentication pattern with security considerations
