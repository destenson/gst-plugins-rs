#![allow(unused)]
// Buffer pool implementation for efficient memory management
//
// This module provides buffer pooling to reduce allocations in hot paths
// and improve performance for high-throughput RTSP streams

use bytes::{Bytes, BytesMut};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// Size buckets for common packet sizes based on MTU
const BUCKET_SIZES: &[usize] = &[
    64,   // Small control packets
    256,  // Small packets
    512,  // Medium packets
    1024, // Common packet size
    1500, // Standard MTU
    2048, // Jumbo frames
    4096, // Large packets
    9000, // Max jumbo frame
];

// Maximum buffers per bucket to prevent unbounded growth
const MAX_BUFFERS_PER_BUCKET: usize = 32;

// Statistics for monitoring buffer pool performance
#[derive(Debug, Default, Clone)]
pub struct BufferPoolStats {
    pub allocations: usize,
    pub reuses: usize,
    pub total_bytes_allocated: usize,
    pub total_bytes_reused: usize,
    pub current_buffers_count: usize,
    pub peak_buffers_count: usize,
    pub misses: usize, // Times we couldn't find a suitable buffer
}

struct BufferBucket {
    size: usize,
    buffers: VecDeque<BytesMut>,
    stats: BufferPoolStats,
}

impl BufferBucket {
    fn new(size: usize) -> Self {
        Self {
            size,
            buffers: VecDeque::with_capacity(MAX_BUFFERS_PER_BUCKET),
            stats: BufferPoolStats::default(),
        }
    }

    fn acquire(&mut self) -> BytesMut {
        if let Some(mut buffer) = self.buffers.pop_front() {
            buffer.clear();
            self.stats.reuses += 1;
            self.stats.total_bytes_reused += self.size;
            self.stats.current_buffers_count = self.buffers.len();
            buffer
        } else {
            self.stats.allocations += 1;
            self.stats.total_bytes_allocated += self.size;
            self.stats.misses += 1;
            BytesMut::with_capacity(self.size)
        }
    }

    fn release(&mut self, buffer: BytesMut) {
        // Only return to pool if we haven't exceeded capacity
        if self.buffers.len() < MAX_BUFFERS_PER_BUCKET {
            self.buffers.push_back(buffer);
            self.stats.current_buffers_count = self.buffers.len();
            if self.stats.current_buffers_count > self.stats.peak_buffers_count {
                self.stats.peak_buffers_count = self.stats.current_buffers_count;
            }
        }
        // Otherwise let the buffer be dropped and freed
    }

    fn clear(&mut self) {
        self.buffers.clear();
        self.stats.current_buffers_count = 0;
    }
}

pub struct BufferPool {
    buckets: Arc<Mutex<Vec<BufferBucket>>>,
    total_memory_limit: AtomicUsize,
    current_memory_usage: AtomicUsize,
}

impl BufferPool {
    pub fn new(memory_limit: usize) -> Self {
        let buckets = BUCKET_SIZES
            .iter()
            .map(|&size| BufferBucket::new(size))
            .collect();

        Self {
            buckets: Arc::new(Mutex::new(buckets)),
            total_memory_limit: AtomicUsize::new(memory_limit),
            current_memory_usage: AtomicUsize::new(0),
        }
    }

    // Acquire a buffer of at least the requested size
    pub fn acquire(&self, size: usize) -> BytesMut {
        let mut buckets = self.buckets.lock().unwrap();

        // Find the smallest bucket that can fit the requested size
        for bucket in buckets.iter_mut() {
            if bucket.size >= size {
                let buffer = bucket.acquire();
                self.current_memory_usage
                    .fetch_add(bucket.size, Ordering::Relaxed);
                return buffer;
            }
        }

        // No suitable bucket found, allocate directly
        // This handles very large buffers that don't fit in our buckets
        BytesMut::with_capacity(size)
    }

    // Release a buffer back to the pool for reuse
    pub fn release(&self, buffer: BytesMut) {
        let capacity = buffer.capacity();

        // Don't pool very large buffers
        if capacity > *BUCKET_SIZES.last().unwrap() {
            return;
        }

        let mut buckets = self.buckets.lock().unwrap();

        // Find the appropriate bucket for this buffer size
        for bucket in buckets.iter_mut() {
            if bucket.size >= capacity {
                // Check memory limit before returning to pool
                let current_usage = self.current_memory_usage.load(Ordering::Relaxed);
                let limit = self.total_memory_limit.load(Ordering::Relaxed);

                if current_usage < limit {
                    bucket.release(buffer);
                }
                break;
            }
        }
    }

    // Get statistics for monitoring
    pub fn stats(&self) -> Vec<(usize, BufferPoolStats)> {
        let buckets = self.buckets.lock().unwrap();
        buckets.iter().map(|b| (b.size, b.stats.clone())).collect()
    }

    // Clear all pooled buffers
    pub fn clear(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        for bucket in buckets.iter_mut() {
            bucket.clear();
        }
        self.current_memory_usage.store(0, Ordering::Relaxed);
    }

    // Get total memory usage
    pub fn memory_usage(&self) -> usize {
        self.current_memory_usage.load(Ordering::Relaxed)
    }

    // Update memory limit
    pub fn set_memory_limit(&self, limit: usize) {
        self.total_memory_limit.store(limit, Ordering::Relaxed);
    }
}

// Thread-safe global buffer pool
static GLOBAL_BUFFER_POOL: LazyLock<BufferPool> = LazyLock::new(|| {
    // Default to 64MB memory limit for the pool
    BufferPool::new(64 * 1024 * 1024)
});

// Convenience functions for global pool access
pub fn acquire_buffer(size: usize) -> BytesMut {
    GLOBAL_BUFFER_POOL.acquire(size)
}

pub fn release_buffer(buffer: BytesMut) {
    GLOBAL_BUFFER_POOL.release(buffer)
}

pub fn buffer_pool_stats() -> Vec<(usize, BufferPoolStats)> {
    GLOBAL_BUFFER_POOL.stats()
}

pub fn clear_buffer_pool() {
    GLOBAL_BUFFER_POOL.clear()
}

// Zero-copy buffer wrapper for efficient data sharing
pub struct SharedBuffer {
    data: Bytes,
}

impl SharedBuffer {
    pub fn new(data: Bytes) -> Self {
        Self { data }
    }

    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(vec),
        }
    }

    pub fn from_bytes_mut(bytes: BytesMut) -> Self {
        Self {
            data: bytes.freeze(),
        }
    }

    // Get a slice without copying
    pub fn slice(&self, start: usize, end: usize) -> Bytes {
        self.data.slice(start..end)
    }

    // Get the entire buffer
    pub fn as_bytes(&self) -> &Bytes {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

use std::sync::LazyLock;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_buffer_pool_acquire_release() {
        let pool = BufferPool::new(1024 * 1024);

        // Acquire a buffer
        let buffer = pool.acquire(1000);
        assert!(buffer.capacity() >= 1000);

        // Release it back
        pool.release(buffer);

        // Acquire again should reuse
        let buffer2 = pool.acquire(1000);
        assert!(buffer2.capacity() >= 1000);

        // Check stats
        let stats = pool.stats();
        let bucket_stats = stats
            .iter()
            .find(|(size, _)| *size == 1024)
            .unwrap()
            .1
            .clone();
        assert_eq!(bucket_stats.reuses, 1);
    }

    #[test]
    fn test_shared_buffer_zero_copy() {
        let data = vec![1, 2, 3, 4, 5];
        let shared = SharedBuffer::from_vec(data);

        // Slice without copying
        let slice = shared.slice(1, 3);
        assert_eq!(&slice[..], &[2, 3]);

        // Original buffer still intact
        assert_eq!(shared.len(), 5);
    }

    #[test]
    fn test_global_pool() {
        let buffer1 = acquire_buffer(512);
        assert!(buffer1.capacity() >= 512);
        release_buffer(buffer1);

        let buffer2 = acquire_buffer(512);
        assert!(buffer2.capacity() >= 512);
        release_buffer(buffer2);

        // Should have reused
        let stats = buffer_pool_stats();
        let bucket_stats = stats
            .iter()
            .find(|(size, _)| *size == 512)
            .unwrap()
            .1
            .clone();
        assert!(bucket_stats.reuses > 0);
    }

    #[test]
    fn test_memory_usage_tests() {
        let pool = BufferPool::new(10 * 1024 * 1024); // 10MB limit

        // Acquire buffers of different sizes
        let buf1 = pool.acquire(512);
        assert!(buf1.capacity() >= 512);

        let buf2 = pool.acquire(1024);
        assert!(buf2.capacity() >= 1024);

        let buf3 = pool.acquire(1500);
        assert!(buf3.capacity() >= 1500);

        // Release them back
        pool.release(buf1);
        pool.release(buf2);
        pool.release(buf3);

        // Check memory usage
        assert!(pool.memory_usage() > 0);

        // Clear and verify
        pool.clear();
        assert_eq!(pool.memory_usage(), 0);
    }

    #[test]
    fn test_buffer_perf() {
        let pool = BufferPool::new(64 * 1024 * 1024);
        let iterations = 1000;

        // Benchmark allocation and release
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let buffer = pool.acquire(1500);
            // Simulate some work
            let mut _data = vec![0u8; 100];
            pool.release(buffer);
        }

        let duration = start.elapsed();
        let avg_time_us = duration.as_micros() / iterations;

        // Should be very fast (< 10 microseconds per operation)
        assert!(
            avg_time_us < 10,
            "Buffer pool operations too slow: {}us",
            avg_time_us
        );

        // Check reuse efficiency
        let stats = pool.stats();
        let mtu_stats = stats
            .iter()
            .find(|(size, _)| *size == 1500 || *size == 2048)
            .unwrap()
            .1
            .clone();
        assert!(mtu_stats.reuses > 0, "Buffer pool should reuse buffers");
    }

    #[test]
    fn test_stress_buffers() {
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(BufferPool::new(128 * 1024 * 1024));
        let num_threads = 10;
        let ops_per_thread = 100;

        let mut handles = vec![];

        for _ in 0..num_threads {
            let pool = pool.clone();
            let handle = thread::spawn(move || {
                for i in 0..ops_per_thread {
                    // Vary buffer sizes
                    let size = 100 + (i * 100) % 4000;
                    let buffer = pool.acquire(size);

                    // Simulate some work
                    thread::yield_now();

                    pool.release(buffer);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify pool is still functional
        let final_buffer = pool.acquire(1000);
        assert!(final_buffer.capacity() >= 1000);
    }
}
