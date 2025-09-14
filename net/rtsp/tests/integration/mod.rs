// GStreamer RTSP integration test suite
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

pub mod server_helper;
pub mod test_harness;
pub mod scenarios;

#[cfg(feature = "integration-tests")]
pub use server_helper::MediaMtxServer;
#[cfg(feature = "integration-tests")]
pub use test_harness::RtspTestHarness;