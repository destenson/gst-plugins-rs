// Simple test file to verify API client functionality
import { StreamManagerAPI } from './index.ts';
import { createMockAPI } from './mock.ts';

// Test real API client initialization
const realAPI = new StreamManagerAPI({
  baseURL: 'http://localhost:3000',
  token: 'test-token'
});

console.log('Real API client created:', !!realAPI);
console.log('API services available:', {
  streams: !!realAPI.streams,
  config: !!realAPI.config,
  metrics: !!realAPI.metrics,
  health: !!realAPI.health
});

// Test mock API client
const mockAPI = createMockAPI();
console.log('Mock API client created:', !!mockAPI);

// Test async functions with mock
async function testMockAPI() {
  try {
    // Test health check
    const health = await mockAPI.health.check();
    console.log('Health check:', health);

    // Test stream list
    const streams = await mockAPI.streams.list();
    console.log('Streams count:', streams.streams.length);

    // Test system status
    const status = await mockAPI.health.getStatus();
    console.log('System status:', status);

    console.log('✅ All mock API tests passed');
  } catch (error) {
    console.error('❌ Mock API test failed:', error);
  }
}

// Export for testing
export { realAPI, mockAPI, testMockAPI };

// Run tests if this file is executed directly
if (import.meta.main) {
  testMockAPI();
}