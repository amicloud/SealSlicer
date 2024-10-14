[![Rust](https://github.com/amicloud/SealSlicer/actions/workflows/rust.yml/badge.svg)](https://github.com/amicloud/SealSlicer/actions/workflows/rust.yml)
# SealSlicer

SealSlicer is a **work-in-progress** slicer tailored specifically for MSLA resin 3D printing. Designed with both performance and usability in mind, SealSlicer leverages GPU acceleration to deliver fast and efficient slicing operations.

*This file last updated on October 14, 2024*

---

## 🛠️ Current Progress

### ✔️ Mesh Rendering Engine

- Fully implemented and operational.
- Provides accurate and efficient rendering of 3D meshes for slicing.

### 🖥️ CPU-only Slicer

- **Status:** Written but not yet tested.
- Implements basic slicing functionality using CPU resources.

### 🎛️ GPU Compute Slicer

- **Status:** Working but slightly broken.
- Utilizes GPU acceleration for enhanced slicing performance.
- Minor issues are being addressed and expected to be resolved soon.

### 🖥️ Highly Responsive UI

- **Status:** Implemented.
- The user interface is responsive; however, slicing currently blocks the UI thread. An easy fix is planned to ensure seamless user experience.

### 🔄 Multithreading

- **Status:** In progress.
- Actively working on leveraging multithreading to enhance performance wherever beneficial.

---

## 🚀 Upcoming Features

### 🛠️ Printer Settings Module

- **Status:** Not started.
- **Planned Functionality:**
  - Load and manage printer settings from `.json` files for easy configuration and customization.

### 📝 G-code/3D Printer File Generator

- **Status:** Not started.
- **Planned Functionality:**
  - Generate G-code files compatible with MSLA resin printers, enabling direct printing from SealSlicer.

### 🖼️ G-code/Slice Images Viewer

- **Status:** Not started.
- **Planned Functionality:**
  - Visualize G-code and slice images within the application for better inspection and verification before printing.

### 🖧 Network Printing

- **Status:** Not started.
- **Planned Functionality:**
  - Send files for printing directly to printers over the network.
  - I think this should be possible with the current Chitubox firmwares? 

---

## 📊 Test Coverage

Trying to ensure robust and reliable functionality through comprehensive testing. Currently focusing on components not involving OpenGL.

- **Total Coverage:** **22.29%** (263/1180 lines covered)

## 🌟 Goals

SealSlicer aims to provide a comprehensive and efficient slicing solution for MSLA resin printers with the following objectives:

- **Mesh Rendering Engine:** Accurate and efficient rendering of 3D meshes.
- **CPU-only Slicer:** Basic slicing functionality using CPU resources.
- **GPU Compute Slicer:** Enhanced slicing performance leveraging GPU acceleration.
- **Printer Settings Module:** Easy configuration through `.json` files.
- **G-code/3D Printer File Generator:** Direct generation of printer-compatible files.
- **G-code/Slice Images Viewer:** Visual inspection of slicing results.
- **High Responsiveness:** Ensuring the UI remains responsive during intensive operations.
- **Multithreading:** Utilizing multiple threads to optimize performance.

---

## 🤝 Contributing

SealSlicer is currently a personal project currently tailored for a specific printer model. Contributions are welcome once the core functionalities are stable and generalized. Stay tuned for updates!

If you're interested in contributing, please follow these guidelines:

1. **Fork the Repository:** Create your own fork of SealSlicer.
2. **Create a Feature Branch:** Branch off from `main` to work on your feature or bug fix.
3. **Commit Your Changes:** Ensure your commits are descriptive.
4. **Open a Pull Request:** Submit your changes for review. Please only submit properly formatted code. Use cargo fmt

Feel free to reach out with suggestions or if you're interested in collaborating!

---

## 📝 Notes

- **GPU Slicing:** The GPU slicing component is somewhat functional, with some bugs remaining.
- **Responsive UI:** While the UI is currently responsive, slicing operations block the UI thread. Fixes are planned to resolve this.
- **Test Coverage:** Focused on ensuring reliability for non-OpenGL components, with ongoing efforts to increase coverage.
- **Future Enhancements:** Plans include expanding compatibility, enhancing multithreading, and adding user-friendly features like settings management and file visualization.


---

## 📝 License

This project is licensed under the **GNU Affero General Public License v3.0**.

You are free to run, study, modify, and share this software under the terms of the AGPL-3.0, with the added requirement that if the software is used to interact with users over a network, the source code must be made available to those users.

### Key License Points:
- You may **use**, **modify**, and **share** this software.
- If you make modifications and provide it over a network
