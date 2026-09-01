# Computer Diagnostics (Terminal Edition)

A lightweight, portable CLI hardware analysis and system profiling tool built in Rust. It inspects internal components, network metrics, mounted and physical storage devices, graphics pipelines, and active workloads through a structured, interactive terminal interface.

---

## Overview

Computer Diagnostics provides system information without background telemetry, heavy web-view frameworks, or installers. It compiles to a self-contained, single-file binary designed to run across Windows environments (Windows 10, Windows 11, with legacy support in development).

## Core Capabilities

* **Compute & Memory:** Per-core CPU utilization with dynamic ASCII metering, base clock frequencies, physical RAM statistics, and page file load.
* **Motherboard & Platform:** Board manufacturer, product model, serial number, OS build version, and system uptime counters.
* **Storage Systems:** Mounted filesystem volumes, physical bus devices (NVMe, SATA HDD/SSD), and optical media states via WMI.
* **Graphics Subsystem:** Multi-GPU controller detection, display driver versioning, and dedicated/shared memory tracking.
* **Networking:** Adapter detection, MAC address retrieval, and session I/O byte counters.
* **Process Monitor:** Top resource-consuming tasks sorted by CPU utilization with crash-safe float handling.
* **Diagnostics Context:** Integrated debug probe displaying system latency, process counts, architecture details, and COM/WMI health.

---

## Getting Started

### Pre-Built Binaries
Download the latest binary from the Releases tab and run it directly in your terminal or via double-click.

### Build from Source
Requirements: Rust (cargo, rustc 1.75+)
