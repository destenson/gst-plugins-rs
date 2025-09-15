// GStreamer RTSP Source v2
//
// Copyright (C) 2023 Tim-Philipp Müller <tim centricular com>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

/**
 * SECTION:element-rtspsrc2
 *
 * `rtspsrc2` is a from-scratch rewrite of the `rtspsrc` element to fix some fundamental
 * architectural issues, with the aim of making the two functionally equivalent.
 *
 * Implemented features:
 * * RTSP 1.0 support
 * * Lower transports: TCP, UDP, UDP-Multicast
 * * RTCP SR and RTCP RR
 * * RTCP-based A/V sync
 * * Lower transport selection and priority (NEW!)
 *   - Also supports different lower transports for each SETUP
 *
 * Some missing features:
 * * SET_PARAMETER/GET_PARAMETER messages
 * * SRTP support
 * * VOD support: PAUSE, seeking, etc
 * * ONVIF backchannel and trick mode support
 * * and more
 *
 * Please see the [README](https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs/-/blob/main/net/rtsp/README.md)
 * for a complete and up-to-date list.
 */
use gst::glib;
use gst::prelude::*;

#[cfg(feature = "adaptive")]
mod adaptive_retry;
mod auth;
mod auto_selector;
mod body;
mod buffer_mode;
mod buffer_pool;
mod connection_pool;
mod connection_racer;
mod debug;
#[cfg(test)]
mod debug_tests;
pub mod error;
mod error_migration_example;
pub mod error_recovery;
#[cfg(test)]
mod error_tests;
mod http_tunnel;
#[cfg(test)]
mod http_tunnel_tests;
mod imp;
mod proxy;
#[cfg(test)]
mod racing_strategy_tests;
pub mod retry;
#[cfg(test)]
mod retry_integration_tests;
mod rtcp_enhanced;
mod sdp;
mod session_manager;
pub mod srtp;
mod tcp_message;
#[cfg(feature = "telemetry")]
mod telemetry;
mod tls;
mod transport;
mod version_detection;

glib::wrapper! {
    pub struct RtspSrc(ObjectSubclass<imp::RtspSrc>) @extends gst::Bin, gst::Element, gst::Object, @implements gst::URIHandler;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "rtspsrc2",
        gst::Rank::PRIMARY,
        RtspSrc::static_type(),
    )
}
